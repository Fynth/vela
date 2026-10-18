//! Native output and local decoding. No HTTP, credentials or listening history.
use std::{
    fs::File,
    io::BufReader,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread,
    time::{Duration, Instant},
};

use rodio::{
    ChannelCount, Decoder, DeviceSinkBuilder, Player, SampleRate, Source,
    cpal::traits::{DeviceTrait, HostTrait},
};

use crate::{AudioPhase, AudioSnapshot, PreparedAudio};

const QUEUE_SIZE: usize = 32;
const TICK: Duration = Duration::from_millis(20);
const MAX_BYTES: u64 = 250 * 1024 * 1024;
const OUTPUT_ERROR: &str =
    "Native audio output is unavailable; check the output device and restart playback";
const DECODE_ERROR: &str = "This recording could not be decoded by the native player";
const WORKER_ERROR: &str = "The native audio worker stopped unexpectedly; restart the player";

#[derive(Clone)]
pub struct AudioEngine {
    inner: Arc<EngineInner>,
}

struct EngineInner {
    sender: SyncSender<Command>,
    shared: Arc<Shared>,
}

struct Shared {
    submission: Mutex<Submission>,
    snapshot: Mutex<AudioSnapshot>,
    gate: Arc<SourceGate>,
    worker_alive: AtomicBool,
    device_failed: AtomicBool,
}

#[derive(Default)]
struct Submission {
    generation: u64,
    source: u64,
    stopped: bool,
}

struct SourceGate {
    alive: AtomicBool,
    source: AtomicU64,
}

enum Command {
    Load {
        generation: u64,
        source: u64,
        audio: Arc<PreparedAudio>,
        autoplay: bool,
        position: u64,
    },
    Pause {
        generation: u64,
        source: u64,
    },
    Resume {
        generation: u64,
        source: u64,
    },
    Seek {
        generation: u64,
        source: u64,
        position: u64,
    },
    Volume {
        generation: u64,
        source: u64,
        volume: f32,
    },
    Stop {
        generation: u64,
        source: u64,
    },
}

impl Command {
    fn identity(&self) -> (u64, u64) {
        match self {
            Self::Load {
                generation, source, ..
            }
            | Self::Pause { generation, source }
            | Self::Resume { generation, source }
            | Self::Seek {
                generation, source, ..
            }
            | Self::Volume {
                generation, source, ..
            }
            | Self::Stop { generation, source } => (*generation, *source),
        }
    }
}

impl AudioEngine {
    pub fn new() -> Result<Self, String> {
        Self::start(None)
    }

    /// Select exactly this device. There is no fallback to a user's default
    /// output, which is essential for opt-in isolated null-sink validation.
    pub fn with_device_name(name: &str) -> Result<Self, String> {
        if name.is_empty() || name.len() > 512 || name.chars().any(char::is_control) {
            return Err("Invalid audio output device name".into());
        }
        Self::start(Some(name.to_owned()))
    }

