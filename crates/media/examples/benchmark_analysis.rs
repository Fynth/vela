//! Sequential offline extractor measurement; no server, streams or user history.
//! Generated fixture creation and BLAKE3 reads are outside timed extraction.

use std::{
    ffi::OsString,
    fs::File,
    hint::black_box,
    io::{self, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
    time::Instant,
};

use serde_json::{Value, json};
use vela_media::analysis::{AnalysisLimits, AudioFeatures, EXTRACTOR_VERSION, analyze_file};

const MAX_INPUTS: usize = 32;
const USAGE: &str = "Usage: benchmark_analysis [--iterations 1..30] [--generate] [--] [audio-file ...]\nGenerates ephemeral original 1s/20s mono, 20s eight-channel and 120s mono WAVs.\nJSON reports sequential warm-cache extraction; paths are reduced to basenames.";

struct Options {
    iterations: usize,
    generate: bool,
    paths: Vec<PathBuf>,
}

fn options() -> Result<Option<Options>, &'static str> {
    let mut result = Options {
        iterations: 3,
        generate: false,
        paths: Vec::new(),
    };
    let mut args = std::env::args_os().skip(1);
    let mut positional = false;
    let mut set_iterations = false;
    while let Some(arg) = args.next() {
        if !positional && arg == "--" {
            positional = true;
        } else if !positional && (arg == "--help" || arg == "-h") {
            return Ok(None);
        } else if !positional && arg == "--generate" {
            result.generate = true;
        } else if !positional && arg == "--iterations" {
            if set_iterations {
                return Err("Iterations may be specified only once");
            }
            set_iterations = true;
            result.iterations = args
                .next()
                .and_then(|value| value.to_str().and_then(|value| value.parse().ok()))
                .filter(|value| (1..=30).contains(value))
                .ok_or("Iterations must be an integer between 1 and 30")?;
        } else if !positional && arg.to_string_lossy().starts_with('-') {
            return Err("Unknown option; use -- before a filename starting with a dash");
        } else {
            result.paths.push(arg.into());
        }
    }
    if result.paths.is_empty() && !result.generate {
        return Err("Provide audio files or --generate");
    }
    if result.paths.len() + if result.generate { 4 } else { 0 } > MAX_INPUTS {
        return Err("At most 32 input files, including generated fixtures, are supported");
    }
    Ok(Some(result))
}

