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

pub const EXTRACTOR_VERSION: &str = "vela-pcm-spectral-1";
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
                self.power[bin] += value.norm_sqr() * factor / self.channels as f64;
            }
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
            tempo_bpm: None,
            tempo_confidence: None,
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
}