    fn start(device: Option<String>) -> Result<Self, String> {
        let shared = Arc::new(Shared {
            submission: Mutex::new(Submission::default()),
            snapshot: Mutex::new(AudioSnapshot::default()),
            gate: Arc::new(SourceGate {
                alive: AtomicBool::new(true),
                source: AtomicU64::new(0),
            }),
            worker_alive: AtomicBool::new(true),
            device_failed: AtomicBool::new(false),
        });
        let (sender, receiver) = mpsc::sync_channel(QUEUE_SIZE);
        let (ready_sender, ready) = mpsc::sync_channel(1);
        let worker_shared = shared.clone();
        thread::Builder::new()
            .name("vela-native-audio".into())
            .spawn(move || {
                let result = catch_unwind(AssertUnwindSafe(|| {
                    run(receiver, &worker_shared, device, &ready_sender)
                }));
                if result.is_err() {
                    worker_shared.fail(WORKER_ERROR);
                    let _ = ready_sender.try_send(Err(WORKER_ERROR.to_owned()));
                }
                worker_shared.worker_alive.store(false, Ordering::Release);
            })
            .map_err(|_| WORKER_ERROR.to_owned())?;
        match ready.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => Ok(Self {
                inner: Arc::new(EngineInner { sender, shared }),
            }),
            Ok(Err(error)) => {
                shared.gate.alive.store(false, Ordering::Release);
                Err(error)
            }
            Err(_) => {
                shared.gate.alive.store(false, Ordering::Release);
                Err(OUTPUT_ERROR.into())
            }
        }
    }

    pub fn snapshot(&self) -> AudioSnapshot {
        self.inner
            .shared
            .snapshot
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }

    /// The output worker died after a device/stream failure. Controls must stop
    /// claiming retryability until the UI installs a new engine.
    pub fn alive(&self) -> bool {
        self.inner.shared.worker_alive.load(Ordering::Acquire)
    }

    pub fn load(
        &self,
        generation: u64,
        audio: Arc<PreparedAudio>,
        autoplay: bool,
        position_ms: u64,
    ) -> Result<(), String> {
        if audio.length_bytes == 0 || audio.length_bytes > MAX_BYTES {
            return Err(DECODE_ERROR.into());
        }
        self.replace(
            generation,
            move |source| Command::Load {
                generation,
                source,
                audio,
                autoplay,
                position: position_ms,
            },
            false,
        )
    }

    pub fn pause(&self, generation: u64) -> Result<(), String> {
        self.control(generation, |source| Command::Pause { generation, source })
    }

    pub fn resume(&self, generation: u64) -> Result<(), String> {
        self.control(generation, |source| Command::Resume { generation, source })
    }

    pub fn seek(&self, generation: u64, position_ms: u64) -> Result<(), String> {
        let snapshot = self.snapshot();
        if snapshot.generation == generation
            && snapshot.duration_ms > 0
            && position_ms > snapshot.duration_ms
        {
            return Err("Playback position is outside this recording".into());
        }
        self.control(generation, |source| Command::Seek {
            generation,
            source,
            position: position_ms,
        })
    }

    pub fn set_volume(&self, generation: u64, volume: f32) -> Result<(), String> {
        if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
            return Err("Volume must be between zero and one".into());
        }
        self.control(generation, |source| Command::Volume {
            generation,
            source,
            volume,
        })
    }

    pub fn stop(&self, generation: u64) -> Result<(), String> {
        self.replace(
            generation,
            |source| Command::Stop { generation, source },
            true,
        )
    }

    fn control(&self, generation: u64, command: impl FnOnce(u64) -> Command) -> Result<(), String> {
        let shared = &self.inner.shared;
        if !shared.worker_alive.load(Ordering::Acquire) {
            return Err(WORKER_ERROR.into());
        }
        let state = lock_control(shared)?;
        if generation != state.generation {
            return Ok(());
        }
        self.inner
            .sender
            .try_send(command(state.source))
            .map_err(send_error)
    }

    fn replace(
        &self,
        generation: u64,
        command: impl FnOnce(u64) -> Command,
        stop: bool,
    ) -> Result<(), String> {
        let shared = &self.inner.shared;
        if !shared.worker_alive.load(Ordering::Acquire) {
            return Err(WORKER_ERROR.into());
        }
        // Stop is also the account-boundary barrier. Its tiny critical section
        // contains no decoding, device calls, file cleanup or blocking sends,
        // so transient publication contention must not turn it into Busy.
        let mut state = if stop {
            shared
                .submission
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
        } else {
            lock_control(shared)?
        };
        if generation < state.generation {
            return Ok(());
        }
        let source = state
            .source
            .checked_add(1)
            .ok_or_else(|| WORKER_ERROR.to_owned())?;
        match self.inner.sender.try_send(command(source)) {
            Ok(()) => {}
            // Stop also invalidates the source out of band, so a saturated
            // ordinary command queue cannot keep old-account audio playing.
            Err(TrySendError::Full(_)) if stop => {}
            Err(error) => {
                // A rejected Load may own the last temporary-file lease. Drop
                // it only after leaving the account-boundary critical section.
                drop(state);
                return Err(send_error(error));
            }
        }
        state.generation = generation;
        state.source = source;
        state.stopped = stop;
        shared.gate.source.store(source, Ordering::Release);
        Ok(())
    }
}

