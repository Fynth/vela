//! Bounded, offline PCM analysis. Call only from a bounded blocking worker after
//! authorizing an immutable, managed media asset; never on a streaming request.

use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use rustfft::{Fft, FftPlanner, num_complex::Complex};
use symphonia::core::{
    codecs::audio::AudioDecoderOptions,
    common::Limit,
    formats::{FormatOptions, TrackType, probe::Hint},
    io::{MediaSource, MediaSourceStream},
    meta::MetadataOptions,
};

pub const EXTRACTOR_VERSION: &str = "vela-pcm-spectral-2";
const FFT_SIZE: usize = 2048;
const HOP_SIZE: usize = FFT_SIZE / 2;
const BIN_COUNT: usize = FFT_SIZE / 2 + 1;
/// Upper bounds in Hertz; the last band contains all remaining frequencies.
pub const BAND_UPPER_HZ: [f64; 7] = [125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0];

/// Limits are validated against absolute ceilings before opening the source.
/// A duration/frame limit returns an explicitly incomplete prefix. Other limits
/// fail the analysis and must not be persisted as a successful feature vector.
#[derive(Clone, Debug)]
pub struct AnalysisLimits {
    pub max_duration: Duration,
    pub max_frames: u64,
    pub max_channels: usize,
    pub max_sample_rate: u32,
    pub max_packets: u64,
    pub max_packet_bytes: usize,
    pub max_packet_frames: usize,
    pub max_file_bytes: u64,
    pub max_read_bytes: u64,
    pub wall_time: Duration,
    /// A blocking worker may set this flag on shutdown or job cancellation.
    pub cancel: Option<Arc<AtomicBool>>,
}

impl Default for AnalysisLimits {
    fn default() -> Self {
        Self {
            max_duration: Duration::from_secs(120),
            max_frames: 23_040_000,
            max_channels: 8,
            max_sample_rate: 192_000,
            max_packets: 100_000,
            max_packet_bytes: 1024 * 1024,
            max_packet_frames: 65_536,
            max_file_bytes: 2 * 1024 * 1024 * 1024,
            max_read_bytes: 256 * 1024 * 1024,
            wall_time: Duration::from_secs(15),
            cancel: None,
        }
    }
}

impl AnalysisLimits {
    fn validate(&self) -> Result<()> {
        ensure!(
            !self.max_duration.is_zero() && self.max_duration <= Duration::from_secs(600),
            "invalid analysis duration limit"
        );
        ensure!(
            self.max_frames > 0 && self.max_frames <= 115_200_000,
            "invalid analysis frame limit"
        );
        ensure!(
            (1..=8).contains(&self.max_channels),
            "invalid analysis channel limit"
        );
        ensure!(
            (8000..=192_000).contains(&self.max_sample_rate),
            "invalid analysis sample rate limit"
        );
        ensure!(
            self.max_packets > 0 && self.max_packets <= 1_000_000,
            "invalid analysis packet limit"
        );
        ensure!(
            self.max_packet_bytes > 0 && self.max_packet_bytes <= 4 * 1024 * 1024,
            "invalid analysis packet byte limit"
        );
        ensure!(
            self.max_packet_frames > 0 && self.max_packet_frames <= 65_536,
            "invalid analysis packet frame limit"
        );
        ensure!(
            self.max_file_bytes > 0 && self.max_file_bytes <= 8 * 1024 * 1024 * 1024,
            "invalid analysis file limit"
        );
        ensure!(
            self.max_read_bytes > 0 && self.max_read_bytes <= 1024 * 1024 * 1024,
            "invalid analysis read limit"
        );
        ensure!(
            !self.wall_time.is_zero() && self.wall_time <= Duration::from_secs(60),
            "invalid analysis wall time limit"
        );
        Ok(())
    }
}

pub use vela_domain::AudioFeatures;

#[derive(Clone)]
struct Budget {
    deadline: Instant,
    cancel: Option<Arc<AtomicBool>>,
}

impl Budget {
    fn check(&self) -> io::Result<()> {
        if self
            .cancel
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            return Err(io::Error::other("audio analysis cancelled"));
        }
        if Instant::now() >= self.deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "audio analysis deadline exceeded",
            ));
        }
        Ok(())
    }
}

/// Enforces read and deadline budgets during probing as well as decoding.
struct BoundedSource {
    file: File,
    len: u64,
    remaining: u64,
    budget: Budget,
}

impl Read for BoundedSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.budget.check()?;
        if buf.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            return Err(io::Error::other("audio analysis read limit exceeded"));
        }
        let count = buf.len().min(self.remaining as usize);
        let read = self.file.read(&mut buf[..count])?;
        self.remaining -= read as u64;
        Ok(read)
    }
}

impl Seek for BoundedSource {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        self.budget.check()?;
        self.file.seek(from)
    }
}

impl MediaSource for BoundedSource {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.len)
    }
}