fn main() -> ExitCode {
    let options = match options() {
        Ok(Some(options)) => options,
        Ok(None) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("{message}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(options) {
        Ok((report, successful)) => {
            let mut output = io::BufWriter::new(io::stdout().lock());
            if serde_json::to_writer_pretty(&mut output, &report).is_err()
                || writeln!(output).is_err()
                || output.flush().is_err()
            {
                eprintln!("Cannot write benchmark JSON");
                return ExitCode::from(2);
            }
            if successful {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

fn run(mut options: Options) -> Result<(Value, bool), &'static str> {
    let _generated_directory = if options.generate {
        let directory = tempfile::Builder::new()
            .prefix("vela-analysis-benchmark-")
            .tempdir()
            .map_err(|_| "Cannot create private generated-fixture directory")?;
        for (name, rate, seconds, channels) in [
            ("generated-1s-mono-44100.wav", 44_100, 1, 1),
            ("generated-20s-mono-48000.wav", 48_000, 20, 1),
            ("generated-20s-8ch-48000.wav", 48_000, 20, 8),
            ("generated-120s-mono-48000.wav", 48_000, 120, 1),
        ] {
            let path = directory.path().join(name);
            generate_wav(&path, rate, seconds, channels)
                .map_err(|_| "Cannot write generated PCM fixture")?;
            options.paths.push(path);
        }
        Some(directory)
    } else {
        None
    };
    let mut successful = true;
    let results: Vec<Value> = options
        .paths
        .iter()
        .map(|path| {
            let (result, valid) = measure(path, options.iterations);
            successful &= valid;
            result
        })
        .collect();
    Ok((
        json!({
            "format_version": 1,
            "application_version": env!("CARGO_PKG_VERSION"),
            "extractor_version": EXTRACTOR_VERSION,
            "os": std::env::consts::OS,
            "architecture": std::env::consts::ARCH,
            "iterations_per_file": options.iterations,
            "untimed_warmup_runs_per_file": 1,
            "concurrency": 1,
            "generated_original_fixtures": options.generate,
            "timing_scope": "analyze_file only; sequential warm-cache offline extraction; excludes fixture generation and source hashing",
            "quantile_method": "nearest rank, ceil(p*n)-1 on sorted successful durations",
            "confidence": "1..30 observations per file; p95/p99 usually equal the maximum; descriptive measurements, not population tail estimates",
            "resource_limits": {
                "max_audio_seconds": 120,
                "max_frames": 23_040_000,
                "max_channels": 8,
                "max_sample_rate": 192_000,
                "cooperative_wall_seconds_per_run": 15,
                "max_read_bytes_per_run": 256 * 1024 * 1024,
            },
            "all_runs_valid": successful,
            "results": results,
        }),
        successful,
    ))
}

fn measure(path: &Path, iterations: usize) -> (Value, bool) {
    let name: OsString = path.file_name().unwrap_or_default().to_owned();
    let label: String = name.to_string_lossy().chars().take(128).collect();
    let before = match source_hash(path) {
        Ok(hash) => hash,
        Err(_) => {
            return (
                json!({"source": label, "status": "failed", "error": "input_is_not_a_bounded_regular_readable_file"}),
                false,
            );
        }
    };
    let warmup_start = Instant::now();
    let warmup = analyze_file(path, AnalysisLimits::default());
    let warmup_ms = warmup_start.elapsed().as_secs_f64() * 1000.0;
    let warmup_valid = warmup.is_ok_and(|features| descriptor_valid(black_box(&features)));
    let mut samples = Vec::with_capacity(iterations);
    let mut successful_times = Vec::with_capacity(iterations);
    let mut first_features = None;
    let mut first_fingerprint: Option<String> = None;
    let mut consistent = true;
    let mut failed_runs = 0;
    for iteration in 1..=iterations {
        let start = Instant::now();
        let result = analyze_file(path, AnalysisLimits::default());
        let milliseconds = start.elapsed().as_secs_f64() * 1000.0;
        match result {
            Ok(features) => {
                let features = black_box(features);
                let valid = descriptor_valid(&features);
                let encoded =
                    serde_json::to_vec(&features).expect("fixed descriptor is serializable");
                let fingerprint = blake3::hash(&encoded).to_hex().to_string();
                if let Some(first) = first_fingerprint.as_ref() {
                    consistent &= first == &fingerprint;
                } else {
                    first_fingerprint = Some(fingerprint.clone());
                    first_features = Some(features.clone());
                }
                if valid {
                    successful_times.push(milliseconds);
                } else {
                    failed_runs += 1;
                }
                samples.push(json!({
                    "iteration": iteration,
                    "elapsed_ms": milliseconds,
                    "status": if valid { "ok" } else { "descriptor_sanity_failed" },
                    "descriptor_sanity": valid,
                    "descriptor_blake3": fingerprint,
                    "complete": features.complete,
                    "analyzed_frames": features.analyzed_frames,
                    "analyzed_ms": features.analyzed_ms,
                    "sample_rate": features.sample_rate,
                    "channels": features.channels,
                }));
            }
            Err(_) => {
                failed_runs += 1;
                samples.push(json!({
                    "iteration": iteration,
                    "elapsed_ms": milliseconds,
                    "status": "failed",
                    "error": "unsupported_invalid_or_resource_limited_audio",
                }));
            }
        }
    }
    let after = source_hash(path).ok();
    let unchanged = after.as_ref() == Some(&before);
    let valid = warmup_valid && failed_runs == 0 && consistent && unchanged;
    (
        json!({
            "source": label,
            "source_blake3": before,
            "source_unchanged": unchanged,
            "status": if valid { "ok" } else { "failed" },
            "warmup": {"elapsed_ms": warmup_ms, "descriptor_sanity": warmup_valid},
            "successful_runs": successful_times.len(),
            "failed_runs": failed_runs,
            "consistent_descriptors": consistent,
            "successful_latency_ms": statistics(&mut successful_times),
            "first_successful_descriptor": first_features,
            "samples": samples,
        }),
        valid,
    )
}

fn statistics(values: &mut [f64]) -> Value {
    values.sort_by(f64::total_cmp);
    json!({
        "p50": percentile(values, 0.50),
        "p95": percentile(values, 0.95),
        "p99": percentile(values, 0.99),
        "min": values.first(),
        "max": values.last(),
    })
}

fn percentile(sorted: &[f64], percentile: f64) -> Option<f64> {
    if sorted.is_empty() {
        None
    } else {
        let rank = (percentile * sorted.len() as f64).ceil() as usize;
        Some(sorted[rank.saturating_sub(1).min(sorted.len() - 1)])
    }
}

fn descriptor_valid(features: &AudioFeatures) -> bool {
    let bounded = |value: Option<f64>, low: f64, high: f64| {
        value.is_none_or(|value| value.is_finite() && (low..=high).contains(&value))
    };
    let nyquist = f64::from(features.sample_rate) / 2.0;
    features.extractor_version == EXTRACTOR_VERSION
        && (8000..=192_000).contains(&features.sample_rate)
        && (1..=8).contains(&features.channels)
        && features.analyzed_frames > 0
        && features.analyzed_ms == features.analyzed_frames * 1000 / u64::from(features.sample_rate)
        && features.rms.is_finite()
        && features.rms >= 0.0
        && features.peak.is_finite()
        && features.peak >= features.rms
        && features.peak <= 16.0
        && bounded(features.rms_dbfs, -1000.0, 25.0)
        && bounded(features.peak_dbfs, -1000.0, 25.0)
        && features.zero_crossing_rate.is_finite()
        && (0.0..=1.0).contains(&features.zero_crossing_rate)
        && features.spectral_windows > 0
        && bounded(features.spectral_centroid_hz, 0.0, nyquist)
        && bounded(features.spectral_bandwidth_hz, 0.0, nyquist)
        && bounded(features.spectral_rolloff_hz, 0.0, nyquist)
        && bounded(features.spectral_flatness, 0.0, 1.0)
        && features.band_energy.is_none_or(|bands| {
            bands
                .iter()
                .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
                && (bands.iter().sum::<f64>() - 1.0).abs() < 1e-6
        })
}

fn source_hash(path: &Path) -> io::Result<String> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > AnalysisLimits::default().max_file_bytes
    {
        return Err(io::Error::other("input is not a bounded regular file"));
    }
    let mut source = File::open(path)?;
    let mut hash = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = source.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(hash.finalize().to_hex().to_string())
}

fn generate_wav(path: &Path, rate: u32, seconds: u32, channels: u16) -> io::Result<()> {
    let frames = rate * seconds;
    let byte_len = frames * u32::from(channels) * 2;
    let mut output = BufWriter::new(File::create(path)?);
    output.write_all(b"RIFF")?;
    output.write_all(&(byte_len + 36).to_le_bytes())?;
    output.write_all(b"WAVEfmt ")?;
    output.write_all(&16_u32.to_le_bytes())?;
    output.write_all(&1_u16.to_le_bytes())?;
    output.write_all(&channels.to_le_bytes())?;
    output.write_all(&rate.to_le_bytes())?;
    output.write_all(&(rate * u32::from(channels) * 2).to_le_bytes())?;
    output.write_all(&(channels * 2).to_le_bytes())?;
    output.write_all(&16_u16.to_le_bytes())?;
    output.write_all(b"data")?;
    output.write_all(&byte_len.to_le_bytes())?;
    let mut buffer = Vec::with_capacity(64 * 1024);
    for frame in 0..frames {
        for channel in 0..channels {
            let phase = std::f64::consts::TAU * 440.0 * f64::from(frame) / f64::from(rate)
                + f64::from(channel) * 0.1;
            let sample = (phase.sin() * 16_383.0) as i16;
            buffer.extend_from_slice(&sample.to_le_bytes());
        }
        if buffer.len() >= 64 * 1024 {
            output.write_all(&buffer)?;
            buffer.clear();
        }
    }
    output.write_all(&buffer)?;
    output.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank_quantiles_are_explicit_for_small_samples() {
        assert_eq!(percentile(&[], 0.99), None);
        assert_eq!(percentile(&[7.0], 0.99), Some(7.0));
        assert_eq!(percentile(&[10.0, 20.0, 30.0, 40.0], 0.50), Some(20.0));
        assert_eq!(percentile(&[10.0, 20.0, 30.0, 40.0], 0.95), Some(40.0));
        assert_eq!(percentile(&[10.0, 20.0, 30.0, 40.0], 0.99), Some(40.0));
    }
}