// The worker takes this lock for a few instructions on every tick. A single
// failed try_lock is not real saturation, and volume/load right after engine
// startup must not surface a spurious Busy error. Retry for a bounded moment,
// then fail honestly if something actually retains the lock.
fn lock_control(shared: &Shared) -> Result<std::sync::MutexGuard<'_, Submission>, String> {
    for attempt in 0..64_u32 {
        match shared.submission.try_lock() {
            Ok(guard) => return Ok(guard),
            Err(std::sync::TryLockError::Poisoned(poison)) => return Ok(poison.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) if attempt < 32 => std::thread::yield_now(),
            Err(std::sync::TryLockError::WouldBlock) => {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
    Err("Audio controls are busy; try again".to_owned())
}

fn send_error(error: TrySendError<Command>) -> String {
    match error {
        TrySendError::Full(_) => "Audio controls are busy; try again".into(),
        TrySendError::Disconnected(_) => WORKER_ERROR.into(),
    }
}

impl Drop for EngineInner {
    fn drop(&mut self) {
        // The callback stops consuming immediately; the worker drops decoder,
        // private lease, queued leases and device without joining on the UI.
        self.shared.gate.alive.store(false, Ordering::Release);
    }
}

impl Shared {
    fn current(&self, generation: u64, source: u64) -> bool {
        // Submission holds this lock across try_send and publication of the
        // source token, preventing a receiver racing a newly enqueued load.
        let state = self
            .submission
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        self.gate.alive.load(Ordering::Acquire)
            && state.generation == generation
            && state.source == source
    }

    fn publish(&self, snapshot: AudioSnapshot, source: u64) {
        let state = self
            .submission
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if self.gate.alive.load(Ordering::Acquire)
            && state.generation == snapshot.generation
            && state.source == source
        {
            *self
                .snapshot
                .lock()
                .unwrap_or_else(|poison| poison.into_inner()) = snapshot;
        }
    }

    fn fail(&self, error: &str) {
        let state = self
            .submission
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let mut snapshot = self
            .snapshot
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        snapshot.generation = state.generation;
        snapshot.phase = AudioPhase::Error;
        snapshot.error = Some(error.into());
    }
}

fn run(
    receiver: Receiver<Command>,
    shared: &Arc<Shared>,
    device: Option<String>,
    ready: &SyncSender<Result<(), String>>,
) {
    let builder = if let Some(name) = device {
        rodio::cpal::default_host()
            .output_devices()
            .ok()
            .and_then(|mut devices| {
                devices.find(|device| {
                    device
                        .description()
                        .is_ok_and(|description| description.name() == name)
                })
            })
            .ok_or_else(|| OUTPUT_ERROR.to_owned())
            .and_then(|device| {
                DeviceSinkBuilder::from_device(device).map_err(|_| OUTPUT_ERROR.to_owned())
            })
    } else {
        DeviceSinkBuilder::from_default_device().map_err(|_| OUTPUT_ERROR.to_owned())
    };
    let callback_shared = shared.clone();
    let output = builder.and_then(|builder| {
        builder
            .with_error_callback(move |_| {
                callback_shared.device_failed.store(true, Ordering::Release);
            })
            .open_sink_or_fallback()
            .map_err(|_| OUTPUT_ERROR.to_owned())
    });
    let mut output = match output {
        Ok(output) => output,
        Err(error) => {
            shared.fail(&error);
            let _ = ready.try_send(Err(error));
            return;
        }
    };
    output.log_on_drop(false);
    if ready.try_send(Ok(())).is_err() {
        return;
    }
    let mut current: Option<Current> = None;
    let mut volume = 0.7;
    while shared.gate.alive.load(Ordering::Acquire) {
        if shared.device_failed.load(Ordering::Acquire) {
            shared.fail(OUTPUT_ERROR);
            break;
        }
        // A stop remains observable even if its best-effort wakeup could not
        // enter a full queue while no decoder was loaded.
        let stopped = {
            let state = shared
                .submission
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            state.stopped.then_some((state.generation, state.source))
        };
        if let Some((generation, source)) = stopped {
            current = None;
            shared.publish(
                AudioSnapshot {
                    generation,
                    volume,
                    ..AudioSnapshot::default()
                },
                source,
            );
        }
        if current
            .as_ref()
            .is_some_and(|current| !shared.current(current.generation, current.source))
        {
            current = None;
            let state = shared
                .submission
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            let generation = state.generation;
            let source = state.source;
            drop(state);
            shared.publish(
                AudioSnapshot {
                    generation,
                    volume,
                    ..AudioSnapshot::default()
                },
                source,
            );
        }
        match receiver.recv_timeout(TICK) {
            Ok(command) => {
                let (generation, source) = command.identity();
                if !shared.current(generation, source) {
                    continue;
                }
                match command {
                    Command::Load {
                        audio,
                        autoplay,
                        position,
                        ..
                    } => {
                        current = None;
                        shared.publish(
                            AudioSnapshot {
                                generation,
                                phase: AudioPhase::Loading,
                                volume,
                                ..AudioSnapshot::default()
                            },
                            source,
                        );
                        match Current::load(
                            &output, shared, generation, source, audio, autoplay, position, volume,
                        ) {
                            Ok(loaded) => {
                                if shared.current(generation, source) {
                                    current = Some(loaded);
                                }
                            }
                            Err(error) => shared.publish(
                                AudioSnapshot {
                                    generation,
                                    phase: AudioPhase::Error,
                                    volume,
                                    error: Some(error),
                                    ..AudioSnapshot::default()
                                },
                                source,
                            ),
                        }
                    }
                    Command::Stop { .. } => {
                        current = None;
                        shared.publish(
                            AudioSnapshot {
                                generation,
                                volume,
                                ..AudioSnapshot::default()
                            },
                            source,
                        );
                    }
                    Command::Volume { volume: value, .. } => {
                        volume = value;
                        if let Some(current) = &current {
                            current.player.set_volume(volume);
                        }
                        let mut snapshot = shared
                            .snapshot
                            .lock()
                            .unwrap_or_else(|poison| poison.into_inner())
                            .clone();
                        snapshot.generation = generation;
                        snapshot.volume = volume;
                        shared.publish(snapshot, source);
                    }
                    Command::Pause { .. } => {
                        if let Some(current) = &mut current {
                            current.player.pause();
                            current.playing = false;
                        }
                    }
                    Command::Resume { .. } => {
                        if let Some(mut old) = current.take() {
                            if old.watch.ended.load(Ordering::Acquire) {
                                let audio = old.audio.clone();
                                drop(old);
                                match Current::load(
                                    &output, shared, generation, source, audio, true, 0, volume,
                                ) {
                                    Ok(loaded) => current = Some(loaded),
                                    Err(error) => shared.publish(
                                        AudioSnapshot {
                                            generation,
                                            phase: AudioPhase::Error,
                                            volume,
                                            error: Some(error),
                                            ..AudioSnapshot::default()
                                        },
                                        source,
                                    ),
                                }
                            } else {
                                old.player.play();
                                old.playing = true;
                                old.last_progress = Instant::now();
                                current = Some(old);
                            }
                        }
                    }
                    Command::Seek { position, .. } => {
                        if let Some(old) = current.take() {
                            let playing = old.playing && !old.watch.ended.load(Ordering::Acquire);
                            let audio = old.audio.clone();
                            drop(old);
                            match Current::load(
                                &output, shared, generation, source, audio, playing, position,
                                volume,
                            ) {
                                Ok(loaded) => current = Some(loaded),
                                Err(error) => shared.publish(
                                    AudioSnapshot {
                                        generation,
                                        phase: AudioPhase::Error,
                                        volume,
                                        error: Some(error),
                                        ..AudioSnapshot::default()
                                    },
                                    source,
                                ),
                            }
                        }
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        if let Some(current) = &mut current {
            current.update(shared, volume);
        }
    }
    drop(current);
    drop(output);
}

#[derive(Default)]
struct Watch {
    canceled: AtomicBool,
    ended: AtomicBool,
    failed: AtomicBool,
    samples: AtomicU64,
}

struct GuardedSource<S> {
    decoder: S,
    first: Option<f32>,
    channels: ChannelCount,
    rate: SampleRate,
    duration: Option<Duration>,
    _lease: Arc<PreparedAudio>,
    gate: Arc<SourceGate>,
    source: u64,
    watch: Arc<Watch>,
}

impl<S: Source> Iterator for GuardedSource<S> {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if !self.gate.alive.load(Ordering::Acquire)
            || self.gate.source.load(Ordering::Acquire) != self.source
            || self.watch.canceled.load(Ordering::Acquire)
        {
            return None;
        }
        match catch_unwind(AssertUnwindSafe(|| {
            self.first.take().or_else(|| self.decoder.next())
        })) {
            Ok(Some(sample)) if sample.is_finite() => {
                self.watch.samples.fetch_add(1, Ordering::Relaxed);
                Some(sample)
            }
            Ok(None) => {
                self.watch.ended.store(true, Ordering::Release);
                None
            }
            _ => {
                self.watch.failed.store(true, Ordering::Release);
                None
            }
        }
    }
}

impl<S: Source> Source for GuardedSource<S> {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> ChannelCount {
        self.channels
    }
    fn sample_rate(&self) -> SampleRate {
        self.rate
    }
    fn total_duration(&self) -> Option<Duration> {
        self.duration
    }
}

type LocalDecoder = Decoder<BufReader<File>>;

fn decode(audio: &PreparedAudio, position: u64) -> Result<(LocalDecoder, Option<f32>), String> {
    let file = File::open(audio.path()).map_err(|_| DECODE_ERROR.to_owned())?;
    if file.metadata().map_err(|_| DECODE_ERROR.to_owned())?.len() != audio.length_bytes {
        return Err(DECODE_ERROR.into());
    }
    let mut decoder = Decoder::try_from(file).map_err(|_| DECODE_ERROR.to_owned())?;
    let first = decoder
        .next()
        .filter(|sample| sample.is_finite())
        .ok_or_else(|| DECODE_ERROR.to_owned())?;
    if position > 0 {
        let duration = decoder
            .total_duration()
            .ok_or_else(|| "Seeking is unavailable for this recording".to_owned())?;
        if Duration::from_millis(position) > duration {
            return Err("Playback position is outside this recording".into());
        }
        decoder
            .try_seek(Duration::from_millis(position))
            .map_err(|_| "This recording does not support native seeking".to_owned())?;
        Ok((decoder, None))
    } else {
        Ok((decoder, Some(first)))
    }
}

struct Current {
    player: Player,
    audio: Arc<PreparedAudio>,
    watch: Arc<Watch>,
    generation: u64,
    source: u64,
    base: u64,
    duration: u64,
    channels: u64,
    rate: u64,
    playing: bool,
    last_position: u64,
    last_progress: Instant,
}

impl Current {
    #[allow(clippy::too_many_arguments)]
    fn load(
        output: &rodio::MixerDeviceSink,
        shared: &Shared,
        generation: u64,
        source: u64,
        audio: Arc<PreparedAudio>,
        playing: bool,
        position: u64,
        volume: f32,
    ) -> Result<Self, String> {
        let (decoder, first) = decode(&audio, position)?;
        let channels = decoder.channels();
        let rate = decoder.sample_rate();
        let duration = decoder.total_duration();
        let duration_ms = duration.map(duration_millis).unwrap_or(0);
        let watch = Arc::new(Watch::default());
        let player = Player::connect_new(output.mixer());
        player.pause();
        player.set_volume(volume);
        player.append(GuardedSource {
            decoder,
            first,
            channels,
            rate,
            duration,
            _lease: audio.clone(),
            gate: shared.gate.clone(),
            source,
            watch: watch.clone(),
        });
        if playing && shared.current(generation, source) {
            player.play();
        }
        Ok(Self {
            player,
            audio,
            watch,
            generation,
            source,
            base: position,
            duration: duration_ms,
            channels: u64::from(channels.get()),
            rate: u64::from(rate.get()),
            playing,
            last_position: position,
            last_progress: Instant::now(),
        })
    }

    fn update(&mut self, shared: &Shared, volume: f32) {
        let mut position = self
            .base
            .saturating_add(duration_millis(self.player.get_pos()));
        if self.duration > 0 {
            position = position.min(self.duration);
        }
        let failed = self.watch.failed.load(Ordering::Acquire);
        let ended = self.watch.ended.load(Ordering::Acquire) && self.player.empty();
        if position != self.last_position {
            self.last_progress = Instant::now();
            self.last_position = position;
        }
        let stalled =
            self.playing && !ended && self.last_progress.elapsed() > Duration::from_secs(5);
        let phase = if failed || stalled {
            self.player.pause();
            AudioPhase::Error
        } else if ended {
            AudioPhase::Ended
        } else if self.playing {
            AudioPhase::Playing
        } else {
            AudioPhase::Paused
        };
        if ended {
            let decoded_ms = self
                .watch
                .samples
                .load(Ordering::Relaxed)
                .saturating_mul(1000)
                / self.channels
                / self.rate;
            position = self.base.saturating_add(decoded_ms);
        }
        shared.publish(
            AudioSnapshot {
                generation: self.generation,
                phase,
                position_ms: position,
                duration_ms: if ended && self.duration == 0 {
                    position
                } else {
                    self.duration
                },
                volume,
                error: if failed {
                    Some(DECODE_ERROR.into())
                } else if stalled {
                    Some(OUTPUT_ERROR.into())
                } else {
                    None
                },
            },
            self.source,
        );
    }
}

impl Drop for Current {
    fn drop(&mut self) {
        self.watch.canceled.store(true, Ordering::Release);
        self.player.stop();
    }
}

fn duration_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, sync::Weak};

    const FLAC: &[u8] = include_bytes!("../../../crates/media/tests/fixtures/signal.flac");
    const MP3: &[u8] = include_bytes!("../../../crates/media/tests/fixtures/beacon.mp3");

    fn prepared(bytes: &[u8]) -> Arc<PreparedAudio> {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(bytes).unwrap();
        file.flush().unwrap();
        Arc::new(PreparedAudio::new(file, bytes.len() as u64))
    }

    fn gate() -> Arc<SourceGate> {
        Arc::new(SourceGate {
            alive: AtomicBool::new(true),
            source: AtomicU64::new(1),
        })
    }

    fn source(bytes: &[u8]) -> (GuardedSource<LocalDecoder>, Weak<PreparedAudio>) {
        let audio = prepared(bytes);
        let lease = Arc::downgrade(&audio);
        let (decoder, first) = decode(&audio, 0).unwrap();
        let channels = decoder.channels();
        let rate = decoder.sample_rate();
        let duration = decoder.total_duration();
        (
            GuardedSource {
                decoder,
                first,
                channels,
                rate,
                duration,
                _lease: audio,
                gate: gate(),
                source: 1,
                watch: Arc::new(Watch::default()),
            },
            lease,
        )
    }

    fn control_fixture() -> (AudioEngine, Receiver<Command>) {
        let (sender, receiver) = mpsc::sync_channel(QUEUE_SIZE);
        let shared = Arc::new(Shared {
            submission: Mutex::new(Submission::default()),
            snapshot: Mutex::new(AudioSnapshot::default()),
            gate: gate(),
            worker_alive: AtomicBool::new(true),
            device_failed: AtomicBool::new(false),
        });
        shared.gate.source.store(0, Ordering::Release);
        (
            AudioEngine {
                inner: Arc::new(EngineInner { sender, shared }),
            },
            receiver,
        )
    }

    #[test]
    fn actual_flac_and_mp3_decoding_yields_finite_signal_and_releases_private_file() {
        for bytes in [FLAC, MP3] {
            let (mut source, lease) = source(bytes);
            assert!(source.channels().get() > 0);
            assert!(source.sample_rate().get() > 0);
            assert!(
                source
                    .total_duration()
                    .is_some_and(|duration| duration > Duration::ZERO)
            );
            let mut count = 0;
            let mut energy = 0.0_f64;
            for sample in source.by_ref().take(10_000_000) {
                assert!(sample.is_finite());
                count += 1;
                energy += f64::from(sample).powi(2);
            }
            assert!(count > 1000);
            assert!(energy > 0.001);
            assert!(source.watch.ended.load(Ordering::Acquire));
            assert!(!source.watch.failed.load(Ordering::Acquire));
            assert!(source._lease.path().exists());
            drop(source);
            assert!(lease.upgrade().is_none());
        }
    }

    #[test]
    fn decoder_seek_uses_real_duration_and_rejects_invalid_positions_and_data_safely() {
        for bytes in [FLAC, MP3] {
            let audio = prepared(bytes);
            let (decoder, _) = decode(&audio, 0).unwrap();
            let duration = duration_millis(decoder.total_duration().unwrap());
            let full_count = decoder.count();
            let (remaining, first) = decode(&audio, duration / 2).unwrap();
            assert!(first.is_none());
            let remaining_count = remaining.count();
            assert!(remaining_count > 0 && remaining_count < full_count);
            assert!(decode(&audio, u64::MAX).is_err());
        }
        let invalid = prepared(b"private-password https://private.example.org/credential");
        let error = decode(&invalid, 0).err().unwrap();
        assert_eq!(error, DECODE_ERROR);
        assert!(!error.contains("private-password"));
        assert!(!error.contains(&invalid.path().display().to_string()));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_unlinked_original_readers_have_independent_positions_and_last_arc_closes_owner_fd() {
        use std::{
            io::{Read, Seek, SeekFrom},
            os::unix::fs::MetadataExt,
        };

        let mut temporary = tempfile::NamedTempFile::new().unwrap();
        temporary.write_all(b"abcdefgh").unwrap();
        temporary.flush().unwrap();
        let named_path = temporary.path().to_owned();
        let metadata = temporary.as_file().metadata().unwrap();
        let identity = (metadata.dev(), metadata.ino());
        let audio = Arc::new(PreparedAudio::new(temporary, 8));
        let owner_path = audio.path().to_owned();
        let same_original = |path: &std::path::Path| {
            std::fs::metadata(path)
                .is_ok_and(|metadata| (metadata.dev(), metadata.ino()) == identity)
        };
        assert!(!same_original(&named_path));
        assert!(same_original(&owner_path));
        let mut first = File::open(audio.path()).unwrap();
        let mut second = File::open(audio.path()).unwrap();
        let mut bytes = [0; 2];
        first.read_exact(&mut bytes).unwrap();
        assert_eq!(&bytes, b"ab");
        second.read_exact(&mut bytes).unwrap();
        assert_eq!(&bytes, b"ab");
        first.read_exact(&mut bytes).unwrap();
        assert_eq!(&bytes, b"cd");
        second.seek(SeekFrom::Start(1)).unwrap();
        assert_eq!(first.stream_position().unwrap(), 4);
        assert_eq!(second.stream_position().unwrap(), 1);
        let weak = Arc::downgrade(&audio);
        let lease = audio.clone();
        drop(audio);
        assert!(same_original(&owner_path));
        drop(lease);
        assert!(weak.upgrade().is_none());
        // Independent readers pin the old inode here. A different file opened
        // concurrently on the reused owner-fd number cannot produce a false
        // failure, and the unlinked inode cannot yet be recycled.
        assert!(!same_original(&owner_path));
        first.read_exact(&mut bytes).unwrap();
        assert_eq!(&bytes, b"ef");
        second.read_exact(&mut bytes).unwrap();
        assert_eq!(&bytes, b"bc");
    }

    #[test]
    fn source_generation_and_lifetime_cancellation_are_distinct_from_natural_eof() {
        let (mut source, lease) = source(FLAC);
        assert!(source.next().is_some());
        source.gate.source.store(2, Ordering::Release);
        assert!(source.next().is_none());
        assert!(!source.watch.ended.load(Ordering::Acquire));
        drop(source);
        assert!(lease.upgrade().is_none());
        let (mut source, _) = self::source(MP3);
        source.gate.alive.store(false, Ordering::Release);
        assert!(source.next().is_none());
        assert!(!source.watch.ended.load(Ordering::Acquire));
    }

    struct PanickingDecoder;
    impl Iterator for PanickingDecoder {
        type Item = f32;
        fn next(&mut self) -> Option<f32> {
            panic!("simulated decoder failure")
        }
    }
    impl Source for PanickingDecoder {
        fn current_span_len(&self) -> Option<usize> {
            None
        }
        fn channels(&self) -> ChannelCount {
            ChannelCount::new(1).unwrap()
        }
        fn sample_rate(&self) -> SampleRate {
            SampleRate::new(44100).unwrap()
        }
        fn total_duration(&self) -> Option<Duration> {
            None
        }
    }

    #[test]
    fn callback_decoder_panic_becomes_explicit_failure_without_unwinding_device_thread() {
        let mut source = GuardedSource {
            decoder: PanickingDecoder,
            first: None,
            channels: ChannelCount::new(1).unwrap(),
            rate: SampleRate::new(44100).unwrap(),
            duration: None,
            _lease: prepared(FLAC),
            gate: gate(),
            source: 1,
            watch: Arc::new(Watch::default()),
        };
        assert!(source.next().is_none());
        assert!(source.watch.failed.load(Ordering::Acquire));
        assert!(!source.watch.ended.load(Ordering::Acquire));
    }

    #[test]
    fn commands_are_bounded_stale_controls_are_ignored_and_stop_invalidates_a_full_queue() {
        let (engine, receiver) = control_fixture();
        let audio = prepared(FLAC);
        for generation in 1..=QUEUE_SIZE as u64 {
            engine.load(generation, audio.clone(), false, 0).unwrap();
        }
        assert!(engine.load(33, audio.clone(), false, 0).is_err());
        engine.pause(0).unwrap();
        engine.seek(0, 0).unwrap();
        engine.stop(33).unwrap();
        assert_eq!(engine.inner.shared.gate.source.load(Ordering::Acquire), 33);
        for command in receiver.try_iter() {
            let (generation, source) = command.identity();
            assert!(!engine.inner.shared.current(generation, source));
        }
        for volume in [f32::NAN, f32::INFINITY, -1.0, 1.01] {
            assert!(engine.set_volume(33, volume).is_err());
        }
        assert!(receiver.try_recv().is_err());
        let shared = engine.inner.shared.clone();
        let clone = engine.clone();
        drop(engine);
        assert!(shared.gate.alive.load(Ordering::Acquire));
        drop(clone);
        assert!(!shared.gate.alive.load(Ordering::Acquire));
    }

    #[test]
    fn stop_waits_out_submission_contention_and_invalidates_the_old_account_decoder() {
        let (engine, receiver) = control_fixture();
        engine.load(4, prepared(FLAC), false, 0).unwrap();
        let (mut decoder, _) = source(FLAC);
        decoder.gate = engine.inner.shared.gate.clone();
        assert!(decoder.next().is_some());
        let shared = engine.inner.shared.clone();
        let (locked, held) = mpsc::sync_channel(1);
        let (release, released) = mpsc::sync_channel(1);
        let holder = thread::spawn(move || {
            let _guard = shared.submission.lock().unwrap();
            locked.send(()).unwrap();
            // Bounded safeguard prevents a failing regression from retaining
            // the holder thread or the private-file leases indefinitely.
            released.recv_timeout(Duration::from_secs(2)).unwrap();
        });
        held.recv_timeout(Duration::from_secs(2)).unwrap();
        let stopping = engine.clone();
        let (finished, completion) = mpsc::sync_channel(1);
        let stop_thread = thread::spawn(move || {
            finished.send(stopping.stop(5)).unwrap();
        });
        let early = completion.recv_timeout(Duration::from_millis(50));
        release.send(()).unwrap();
        holder.join().unwrap();
        let waited = matches!(early, Err(mpsc::RecvTimeoutError::Timeout));
        let result = match early {
            Ok(result) => result,
            Err(_) => completion.recv_timeout(Duration::from_secs(2)).unwrap(),
        };
        stop_thread.join().unwrap();
        assert!(
            waited,
            "Stop must wait for submission publication rather than return Busy"
        );
        result.unwrap();
        assert!(!engine.inner.shared.current(4, 1));
        assert!(decoder.next().is_none());
        assert!(!decoder.watch.ended.load(Ordering::Acquire));
        let state = engine.inner.shared.submission.lock().unwrap();
        assert_eq!(state.generation, 5);
        assert!(state.stopped);
        drop(state);
        drop(receiver);
    }

    fn wait(engine: &AudioEngine, predicate: impl Fn(&AudioSnapshot) -> bool) -> AudioSnapshot {
        let start = Instant::now();
        loop {
            let snapshot = engine.snapshot();
            if predicate(&snapshot) {
                return snapshot;
            }
            assert_ne!(snapshot.phase, AudioPhase::Error, "{snapshot:?}");
            assert!(start.elapsed() < Duration::from_secs(8), "{snapshot:?}");
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    #[ignore = "Requires an explicitly owned Pulse null sink and isolated ALSA_CONFIG_PATH; never uses ordinary default output"]
    fn native_owned_null_sink_play_pause_seek_eof_replace_stop_and_cleanup() {
        assert_eq!(
            std::env::var("VELA_NATIVE_TEST_SINK").as_deref(),
            Ok("vela_native_test_null")
        );
        let config = std::env::var_os("ALSA_CONFIG_PATH")
            .expect("isolated ALSA fixture configuration is required");
        let contents = std::fs::read_to_string(config).unwrap();
        // The fixture ALSA configuration makes the host default exactly this
        // owned Pulse null sink, so the normal constructor stays inside the
        // fixture instead of selecting an ordinary output device. CPAL cannot
        // enumerate the pulse alias by name under this synthetic config.
        assert!(contents.contains("vela_native_test_null") && contents.contains("type pulse"));
        let engine = AudioEngine::new().unwrap();
        engine.set_volume(0, 0.0).unwrap();
        wait(&engine, |snapshot| snapshot.volume == 0.0);
        let audio = prepared(FLAC);
        let lease = Arc::downgrade(&audio);
        engine.load(1, audio.clone(), false, 0).unwrap();
        let paused = wait(&engine, |snapshot| {
            snapshot.generation == 1 && snapshot.phase == AudioPhase::Paused
        });
        assert_eq!(paused.volume, 0.0);
        engine.resume(0).unwrap();
        thread::sleep(Duration::from_millis(50));
        assert_eq!(engine.snapshot().phase, AudioPhase::Paused);
        engine.resume(1).unwrap();
        wait(&engine, |snapshot| {
            snapshot.phase == AudioPhase::Playing && snapshot.position_ms > 0
        });
        engine.pause(1).unwrap();
        wait(&engine, |snapshot| snapshot.phase == AudioPhase::Paused);
        engine.seek(1, paused.duration_ms / 2).unwrap();
        wait(&engine, |snapshot| {
            snapshot.phase == AudioPhase::Paused && snapshot.position_ms >= paused.duration_ms / 2
        });
        engine.resume(1).unwrap();
        wait(&engine, |snapshot| snapshot.phase == AudioPhase::Ended);
        engine.load(2, audio.clone(), false, 0).unwrap();
        wait(&engine, |snapshot| {
            snapshot.generation == 2 && snapshot.phase == AudioPhase::Paused
        });
        engine.stop(1).unwrap();
        assert_eq!(engine.snapshot().generation, 2);
        drop(audio);
        engine.stop(3).unwrap();
        wait(&engine, |snapshot| {
            snapshot.generation == 3 && snapshot.phase == AudioPhase::Idle
        });
        let start = Instant::now();
        while lease.upgrade().is_some() && start.elapsed() < Duration::from_secs(2) {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(lease.upgrade().is_none());
    }
}