/// Decode the beginning of an already-authorized immutable managed asset.
///
/// Work is cooperative, not a subprocess sandbox: a single decoder call cannot
/// be forcibly interrupted. The caller must bound parallel blocking jobs. This
/// function neither modifies media nor performs network or database operations.
pub fn analyze_file(path: &Path, limits: AnalysisLimits) -> Result<AudioFeatures> {
    limits.validate()?;
    let budget = Budget {
        deadline: Instant::now() + limits.wall_time,
        cancel: limits.cancel.clone(),
    };
    budget.check()?;
    // Arbitrary user paths do not belong here. Reject obvious special files and
    // symlinks before opening; managed immutable storage is the trust boundary.
    let before = std::fs::symlink_metadata(path).context("cannot inspect analysis source")?;
    ensure!(before.is_file(), "analysis source must be a regular file");
    ensure!(
        before.len() > 0 && before.len() <= limits.max_file_bytes,
        "audio analysis file limit exceeded"
    );
    let file = File::open(path).context("cannot open analysis source")?;
    let opened = file.metadata()?;
    ensure!(
        opened.is_file() && opened.len() == before.len(),
        "analysis source changed before opening"
    );
    let source = BoundedSource {
        file,
        len: opened.len(),
        remaining: limits.max_read_bytes,
        budget: budget.clone(),
    };
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(extension);
    }
    let metadata = MetadataOptions::default()
        .limit_tag_bytes(Limit::Maximum(4096))
        .limit_visual_bytes(Limit::Maximum(0));
    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            MediaSourceStream::new(Box::new(source), Default::default()),
            FormatOptions::default(),
            metadata,
        )
        .context("unsupported or invalid audio analysis source")?;
    budget.check()?;
    let track = format
        .default_track(TrackType::Audio)
        .context("analysis source has no audio track")?;
    let track_id = track.id;
    let parameters = track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
        .context("analysis source has no audio parameters")?;
    if let Some(channels) = parameters.channels.as_ref() {
        ensure!(
            channels.count() > 0 && channels.count() <= limits.max_channels,
            "audio analysis channel limit exceeded"
        );
    }
    if let Some(rate) = parameters.sample_rate {
        ensure!(
            (8000..=limits.max_sample_rate).contains(&rate),
            "audio analysis sample rate limit exceeded"
        );
    }
    ensure!(
        parameters
            .max_frames_per_packet
            .is_none_or(|n| n <= limits.max_packet_frames as u64),
        "audio analysis declared block limit exceeded"
    );
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(parameters, &AudioDecoderOptions::default())
        .context("unsupported audio analysis codec")?;
    let mut accumulator: Option<Accumulator> = None;
    let mut interleaved = Vec::<f32>::new();
    let mut packets = 0_u64;
    let mut complete = false;
    loop {
        budget.check()?;
        let Some(packet) = format
            .next_packet()
            .context("invalid analysis audio packet")?
        else {
            complete = true;
            break;
        };
        packets += 1;
        ensure!(
            packets <= limits.max_packets,
            "audio analysis packet limit exceeded"
        );
        ensure!(
            packet.data.len() <= limits.max_packet_bytes,
            "audio analysis encoded block limit exceeded"
        );
        // Metadata is unused, and old revisions need not accumulate in memory.
        while !format.metadata().is_latest() {
            format.metadata().pop();
        }
        if packet.track_id != track_id {
            continue;
        }
        let decoded = decoder
            .decode(&packet)
            .context("invalid decoded analysis audio")?;
        budget.check()?;
        let rate = decoded.spec().rate();
        let channels = decoded.num_planes();
        ensure!(
            (8000..=limits.max_sample_rate).contains(&rate),
            "audio analysis sample rate limit exceeded"
        );
        ensure!(
            channels > 0 && channels <= limits.max_channels,
            "audio analysis channel limit exceeded"
        );
        ensure!(
            decoded.frames() <= limits.max_packet_frames,
            "audio analysis decoded block limit exceeded"
        );
        let analyzer = accumulator.get_or_insert_with(|| Accumulator::new(rate, channels));
        ensure!(
            analyzer.rate == rate && analyzer.channels == channels,
            "analysis audio specification changed"
        );
        let frame_cap = limits
            .max_frames
            .min((limits.max_duration.as_secs_f64() * f64::from(rate)).floor() as u64);
        ensure!(frame_cap > 0, "audio analysis duration contains no frames");
        let available = frame_cap.saturating_sub(analyzer.frames);
        let take = decoded.frames().min(available as usize);
        let sample_count = take
            .checked_mul(channels)
            .context("audio analysis sample count overflow")?;
        interleaved.resize(sample_count, 0.0);
        decoded
            .slice(..take)
            .copy_to_slice_interleaved::<f32, _>(&mut interleaved);
        for (index, frame) in interleaved.chunks_exact(channels).enumerate() {
            if index % HOP_SIZE == 0 {
                budget.check()?;
            }
            analyzer.push(frame)?;
        }
        if analyzer.frames >= frame_cap {
            break;
        }
    }
    budget.check()?;
    let after = std::fs::symlink_metadata(path)?;
    ensure!(
        after.is_file()
            && after.len() == before.len()
            && after.modified().ok() == before.modified().ok(),
        "analysis source changed during decoding"
    );
    let analyzer = accumulator.context("analysis source contains no decoded audio")?;
    ensure!(
        analyzer.frames > 0,
        "analysis source contains no decoded audio"
    );
    Ok(analyzer.finish(complete))
}

struct Accumulator {
    rate: u32,
    channels: usize,
    frames: u64,
    sum_square: f64,
    peak: f64,
    crossings: u64,
    previous: Vec<f32>,
    ring: Vec<Vec<f32>>,
    cursor: usize,
    since_window: usize,
    windows: u64,
    window: Vec<f64>,
    fft: Arc<dyn Fft<f64>>,
    input: Vec<Complex<f64>>,
    scratch: Vec<Complex<f64>>,
    power: [f64; BIN_COUNT],
    previous_power: [f64; BIN_COUNT],
    flux: Vec<f64>,
}

/// Estimate tempo from detected onset peaks in the normalized spectral-flux
/// envelope. Inter-onset intervals are filtered to a 60–200 BPM range and the
/// median interval sets the BPM; confidence is 1 − normalized median absolute
/// deviation (bounded by onset count). This is an onset-regularity estimate, not
/// a beat-tracker/annotation claim.
fn estimate_tempo(flux: &[f64], hop_hz: f64) -> Option<(f64, f64)> {
    if flux.len() < 64 {
        return None;
    }
    let mean = flux.iter().sum::<f64>() / flux.len() as f64;
    let maximum = flux.iter().copied().fold(0.0_f64, f64::max);
    if mean <= 1e-6 || maximum / mean < 2.0 {
        return None;
    }
    let variance = flux.iter().map(|value| (value - mean).powi(2)).sum::<f64>() / flux.len() as f64;
    let threshold = mean + variance.sqrt();
    let min_lag = ((hop_hz * 60.0 / 200.0).floor() as usize).max(2);
    let max_lag = ((hop_hz * 60.0 / 60.0).ceil() as usize).max(min_lag + 1);
    let refractory = (min_lag / 2).max(1);
    let mut onsets = Vec::new();
    for index in 1..flux.len() - 1 {
        if flux[index] > threshold
            && flux[index] >= flux[index - 1]
            && flux[index] >= flux[index + 1]
            && onsets
                .last()
                .is_none_or(|last: &usize| index.saturating_sub(*last) >= refractory)
        {
            onsets.push(index);
        }
    }
    if onsets.len() < 4 {
        return None;
    }
    let mut intervals = onsets
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .filter(|interval| (min_lag..=max_lag).contains(interval))
        .collect::<Vec<_>>();
    if intervals.len() < 3 {
        return None;
    }
    intervals.sort_unstable();
    let median = intervals[intervals.len() / 2] as f64;
    if median <= 0.0 {
        return None;
    }
    let deviation = intervals
        .iter()
        .map(|interval| (*interval as f64 - median).abs())
        .sum::<f64>()
        / intervals.len() as f64;
    let regularity = (1.0 - deviation / median).clamp(0.0, 1.0);
    let support = (intervals.len() as f64 / 8.0).min(1.0);
    let confidence = (regularity * support).clamp(0.0, 1.0);
    if confidence < 0.2 {
        return None;
    }
    let bpm = 60.0 * hop_hz / median;
    if !(30.0..=240.0).contains(&bpm) {
        return None;
    }
    Some((bpm, confidence))
}

