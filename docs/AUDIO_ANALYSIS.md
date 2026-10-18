# Local audio analysis

The analysis engine lives in `crates/media/src/analysis.rs`. Its synchronous
`analyze_file(path, limits)` operation decodes real PCM from an already-authorized,
immutable managed media asset. The caller must run it on a bounded blocking
worker. It performs no network access, database writes, media modification or
playback work. Persist a result under the original asset's content hash and the
extractor version `vela-pcm-spectral-4`, independently of editable catalog tags.

This module is an extraction boundary, not a genre classifier, recommendation
service, audio playback engine or proof that a track is legally shareable.
Caller/API/job integration and its verification are recorded separately in
`PROGRESS.md`.

## Dependency verification

On 2026-10-17 the primary upstream documentation identified stable
[Symphonia 0.6.1](https://docs.rs/symphonia/0.6.1/symphonia/) and
[RustFFT 6.4.1](https://docs.rs/rustfft/6.4.1/rustfft/).
Symphonia has an MPL-2.0 license and Rust 1.85 minimum; RustFFT is available under
MIT or Apache-2.0 and requires Rust 1.61. See the actual upstream manifests:
[Symphonia](https://docs.rs/crate/symphonia/0.6.1/source/Cargo.toml.orig) and
[RustFFT](https://docs.rs/crate/rustfft/6.4.1/source/Cargo.toml.orig).

The integration pins exact versions. Symphonia defaults are disabled; enabled
features are AAC, ALAC, FLAC, ISO/MP4, MP3, Ogg, PCM, Vorbis and WAV. This declares
decoder availability rather than claiming every file/container combination has
passed product validation. Opus decoding is unavailable in this dependency and
must fail honestly. RustFFT defaults are disabled for a bounded scalar baseline;
SIMD can be evaluated against measurements later. No FFmpeg subprocess or
resampling dependency is required by this extractor.

## Signal definitions

All frequency values use the decoded PCM sample rate, never an assumed 44.1kHz.
RMS and peak include every channel sample. Floating PCM above digital full scale
is preserved up to the explicit absolute amplitude bound of 16; input is never
silently clipped. dBFS is `20*log10(amplitude)` and is absent for digital silence.
RMS is a digital amplitude measurement, not LUFS, ReplayGain or perceived
loudness. No normalization is applied to stored audio.

Zero-crossing rate is the mean sign-change fraction over adjacent frames and
channels. It is dimensionless in 0..1; zeros are treated as nonnegative. It is not
an estimate of musical tempo or the fundamental frequency.

Spectral analysis uses a 2048-frame Hann window and a 1024-frame hop. Each
channel is transformed independently; one-sided power spectra are averaged
across channels and accumulated over the analyzed prefix. This avoids the loss
of energy caused by summing opposite-phase stereo waveforms. The DC and Nyquist
bins are counted once; other bins have doubled one-sided power. Derived values
are calculated from that accumulated power spectrum:

- Centroid: power-weighted frequency in Hertz.
- Bandwidth: power-weighted standard deviation about the centroid in Hertz.
- Rolloff: the first bin reaching 85% of total power, in Hertz.
- Flatness: geometric/arithmetic mean power, using a relative `mean*1e-12` floor
  to keep logarithms finite and preserve amplitude independence.
- Band energy: eight normalized physical-frequency bands with upper bounds
  125, 250, 500, 1000, 2000, 4000 and 8000Hz; the last band contains remaining
  frequencies up to Nyquist. Bands beyond Nyquist naturally contain zero energy.
- Tempo: half-wave-rectified spectral flux between consecutive windows,
  normalized by the current magnitude sum. Local peaks at least twice the mean
  envelope are onsets (with a refractory gap). Inter-onset intervals are filtered
  to 60–200 BPM; the median interval gives the BPM and confidence is
  `1 − median-absolute-deviation/median`, scaled by interval support. At least64
  windows,4 onsets and3 intervals are required; otherwise tempo is absent.
- Chroma: accumulated power between55Hz and5kHz is folded onto12 equal-tempered
  pitch classes relative to A4=440Hz by nearest semitone and normalized to sum1.
  There is no tuning estimate; retuned material can smear across adjacent classes.
  This is pitch-class energy, not a chord transcription.
- Key: when at least3 pitch classes exceed5% chroma, Pearson-correlate chroma
  rotations against the Krumhansl–Kessler major/minor profiles. The best of24 keys
  supplies a label; confidence is the clamped margin over the next-best profile.
  This is a heuristic fit, not calibrated key detection.

Spectral fields are absent for digital silence. A clip shorter than one FFT
window receives one zero-padded window; RMS/peak/frame counts still use only real
PCM. After a complete FFT window exists, a final incomplete hop is excluded from
the spectrum rather than inventing extra samples. Window count and analyzed
duration are exposed so callers can show coverage. These choices are versioned;
changing them requires a new extractor version and independent cache entries.

Tempo/tempo confidence are an onset-regularity estimate on the bounded analyzed
prefix, not a beat-tracked or annotated-tempo model. Chroma is nearest-semitone
pitch-class energy without tuning/chord transcription; key is a heuristic Krumhansl
profile fit, not calibrated key detection. Genre, mood, speech detection, acoustic
fingerprint and semantic embeddings remain explicitly absent.
There is no trained or calibrated model that would justify filling those fields.

## Bounds, cancellation and failure

Default analysis reads at most the first 120 seconds or 23,040,000 decoded
frames. Reaching either bound returns `complete=false`; a true complete result
requires reaching actual decoder EOF. Exactly hitting a limit remains marked
incomplete even if the source might also end there, because EOF was not observed.
All results expose analyzed frames, milliseconds, sample rate and channels.

Default hard failure limits are eight channels, 192kHz sample rate, 100,000
packets, 1MiB per encoded packet, 65,536 decoded frames per packet, a 2GiB source,
256MiB total source reads and a cooperative 15-second wall deadline. Configuration
is itself validated against fixed ceilings. Format probing uses the same read
and deadline budget, a 64KiB source buffer, bounded metadata tags and no artwork
allocation requested by our metadata options. Old metadata revisions are dropped.
Decoder errors, changing signal specifications, nonfinite/oversized PCM,
unexpected source changes, cancellation and exhausted safety budgets fail the
operation; they do not produce apparently successful features.

The extractor retains a bounded decoded packet, channel windows, one FFT scratch
buffer and 1025 accumulated power bins. It does not retain the full PCM stream.
The optional `Arc<AtomicBool>` cancellation flag is checked during reads/seeks,
between packets and at most every 1024 frames during analysis. The same checks
enforce the wall deadline. This is cooperative cancellation: a decoder call or
blocked filesystem operation cannot be forcibly interrupted. Decoder internals
may allocate before their output reaches our block check; these limits are not
a hard process memory sandbox. Public-peer parsing needs additional isolation if
that trust boundary is introduced.

Only internal managed asset paths belong at this boundary. Obvious symlinks,
empty files and special files are rejected before opening, and regular-file
size/mtime are checked for ordinary changes. This is not a capability-secure
arbitrary filesystem picker or a substitute for authorization/content hashing.
Never accept an external user's path directly; existing ingestion owns the
publication and path-safety boundary. No source path appears in `AudioFeatures`.

## Validation status and reproduction

The module includes deterministic original signal fixtures requiring no external
music license:

- Silence with finite levels and absent spectral values.
- One-second 1kHz sine at 16/44.1/48/96kHz with known RMS, peak, frequency,
  rolloff, flatness, band normalization and crossing-rate tolerances.
- Opposite-phase stereo with the same measurable energy/timbre as mono.
- A broadband impulse with expected RMS/centroid and no invented tempo.
- A generated PCM16 stereo WAV decoded through Symphonia; source BLAKE3 is
  compared before/after, with real frame/rate/channel/feature assertions.
- Existing project-generated `signal.flac` and `beacon.mp3` fixtures decoded
  through their actual codecs, with real duration/rate/channel/energy/feature
  assertions and byte-integrity comparisons.
- Duration/frame prefixes, corrupt input, bytes/packets/blocks/channels/rate/file
  limits, an already elapsed deadline, cancellation and invalid/nonfinite input.
- Sub-window audio with real duration and one zero-padded spectral window.

Integration command, run by the principal who owns compiler scheduling:

```bash
CARGO_BUILD_JOBS=2 cargo test --locked -p vela-media analysis::tests -- --test-threads=2
```

Root integration executed all10 extractor tests in the full157-test workspace run. Actual FLAC/MP3/WAV decoding, the durable worker and explained radio were also exercised through real HTTP and35 Chromium/35 Firefox scenarios. Final evidence and later visual refinements are recorded in PROGRESS/QA_EVIDENCE. PERFORMANCE contains a separately measured20-run generated-PCM release benchmark; it does not establish a hard memory ceiling, native playback, broad codec compatibility or perceptual recommendation quality.

## Durable service integration

`crates/media/src/analysis_service.rs` orchestrates the extractor against migration
`0009_content_audio_analysis.sql`. Requests contain 1..50 unique canonical owned
track IDs. A `BEGIN IMMEDIATE` transaction authorizes the entire batch before any
write, checks 128 global and 50 requester active-job limits, and enqueues only
missing/failed current-version cache entries. An identical queued/running/complete
request is idempotent. Failed content requires an explicit retry. The partial
active-job index keeps quota checks independent of completed-cache size.

The single clone-shared worker claims jobs atomically, requeues interrupted
current-version work at startup and resolves an owned asset through the existing
`MediaService`. Decoding runs in `spawn_blocking`; shutdown sets its cancellation
flag, awaits the blocking task, then requeues an interrupted job. Unexpected DB or
blocking-task failures propagate to the application's worker supervisor. Invalid
or unsupported media becomes a failed individual job. Raw decoder errors, paths,
source tags and requester identities are neither logged nor included in DTOs.

Completed JSON features are capped at 16KiB and keyed by immutable media asset
and extractor version. Owners of identical bytes may reuse descriptors, but
every get/summary request independently authorizes the caller's current tracks.
Playlist metadata grants do not enter this service. The library summary counts
only owned tracks and current-version states in a single SQL read snapshot. Old
extractor versions remain distinct. Admission quotas count only the current extractor version. Startup retires obsolete queued/running rows as failed while retaining completed historical vectors; a regression seeds129 obsolete active records and confirms actual current-version work still completes.

Nine service tests use actual SQLite and ingestion/decoding: original FLAC/MP3,
cache persistence/idempotence, owner isolation and atomic foreign batches, caps
and concurrent reservations, running-checkpoint replay, genuine corrupt-source
failure/retry, active 20-second multichannel WAV shutdown/drain, duplicate worker
rejection and fatal database failure. Quota-only metadata rows are explicitly
test fixtures and never counted as decoded audio. Root executed these tests in the full workspace run; isolated reproduction:

```bash
CARGO_BUILD_JOBS=2 cargo test --locked -p vela-media analysis_service::tests -- --test-threads=2
```