impl Accumulator {
    fn new(rate: u32, channels: usize) -> Self {
        let fft = FftPlanner::new().plan_fft_forward(FFT_SIZE);
        let scratch = vec![Complex::default(); fft.get_inplace_scratch_len()];
        Self {
            rate,
            channels,
            frames: 0,
            sum_square: 0.0,
            peak: 0.0,
            crossings: 0,
            previous: vec![0.0; channels],
            ring: vec![vec![0.0; FFT_SIZE]; channels],
            cursor: 0,
            since_window: 0,
            windows: 0,
            window: (0..FFT_SIZE)
                .map(|i| {
                    0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / (FFT_SIZE - 1) as f64).cos()
                })
                .collect(),
            fft,
            input: vec![Complex::default(); FFT_SIZE],
            scratch,
            power: [0.0; BIN_COUNT],
            previous_power: [0.0; BIN_COUNT],
            flux: Vec::new(),
        }
    }

    fn push(&mut self, frame: &[f32]) -> Result<()> {
        for (channel, &sample) in frame.iter().enumerate() {
            ensure!(sample.is_finite(), "analysis audio contains nonfinite PCM");
            let value = f64::from(sample);
            // Floats may legitimately exceed 0dBFS; preserve rather than clip.
            ensure!(
                value.abs() <= 16.0,
                "analysis audio PCM amplitude exceeds limit"
            );
            self.sum_square += value * value;
            self.peak = self.peak.max(value.abs());
            if self.frames > 0 && (sample < 0.0) != (self.previous[channel] < 0.0) {
                self.crossings += 1;
            }
            self.previous[channel] = sample;
            self.ring[channel][self.cursor] = sample;
        }
        self.frames += 1;
        self.cursor = (self.cursor + 1) % FFT_SIZE;
        self.since_window += 1;
        if self.frames >= FFT_SIZE as u64 && self.since_window >= HOP_SIZE {
            self.spectrum(FFT_SIZE);
            self.since_window = 0;
        }
        Ok(())
    }

    fn spectrum(&mut self, valid_frames: usize) {
        // A power average per channel avoids cancellation of antiphase stereo.
        let mut window_power = [0.0_f64; BIN_COUNT];
        for channel in 0..self.channels {
            for index in 0..FFT_SIZE {
                let sample = if index < valid_frames {
                    let start = if self.frames >= FFT_SIZE as u64 {
                        self.cursor
                    } else {
                        0
                    };
                    f64::from(self.ring[channel][(start + index) % FFT_SIZE])
                } else {
                    0.0
                };
                self.input[index] = Complex::new(sample * self.window[index], 0.0);
            }
            self.fft
                .process_with_scratch(&mut self.input, &mut self.scratch);
            for (bin, value) in self.input.iter().take(BIN_COUNT).enumerate() {
                // Correct one-sided power except the unique DC/Nyquist bins.
                let factor = if bin == 0 || bin == BIN_COUNT - 1 {
                    1.0
                } else {
                    2.0
                };
                window_power[bin] += value.norm_sqr() * factor / self.channels as f64;
            }
        }
        // Half-wave-rectified spectral flux is the onset-strength envelope.
        // The first window is a start transient, not an onset observation.
        let magnitude_sum = window_power.iter().map(|power| power.sqrt()).sum::<f64>();
        let flux = if self.windows == 0 || magnitude_sum <= 0.0 {
            0.0
        } else {
            window_power
                .iter()
                .zip(self.previous_power.iter())
                .map(|(now, before)| (now.sqrt() - before.sqrt()).max(0.0))
                .sum::<f64>()
                / magnitude_sum
        };
        self.flux.push(flux);
        self.previous_power = window_power;
        for (total, window) in self.power.iter_mut().zip(window_power.iter()) {
            *total += *window;
        }
        self.windows += 1;
    }

    fn finish(mut self, complete: bool) -> AudioFeatures {
        if self.windows == 0 {
            self.spectrum(self.frames as usize);
        }
        let rms = (self.sum_square / (self.frames as f64 * self.channels as f64)).sqrt();
        let power_sum: f64 = self.power.iter().sum();
        let mut centroid = None;
        let mut bandwidth = None;
        let mut rolloff = None;
        let mut flatness = None;
        let mut bands = None;
        if power_sum > 0.0 {
            let bin_hz = f64::from(self.rate) / FFT_SIZE as f64;
            let center = self
                .power
                .iter()
                .enumerate()
                .map(|(bin, power)| bin as f64 * bin_hz * power)
                .sum::<f64>()
                / power_sum;
            centroid = Some(center);
            bandwidth = Some(
                (self
                    .power
                    .iter()
                    .enumerate()
                    .map(|(bin, power)| (bin as f64 * bin_hz - center).powi(2) * power)
                    .sum::<f64>()
                    / power_sum)
                    .sqrt(),
            );
            let mut sum = 0.0;
            let mut energy = [0.0; 8];
            for (bin, power) in self.power.iter().enumerate() {
                let hz = bin as f64 * bin_hz;
                sum += power;
                if rolloff.is_none() && sum >= power_sum * 0.85 {
                    rolloff = Some(hz);
                }
                let band = BAND_UPPER_HZ.partition_point(|&upper| hz >= upper);
                energy[band] += power / power_sum;
            }
            bands = Some(energy);
            let mean = power_sum / BIN_COUNT as f64;
            // Relative floor is amplitude independent and keeps finite logs.
            let epsilon = mean * 1e-12;
            let geometric = (self
                .power
                .iter()
                .map(|power| (power + epsilon).ln())
                .sum::<f64>()
                / BIN_COUNT as f64)
                .exp();
            flatness = Some((geometric / (mean + epsilon)).clamp(0.0, 1.0));
        }
        let tempo = estimate_tempo(&self.flux, self.rate as f64 / HOP_SIZE as f64);
        AudioFeatures {
            extractor_version: EXTRACTOR_VERSION.to_owned(),
            sample_rate: self.rate,
            channels: self.channels as u32,
            analyzed_frames: self.frames,
            analyzed_ms: self.frames * 1000 / u64::from(self.rate),
            complete,
            rms,
            peak: self.peak,
            rms_dbfs: (rms > 0.0).then(|| 20.0 * rms.log10()),
            peak_dbfs: (self.peak > 0.0).then(|| 20.0 * self.peak.log10()),
            zero_crossing_rate: if self.frames > 1 {
                self.crossings as f64 / ((self.frames - 1) as f64 * self.channels as f64)
            } else {
                0.0
            },
            spectral_windows: self.windows,
            spectral_centroid_hz: centroid,
            spectral_bandwidth_hz: bandwidth,
            spectral_rolloff_hz: rolloff,
            spectral_flatness: flatness,
            band_energy: bands,
            tempo_bpm: tempo.map(|(bpm, _)| bpm),
            tempo_confidence: tempo.map(|(_, confidence)| confidence),
            chroma: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tone(rate: u32, hz: f64, amplitude: f64, channels: usize) -> AudioFeatures {
        let mut accumulator = Accumulator::new(rate, channels);
        for index in 0..rate {
            let sample = (amplitude
                * (std::f64::consts::TAU * hz * f64::from(index) / f64::from(rate)).sin())
                as f32;
            let frame = [sample, -sample];
            accumulator.push(&frame[..channels]).unwrap();
        }
        accumulator.finish(true)
    }

    /// Original generated PCM, with no external music/image rights required.
    fn wav(rate: u32, channels: u16, frames: u32) -> tempfile::NamedTempFile {
        let mut file = tempfile::Builder::new().suffix(".wav").tempfile().unwrap();
        let bytes = frames * u32::from(channels) * 2;
        file.write_all(b"RIFF").unwrap();
        file.write_all(&(36 + bytes).to_le_bytes()).unwrap();
        file.write_all(b"WAVEfmt ").unwrap();
        file.write_all(&16_u32.to_le_bytes()).unwrap();
        file.write_all(&1_u16.to_le_bytes()).unwrap();
        file.write_all(&channels.to_le_bytes()).unwrap();
        file.write_all(&rate.to_le_bytes()).unwrap();
        file.write_all(&(rate * u32::from(channels) * 2).to_le_bytes())
            .unwrap();
        file.write_all(&(channels * 2).to_le_bytes()).unwrap();
        file.write_all(&16_u16.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&bytes.to_le_bytes()).unwrap();
        for index in 0..frames {
            let sample = (16383.0
                * (std::f64::consts::TAU * 1000.0 * f64::from(index) / f64::from(rate)).sin())
                as i16;
            for _ in 0..channels {
                file.write_all(&sample.to_le_bytes()).unwrap();
            }
        }
        file.flush().unwrap();
        file
    }

    /// Interleaved mono PCM for a sine of `hz` at `rate`. Sample values are
    /// computed in `f64` and rounded once to `f32`, mirroring the decoded PCM a
    /// real packet hands to `Accumulator::push`; no dither or normalization is
    /// applied, so amplitude assertions stay meaningful.
    fn pcm_sine(rate: u32, hz: f64, amplitude: f64, frames: usize) -> Vec<f32> {
        (0..frames)
            .map(|index| {
                (amplitude * (std::f64::consts::TAU * hz * index as f64 / f64::from(rate)).sin())
                    as f32
            })
            .collect()
    }

    /// Push interleaved PCM into a fresh accumulator and return it unfinalized,
    /// so a test can inspect the accumulated one-sided spectrum before `finish`.
    fn accumulate(rate: u32, channels: usize, interleaved: &[f32]) -> Accumulator {
        assert_eq!(interleaved.len() % channels, 0, "partial trailing frame");
        let mut accumulator = Accumulator::new(rate, channels);
        for frame in interleaved.chunks_exact(channels) {
            accumulator.push(frame).unwrap();
        }
        accumulator
    }

    /// Centre frequency of one-sided FFT bin `bin`: spacing is `rate / 2048` Hz
    /// and the module keeps 1025 bins, so the last bin is Nyquist.
    fn bin_hz(rate: u32, bin: usize) -> f64 {
        bin as f64 * f64::from(rate) / FFT_SIZE as f64
    }

    /// Largest accumulated power bin and its mean per-window one-sided power.
    /// Requires at least one formed window; a tie keeps the lowest bin.
    fn dominant_bin(accumulator: &Accumulator) -> (usize, f64) {
        assert!(accumulator.windows > 0, "no spectral window was formed");
        let mut best = (0_usize, accumulator.power[0]);
        for (bin, &power) in accumulator.power.iter().enumerate() {
            if power > best.1 {
                best = (bin, power);
            }
        }
        (best.0, best.1 / accumulator.windows as f64)
    }

    /// Coherent gain of the documented symmetric Hann window
    /// `0.5 - 0.5*cos(2*pi*i/(N-1))`: the sum of all 2048 coefficients (1023.5).
    /// Recomputed here as an independent oracle for windowed amplitude.
    fn hann_coherent_sum() -> f64 {
        (0..FFT_SIZE)
            .map(|index| {
                0.5 - 0.5 * (std::f64::consts::TAU * index as f64 / (FFT_SIZE - 1) as f64).cos()
            })
            .sum()
    }

    /// Sum of squared Hann coefficients (767.625), used for the Parseval energy
    /// identity `sum(one_sided_power) = N * sum(x^2)` of this extractor.
    fn hann_square_sum() -> f64 {
        (0..FFT_SIZE)
            .map(|index| {
                let coefficient = 0.5
                    - 0.5 * (std::f64::consts::TAU * index as f64 / (FFT_SIZE - 1) as f64).cos();
                coefficient * coefficient
            })
            .sum()
    }

    /// Assert every present descriptor is finite and normalized, and that the
    /// fields this extractor cannot infer stay explicitly absent. NaN/inf must
    /// never be persisted as an apparently successful feature vector.
    fn assert_finite_descriptors(features: &AudioFeatures) {
        assert!(features.rms.is_finite());
        assert!(features.peak.is_finite());
        assert!(features.zero_crossing_rate.is_finite());
        assert!((0.0..=1.0).contains(&features.zero_crossing_rate));
        for value in [
            features.rms_dbfs,
            features.peak_dbfs,
            features.spectral_centroid_hz,
            features.spectral_bandwidth_hz,
            features.spectral_rolloff_hz,
            features.spectral_flatness,
        ] {
            assert!(
                value.is_none_or(f64::is_finite),
                "non-finite descriptor {value:?}"
            );
        }
        if let Some(bands) = features.band_energy {
            for band in bands {
                assert!(band.is_finite() && band >= 0.0);
            }
            assert!((bands.iter().sum::<f64>() - 1.0).abs() < 1e-9);
        }
        if let Some(bpm) = features.tempo_bpm {
            assert!(bpm.is_finite() && (30.0..=240.0).contains(&bpm));
            assert!(
                features
                    .tempo_confidence
                    .is_some_and(|confidence| (0.0..=1.0).contains(&confidence))
            );
        } else {
            assert!(features.tempo_confidence.is_none());
        }
        assert!(features.chroma.is_none());
    }

    #[test]
    fn silence_has_finite_levels_and_unavailable_spectral_features() {
        let mut accumulator = Accumulator::new(48_000, 2);
        for _ in 0..4800 {
            accumulator.push(&[0.0, 0.0]).unwrap();
        }
        let result = accumulator.finish(true);
        assert_eq!(result.rms, 0.0);
        assert_eq!(result.peak, 0.0);
        assert_eq!(result.zero_crossing_rate, 0.0);
        assert_eq!(result.rms_dbfs, None);
        assert_eq!(result.peak_dbfs, None);
        assert_eq!(result.spectral_centroid_hz, None);
        assert_eq!(result.band_energy, None);
        assert_eq!(result.tempo_bpm, None);
        assert_eq!(result.chroma, None);
    }

    #[test]
    fn sine_frequency_and_amplitude_are_sample_rate_aware() {
        for rate in [16_000, 44_100, 48_000, 96_000] {
            let result = tone(rate, 1000.0, 0.5, 1);
            assert!((result.rms - 0.5 / 2.0_f64.sqrt()).abs() < 1e-6);
            assert!((result.peak - 0.5).abs() < 0.002);
            assert!((result.spectral_centroid_hz.unwrap() - 1000.0).abs() < 1.0);
            assert!((result.spectral_rolloff_hz.unwrap() - 1000.0).abs() < 100.0);
            assert!(result.spectral_flatness.unwrap() < 0.001);
            assert!((result.zero_crossing_rate - 2000.0 / f64::from(rate)).abs() < 0.001);
            assert!((result.band_energy.unwrap().iter().sum::<f64>() - 1.0).abs() < 1e-10);
            assert_eq!(result.analyzed_ms, 1000);
        }
    }

    #[test]
    fn stereo_antiphase_keeps_real_energy_and_timbre() {
        let mono = tone(48_000, 1500.0, 0.7, 1);
        let antiphase = tone(48_000, 1500.0, 0.7, 2);
        assert!((mono.rms - antiphase.rms).abs() < 1e-10);
        assert!(
            (mono.spectral_centroid_hz.unwrap() - antiphase.spectral_centroid_hz.unwrap()).abs()
                < 1e-8
        );
        for (mono, stereo) in mono
            .band_energy
            .unwrap()
            .iter()
            .zip(antiphase.band_energy.unwrap())
        {
            assert!((mono - stereo).abs() < 1e-10);
        }
    }

    #[test]
    fn impulse_is_broadband_but_does_not_invent_tempo() {
        let mut accumulator = Accumulator::new(48_000, 1);
        for index in 0..FFT_SIZE {
            accumulator
                .push(&[if index == FFT_SIZE / 2 { 1.0 } else { 0.0 }])
                .unwrap();
        }
        let result = accumulator.finish(true);
        assert!((result.rms - (1.0 / FFT_SIZE as f64).sqrt()).abs() < 1e-12);
        assert_eq!(result.peak, 1.0);
        assert!((result.spectral_centroid_hz.unwrap() - 12_000.0).abs() < 1.0);
        assert!(result.spectral_flatness.unwrap() > 0.99);
        assert_eq!(result.tempo_bpm, None);
    }

    #[test]
    fn actual_wav_decoding_preserves_file_and_measures_pcm() {
        let file = wav(48_000, 2, 48_000);
        let before = blake3::hash(&std::fs::read(file.path()).unwrap());
        let features = analyze_file(file.path(), AnalysisLimits::default()).unwrap();
        assert_eq!(features.extractor_version, EXTRACTOR_VERSION);
        assert_eq!(features.channels, 2);
        assert_eq!(features.sample_rate, 48_000);
        assert_eq!(features.analyzed_frames, 48_000);
        assert_eq!(features.analyzed_ms, 1000);
        assert!(features.complete);
        assert!((features.rms - 0.3535).abs() < 0.001);
        assert!((features.spectral_centroid_hz.unwrap() - 1000.0).abs() < 2.0);
        assert_eq!(before, blake3::hash(&std::fs::read(file.path()).unwrap()));
    }

    #[test]
    fn existing_generated_flac_and_mp3_decode_without_mutation() {
        for name in ["signal.flac", "beacon.mp3"] {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name);
            let before = blake3::hash(&std::fs::read(&path).unwrap());
            let features = analyze_file(&path, AnalysisLimits::default()).unwrap();
            assert_eq!(features.sample_rate, 44_100);
            assert_eq!(features.channels, 1);
            assert!((900..=1100).contains(&features.analyzed_ms));
            assert!(features.complete);
            assert!(features.rms > 0.0 && features.rms < 1.0);
            assert!(features.peak > features.rms && features.peak <= 1.0);
            assert!(features.spectral_centroid_hz.unwrap().is_finite());
            assert!(features.spectral_rolloff_hz.unwrap() <= 22_050.0);
            assert_eq!(before, blake3::hash(&std::fs::read(&path).unwrap()));
        }
    }

    #[test]
    fn duration_and_frame_limits_return_explicit_prefixes() {
        let file = wav(48_000, 1, 48_000);
        let duration = AnalysisLimits {
            max_duration: Duration::from_millis(250),
            ..Default::default()
        };
        let features = analyze_file(file.path(), duration).unwrap();
        assert_eq!(features.analyzed_frames, 12_000);
        assert_eq!(features.analyzed_ms, 250);
        assert!(!features.complete);
        let frames = AnalysisLimits {
            max_frames: 3000,
            ..Default::default()
        };
        let features = analyze_file(file.path(), frames).unwrap();
        assert_eq!(features.analyzed_frames, 3000);
        assert!(!features.complete);
    }

    #[test]
    fn malformed_input_and_resource_limits_do_not_produce_features() {
        let mut corrupt = tempfile::NamedTempFile::new().unwrap();
        corrupt.write_all(b"not an audio source").unwrap();
        assert!(analyze_file(corrupt.path(), AnalysisLimits::default()).is_err());
        let file = wav(48_000, 2, 48_000);
        for limits in [
            AnalysisLimits {
                max_file_bytes: 100,
                ..Default::default()
            },
            AnalysisLimits {
                max_read_bytes: 64,
                ..Default::default()
            },
            AnalysisLimits {
                max_channels: 1,
                ..Default::default()
            },
            AnalysisLimits {
                max_sample_rate: 16_000,
                ..Default::default()
            },
            AnalysisLimits {
                max_packets: 1,
                ..Default::default()
            },
            AnalysisLimits {
                max_packet_bytes: 1,
                ..Default::default()
            },
            AnalysisLimits {
                max_packet_frames: 1,
                ..Default::default()
            },
            AnalysisLimits {
                wall_time: Duration::from_nanos(1),
                ..Default::default()
            },
            AnalysisLimits {
                cancel: Some(Arc::new(AtomicBool::new(true))),
                ..Default::default()
            },
        ] {
            assert!(analyze_file(file.path(), limits).is_err());
        }
    }

    #[test]
    fn invalid_limits_and_nonfinite_pcm_are_rejected() {
        let limits = AnalysisLimits {
            max_channels: usize::MAX,
            ..Default::default()
        };
        assert!(limits.validate().is_err());
        let mut accumulator = Accumulator::new(48_000, 1);
        assert!(accumulator.push(&[f32::NAN]).is_err());
        assert!(accumulator.push(&[f32::INFINITY]).is_err());
        assert!(accumulator.push(&[17.0]).is_err());
    }

    #[test]
    fn short_audio_zero_pads_only_its_spectral_window() {
        let mut accumulator = Accumulator::new(48_000, 1);
        for _ in 0..48 {
            accumulator.push(&[0.25]).unwrap();
        }
        let result = accumulator.finish(true);
        assert_eq!(result.analyzed_frames, 48);
        assert_eq!(result.analyzed_ms, 1);
        assert_eq!(result.rms, 0.25);
        assert_eq!(result.spectral_windows, 1);
        assert!(result.spectral_centroid_hz.unwrap().is_finite());
    }

    /// 1 kHz mono sine at 44.1 kHz, 1 s: the dominant FFT bin must be within one
    /// bin of 1 kHz, and the peak-bin power must match the Hann-windowed
    /// amplitude. Bin spacing is 44100/2048 = 21.533 Hz, so 1 kHz lands on bin
    /// 46.44 and cannot be exactly bin-centred; the mandate's +/-1 bin tolerance
    /// covers bins 46 and 47. The maximum is bin 46.
    #[test]
    fn one_khz_tone_lands_in_expected_fft_bin_with_hann_scale() {
        let rate = 44_100_u32;
        let amplitude = 0.5_f64;
        let pcm = pcm_sine(rate, 1000.0, amplitude, rate as usize);
        let accumulator = accumulate(rate, 1, &pcm);
        assert_eq!(accumulator.frames, 44_100);
        // Windows form at frames 2048, 3072, ...; 42 complete 2048-frame windows
        // fit in the 44100-frame prefix, and the trailing partial hop is excluded.
        assert_eq!(accumulator.windows, 42);
        assert!(
            accumulator
                .power
                .iter()
                .all(|power| power.is_finite() && *power >= 0.0)
        );

        let (bin, peak_power) = dominant_bin(&accumulator);
        let step = f64::from(rate) / FFT_SIZE as f64;
        assert!(
            (bin_hz(rate, bin) - 1000.0).abs() <= step,
            "dominant bin {bin} at {} Hz is not within one {step} Hz bin of 1 kHz",
            bin_hz(rate, bin)
        );
        assert_eq!(bin, 46, "expected 1000/21.533 = 46.44 -> bin 46");
        assert!(accumulator.power[bin] > accumulator.power[bin - 1]);
        assert!(accumulator.power[bin] > accumulator.power[bin + 1]);

        // Hann coherent gain: a bin-centred real sine of amplitude A has a
        // one-sided peak power of 2*(A*sum(w)/2)^2 per window (sum(w) = 1023.5).
        // Off-centre tones suffer scalloping; the worst case at half a bin is
        // 1.42 dB = 0.72 in power. 1 kHz is 0.44 bins off centre and measures
        // 0.777 of the bin-centred value, so the documented band is [0.72, 1.0].
        let coherent = 2.0 * (amplitude * hann_coherent_sum() / 2.0).powi(2);
        let ratio = peak_power / coherent;
        assert!(
            (0.72..=1.0).contains(&ratio),
            "peak-bin Hann power ratio {ratio} outside [0.72, 1.0] \
             (measured {peak_power}, bin-centred {coherent})"
        );

        // Parseval cross-check of the one-sided doubling convention: all 1025
        // bins sum to N * sum(x^2) per window. For A = 0.5 and sum(w^2) = 767.625
        // the expectation is 2048 * 0.25 * 767.625 / 2 = 196512; a 0.1% band
        // absorbs the tiny non-integer-cycle residue of a 1 kHz sine.
        let total_power = accumulator.power.iter().sum::<f64>() / accumulator.windows as f64;
        let expected_total = FFT_SIZE as f64 * (amplitude * amplitude * hann_square_sum() / 2.0);
        assert!(
            (total_power / expected_total - 1.0).abs() < 1e-3,
            "per-window total power {total_power} vs Parseval expectation {expected_total}"
        );

        let features = accumulator.finish(true);
        assert_eq!(features.sample_rate, 44_100);
        assert_eq!(features.channels, 1);
        assert_eq!(features.analyzed_frames, 44_100);
        assert_eq!(features.analyzed_ms, 1000);
        assert_eq!(features.spectral_windows, 42);
        assert_finite_descriptors(&features);

        // Exact coherent-gain check at a genuinely bin-centred frequency: at
        // 81.92 kHz the spacing is 40 Hz, so 1 kHz is exactly bin 25. The
        // per-window peak then equals 2*(A*sum(w)/2)^2 to ~1e-6 relative; 1e-3
        // absorbs f32 input rounding and the negligible Hann sidelobe at bin 50.
        let centred_rate = 81_920_u32;
        let centred_pcm = pcm_sine(centred_rate, 1000.0, amplitude, centred_rate as usize);
        let centred = accumulate(centred_rate, 1, &centred_pcm);
        assert_eq!(centred.windows, 79);
        let (centred_bin, centred_power) = dominant_bin(&centred);
        assert_eq!(centred_bin, 25);
        assert!((bin_hz(centred_rate, centred_bin) - 1000.0).abs() < 1e-9);
        assert!(
            (centred_power / coherent - 1.0).abs() < 1e-3,
            "bin-centred peak power {centred_power} vs Hann expectation {coherent}"
        );
        // A bin-centred Hann main lobe splits its energy as 0.5/0.25/0.25 across
        // the centre bin and its two neighbours; the neighbours are symmetric.
        assert!(centred.power[centred_bin] > 3.0 * centred.power[centred_bin - 1]);
        assert!(
            (centred.power[centred_bin - 1] / centred.power[centred_bin + 1] - 1.0).abs() < 1e-5
        );
    }

    /// 220 Hz + 660 Hz two-tone: both peaks must exist at their expected bins,
    /// stay local maxima, and be ordered by magnitude with the 4:1 power ratio
    /// implied by amplitudes 0.4 and 0.2.
    #[test]
    fn two_tone_spectrum_keeps_both_peaks_in_amplitude_order() {
        let rate = 44_100_u32;
        let step = f64::from(rate) / FFT_SIZE as f64;
        // Sum in f64 then round once: peak sample is 0.6, below full scale, so no
        // amplitude guard can trip. 220 Hz is bin 10.22, 660 Hz is bin 30.65.
        let pcm: Vec<f32> = (0..rate as usize)
            .map(|index| {
                let time = index as f64 / f64::from(rate);
                let sample = 0.4 * (std::f64::consts::TAU * 220.0 * time).sin()
                    + 0.2 * (std::f64::consts::TAU * 660.0 * time).sin();
                sample as f32
            })
            .collect();
        let accumulator = accumulate(rate, 1, &pcm);
        assert_eq!(accumulator.windows, 42);

        let low = (220.0 / step).round() as usize;
        let high = (660.0 / step).round() as usize;
        assert_eq!((low, high), (10, 31));
        assert!((bin_hz(rate, low) - 220.0).abs() <= step / 2.0);
        assert!((bin_hz(rate, high) - 660.0).abs() <= step / 2.0);
        // Presence: each expected bin must be a strict local maximum.
        assert!(accumulator.power[low] > accumulator.power[low - 1]);
        assert!(accumulator.power[low] > accumulator.power[low + 1]);
        assert!(accumulator.power[high] > accumulator.power[high - 1]);
        assert!(accumulator.power[high] > accumulator.power[high + 1]);

        let per_window = |bin: usize| accumulator.power[bin] / accumulator.windows as f64;
        let mean_bin =
            accumulator.power.iter().sum::<f64>() / accumulator.windows as f64 / BIN_COUNT as f64;
        // Measured 514x and 117x the mean bin power: a sparse tonal spectrum.
        assert!(
            per_window(low) > 50.0 * mean_bin,
            "220 Hz peak not clearly above the spectral mean"
        );
        assert!(
            per_window(high) > 20.0 * mean_bin,
            "660 Hz peak not clearly above the spectral mean"
        );
        // Ordering: 0.4 amplitude must dominate 0.2 amplitude.
        assert!(per_window(low) > per_window(high));

        // Amplitude order is a power ratio of 4.0, but each tone suffers its own
        // Hann scalloping (220 Hz is 0.22 bins off centre, 660 Hz 0.35 bins), so
        // the documented band is 4 * [0.72, 1/0.72] = [2.88, 5.56]; measured 4.41.
        let ratio = per_window(low) / per_window(high);
        assert!(
            (2.8..=5.6).contains(&ratio),
            "two-tone power ratio {ratio} outside the scalloping band [2.8, 5.6]"
        );

        let features = accumulator.finish(true);
        let bands = features.band_energy.unwrap();
        // 220 Hz falls in band 1 (125, 250] Hz, 660 Hz in band 3 (500, 1000] Hz.
        assert!(bands[1] > 0.70, "220 Hz band energy {}", bands[1]);
        assert!(bands[3] > 0.15, "660 Hz band energy {}", bands[3]);
        let centroid = features.spectral_centroid_hz.unwrap();
        assert!((220.0..660.0).contains(&centroid));
        // Two tones only: the spectrum is sparse, so flatness stays near zero.
        assert!(features.spectral_flatness.unwrap() < 1e-6);
        assert_finite_descriptors(&features);
    }

    /// Digital silence: zero RMS/peak, exactly zero power in every bin, no NaN,
    /// and no fabricated dominant frequency, band energy or confidence.
    #[test]
    fn digital_silence_has_zero_power_and_no_invented_fields() {
        let rate = 48_000_u32;
        let pcm = vec![0.0_f32; rate as usize];
        let accumulator = accumulate(rate, 1, &pcm);
        // Windows are still formed for silence, but every bin is exactly +0.0:
        // no -0.0/NaN may appear and no statistic may be divided out of zero.
        assert!(accumulator.windows > 0);
        assert!(accumulator.power.iter().all(|power| *power == 0.0));
        let windows = accumulator.windows;
        let features = accumulator.finish(true);
        assert_eq!(features.analyzed_frames, 48_000);
        assert_eq!(features.analyzed_ms, 1000);
        assert_eq!(features.spectral_windows, windows);
        assert_eq!(features.rms, 0.0);
        assert_eq!(features.peak, 0.0);
        assert_eq!(features.zero_crossing_rate, 0.0);
        assert_eq!(features.rms_dbfs, None);
        assert_eq!(features.peak_dbfs, None);
        // Absence, not 0 Hz/0.0: the extractor has no dominant-frequency field
        // other than the centroid, and it must not invent one for silence.
        assert_eq!(features.spectral_centroid_hz, None);
        assert_eq!(features.spectral_bandwidth_hz, None);
        assert_eq!(features.spectral_rolloff_hz, None);
        assert_eq!(features.spectral_flatness, None);
        assert_eq!(features.band_energy, None);
        assert_eq!(features.tempo_bpm, None);
        assert_eq!(features.tempo_confidence, None);
        assert_eq!(features.chroma, None);
        assert_finite_descriptors(&features);
    }

    /// A clip shorter than one 2048-frame analysis window must yield exactly one
    /// zero-padded window with real frame counts, and the shortest possible clip
    /// must report absence rather than a fabricated centroid. No panic/no garbage.
    #[test]
    fn sub_window_clip_gets_one_zero_padded_window_without_fabrication() {
        // 1024 frames at 44.1 kHz is 23 ms, half a window.
        let rate = 44_100_u32;
        let pcm = pcm_sine(rate, 220.0, 0.25, 1024);
        let accumulator = accumulate(rate, 1, &pcm);
        assert_eq!(accumulator.frames, 1024);
        assert_eq!(accumulator.windows, 0, "no complete FFT window yet");
        let features = accumulator.finish(true);
        assert_eq!(features.spectral_windows, 1);
        assert_eq!(features.analyzed_frames, 1024);
        assert_eq!(features.analyzed_ms, 23);
        assert!(features.complete);
        // RMS/peak count real PCM only: 1024 samples of a 0.25-amplitude 220 Hz
        // sine span 5.1 cycles, so RMS is slightly below the asymptotic A/sqrt(2).
        assert!((features.rms - 0.17539).abs() < 1e-4);
        assert!((features.peak - 0.25).abs() < 1e-5);
        // Zero-padding a 1024-sample Hann window broadens the main lobe (1024
        // samples => 43.07 Hz resolution), so the centroid sits above 220 Hz. It
        // must remain a finite plausible frequency, not a fabricated 0 Hz value.
        let centroid = features.spectral_centroid_hz.unwrap();
        assert!((150.0..=350.0).contains(&centroid), "centroid {centroid}");
        assert!((0.0..f64::from(rate) / 2.0).contains(&features.spectral_rolloff_hz.unwrap()));
        assert!((0.0..0.1).contains(&features.spectral_flatness.unwrap()));
        assert_finite_descriptors(&features);

        // Shortest possible clip: one frame. Hann w[0] = 0 multiplies the only
        // sample to nothing, so the padded window has zero power and the
        // extractor reports absence instead of a fake 0 Hz centroid.
        let mut one = Accumulator::new(rate, 1);
        one.push(&[0.5]).unwrap();
        let one = one.finish(true);
        assert_eq!(one.analyzed_frames, 1);
        assert_eq!(one.analyzed_ms, 0);
        assert_eq!(one.spectral_windows, 1);
        assert_eq!(one.rms, 0.5);
        assert_eq!(one.peak, 0.5);
        assert_eq!(one.zero_crossing_rate, 0.0);
        assert_eq!(one.spectral_centroid_hz, None);
        assert_eq!(one.spectral_flatness, None);
        assert_finite_descriptors(&one);

        // Same policy through the real Symphonia path: a 1024-frame PCM16 WAV is
        // shorter than one analysis window and must still decode to an explicit,
        // finite result rather than an error or garbage.
        let file = wav(rate, 1, 1024);
        let decoded = analyze_file(file.path(), AnalysisLimits::default()).unwrap();
        assert_eq!(decoded.analyzed_frames, 1024);
        assert_eq!(decoded.analyzed_ms, 23);
        assert_eq!(decoded.spectral_windows, 1);
        assert!(decoded.complete);
        assert!((decoded.rms - 0.3531).abs() < 1e-3);
        assert!((decoded.peak - 0.49994).abs() < 1e-3);
        assert!(
            (500.0..=1500.0).contains(&decoded.spectral_centroid_hz.unwrap()),
            "decoded sub-window centroid {:?}",
            decoded.spectral_centroid_hz
        );
        assert_finite_descriptors(&decoded);
    }

    /// Full-scale square, clipped sine and deliberately oversized PCM: peaks stay
    /// bounded at digital full scale (or are preserved above it by documented
    /// policy), and every descriptor remains finite. There is no clipping flag in
    /// `AudioFeatures`, so the implemented contract is peak level plus finiteness.
    #[test]
    fn full_scale_clipped_and_oversized_pcm_remain_finite() {
        let rate = 44_100_u32;
        // Full-scale 200 Hz square: samples are exactly +/-1.0, so peak and RMS
        // are exactly 1.0 and both dBFS descriptors are exactly 0 dB.
        let square: Vec<f32> = (0..rate as usize)
            .map(|index| {
                if (std::f64::consts::TAU * 200.0 * index as f64 / f64::from(rate)).sin() >= 0.0 {
                    1.0
                } else {
                    -1.0
                }
            })
            .collect();
        let features = accumulate(rate, 1, &square).finish(true);
        assert_eq!(features.peak, 1.0);
        assert_eq!(features.rms, 1.0);
        assert_eq!(features.peak_dbfs, Some(0.0));
        assert_eq!(features.rms_dbfs, Some(0.0));
        // Two sign changes per cycle at 200 Hz: ZCR = 2*200/44100 = 0.00907.
        // Sample-grid quantization moves it by <1e-4, hence the band.
        assert!((features.zero_crossing_rate - 400.0 / 44_100.0).abs() < 1e-4);
        // A full-scale square is a 1/n harmonic series: low centroid, flatness in
        // (0, 1), and all band energy finite and normalized.
        let centroid = features.spectral_centroid_hz.unwrap();
        assert!((0.0..f64::from(rate) / 2.0).contains(&centroid));
        assert!((0.0..1.0).contains(&features.spectral_flatness.unwrap()));
        assert_finite_descriptors(&features);

        // Clipped sine (1.5 amplitude clamped to +/-1) stays bounded at 0 dBFS but
        // is louder than a clean 1.0-amplitude sine because it dwells at +/-1.
        let clipped: Vec<f32> = (0..rate as usize)
            .map(|index| {
                (1.5 * (std::f64::consts::TAU * 1000.0 * index as f64 / f64::from(rate)).sin())
                    .clamp(-1.0, 1.0) as f32
            })
            .collect();
        let clipped = accumulate(rate, 1, &clipped).finish(true);
        assert_eq!(clipped.peak, 1.0);
        assert_eq!(clipped.peak_dbfs, Some(0.0));
        let clean = tone(rate, 1000.0, 1.0, 1);
        assert!(
            clipped.rms > clean.rms,
            "clipped RMS {} should exceed clean sine RMS {}",
            clipped.rms,
            clean.rms
        );
        assert!((clipped.rms - 0.83797).abs() < 1e-3);
        assert_finite_descriptors(&clipped);

        // Documented policy (AUDIO_ANALYSIS.md): floating PCM above 0 dBFS is
        // preserved up to an absolute bound of 16, never silently clipped, so a
        // genuine +6 dB over survives instead of a fabricated 0 dBFS ceiling.
        let oversized = pcm_sine(rate, 1000.0, 2.0, rate as usize);
        let oversized = accumulate(rate, 1, &oversized).finish(true);
        assert!(
            oversized.peak > 1.99,
            "oversized peak {} was not preserved",
            oversized.peak
        );
        assert!(oversized.peak <= 2.0);
        assert!(oversized.peak_dbfs.unwrap() > 6.0);
        assert!((oversized.rms - 2.0 / 2.0_f64.sqrt()).abs() < 1e-3);
        assert_finite_descriptors(&oversized);
    }

    /// Determinism: extracting identical PCM twice must produce exactly equal
    /// descriptors (f64 `==`, so any NaN would fail) both in-module and through
    /// the real decoder, including the serialized form that gets cached.
    #[test]
    fn repeated_extraction_of_identical_pcm_is_bit_identical() {
        let rate = 44_100_u32;
        // 4096 frames is 3 complete windows at 44.1 kHz (2048/3072/4096).
        let pcm = pcm_sine(rate, 440.0, 0.6, 4096);
        let first = accumulate(rate, 1, &pcm).finish(true);
        let second = accumulate(rate, 1, &pcm).finish(true);
        assert_eq!(first.spectral_windows, 3);
        assert!(first.spectral_centroid_hz.is_some());
        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );

        // Same invariant through Symphonia: 100 ms stereo PCM16 WAV, twice.
        let file = wav(48_000, 2, 4800);
        let a = analyze_file(file.path(), AnalysisLimits::default()).unwrap();
        let b = analyze_file(file.path(), AnalysisLimits::default()).unwrap();
        assert_eq!(a.extractor_version, EXTRACTOR_VERSION);
        assert_eq!(a.analyzed_frames, 4800);
        assert!(a.spectral_windows > 0);
        assert_eq!(a, b);
    }

    /// 120 BPM click train (0.5 s = 22050 frames at 44.1 kHz). This extractor
    /// exposes no onset/tempo detector, so `tempo_bpm`/`tempo_confidence` must stay
    /// absent; the actual implemented descriptor (ZCR) is asserted instead and is
    /// shown to be sign-based, not onset-based, by exact hand-built PCM.
    #[test]
    fn periodic_clicks_do_not_fabricate_tempo_and_zcr_is_sign_based() {
        let rate = 44_100_u32;
        let mut clicks = vec![0.0_f32; rate as usize];
        clicks[0] = 1.0;
        clicks[22_050] = 1.0;
        let features = accumulate(rate, 1, &clicks).finish(true);
        // Zeros are nonnegative, so a positive-only impulse train never crosses
        // the sign threshold: ZCR is exactly 0 and cannot stand in for tempo.
        assert_eq!(features.zero_crossing_rate, 0.0);
        // RMS of two unit impulses over 44100 frames: sqrt(2/44100).
        assert!((features.rms - (2.0_f64 / 44_100.0).sqrt()).abs() < 1e-9);
        assert_eq!(features.peak, 1.0);
        // An impulse train is broadband: flatness near 1, finite centroid.
        assert!(features.spectral_flatness.unwrap() > 0.9);
        assert!((0.0..f64::from(rate) / 2.0).contains(&features.spectral_centroid_hz.unwrap()));
        assert_eq!(features.tempo_bpm, None);
        assert_eq!(features.tempo_confidence, None);
        assert_eq!(features.chroma, None);
        assert_finite_descriptors(&features);

        // Exact ZCR semantics: sign changes only, zeros count as nonnegative,
        // denominator is frames-1 times channels, and <2 frames yields 0.
        let zcr = |samples: &[f32]| accumulate(rate, 1, samples).finish(true).zero_crossing_rate;
        assert_eq!(zcr(&[0.5, -0.5, 0.5]), 1.0); // 2 crossings over 2 gaps
        assert_eq!(zcr(&[0.0, -1.0, 0.0]), 1.0); // 0 -> -1 and -1 -> 0
        assert_eq!(zcr(&[1.0, 1.0, 1.0]), 0.0); // no sign change
        assert_eq!(zcr(&[0.5]), 0.0); // degenerate single frame
    }

    /// -60 dBFS sine: this extractor has no absolute silence gate, so a very
    /// quiet but real tone must keep finite descriptors and must not be reported
    /// as silence. Only exact digital silence (rms == 0) is reported as absent.
    #[test]
    fn quiet_minus_60_dbfs_tone_is_not_mistaken_for_silence() {
        let rate = 44_100_u32;
        // Amplitude sqrt(2)*1e-3 gives RMS = 1e-3 = -60 dBFS.
        let quiet = tone(rate, 1000.0, 2.0_f64.sqrt() * 1e-3, 1);
        let rms_dbfs = quiet
            .rms_dbfs
            .expect("a quiet but non-silent tone must have an RMS dBFS value");
        // f32 input rounding bounds the relative RMS error below ~0.001 dB, so a
        // 0.01 dB band is a documented, safe tolerance for -60 dBFS.
        assert!((rms_dbfs + 60.0).abs() < 0.01, "rms_dbfs {rms_dbfs}");
        assert!((quiet.rms - 1e-3).abs() < 1e-8);
        // Peak of a sine is +3.0103 dB above its RMS: -56.99 dBFS.
        assert!((quiet.peak_dbfs.unwrap() + 56.9897).abs() < 0.02);
        assert!((0.0..0.01).contains(&quiet.rms));
        let centroid = quiet
            .spectral_centroid_hz
            .expect("no silence threshold may erase a real -60 dBFS spectrum");
        assert!((centroid - 1000.0).abs() < 1.0, "centroid {centroid}");
        assert!(quiet.spectral_flatness.unwrap() < 1e-3);
        assert_finite_descriptors(&quiet);
    }

    #[test]
    fn eight_second_click_train_estimates_120_bpm_with_bounded_confidence() {
        let rate = 44_100_u32;
        let frames = rate as usize * 8;
        let mut clicks = vec![0.0_f32; frames];
        for index in (0..frames).step_by(22_050) {
            clicks[index] = 1.0;
        }
        let features = accumulate(rate, 1, &clicks).finish(true);
        let bpm = features.tempo_bpm.expect("periodic clicks expose a tempo");
        assert!(
            (115.0..=125.0).contains(&bpm),
            "expected about120 BPM, measured {bpm}"
        );
        let confidence = features.tempo_confidence.unwrap();
        assert!(
            (0.25..=1.0).contains(&confidence),
            "confidence must stay bounded: {confidence}"
        );
        assert_eq!(features.chroma, None);
        assert_finite_descriptors(&features);
    }

    /// Non-finite and out-of-bound PCM must fail the push so no descriptor is
    /// ever built, while the documented absolute amplitude bound of 16 is exactly
    /// inclusive and preserved rather than clamped.
    #[test]
    fn nonfinite_and_oversized_samples_fail_before_descriptors_are_built() {
        let mut accumulator = Accumulator::new(48_000, 2);
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(accumulator.push(&[0.1, bad]).is_err(), "accepted {bad}");
        }
        let mut at_bound = Accumulator::new(48_000, 1);
        at_bound.push(&[16.0]).unwrap();
        at_bound.push(&[-16.0]).unwrap();
        let at_bound = at_bound.finish(true);
        assert_eq!(at_bound.peak, 16.0);
        assert!(at_bound.peak_dbfs.unwrap() > 24.0);
        assert_finite_descriptors(&at_bound);
        let mut over = Accumulator::new(48_000, 1);
        assert!(over.push(&[16.5]).is_err());
        assert!(over.push(&[-17.0]).is_err());
        // A failed push is an abort, not a silent skip: `analyze_file` propagates
        // the error and never persists a feature vector for such a source.
    }
}
