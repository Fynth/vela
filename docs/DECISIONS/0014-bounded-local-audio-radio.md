# ADR0014: explicit derived PCM analysis and bounded local radio

Accepted2026-10-17; product verification remains separate.

Keep the seven existing crates. Domain owns typed descriptor/status/radio DTOs; media owns Symphonia PCM decoding, RustFFT measurements, durable analysis orchestration and deterministic ranking. The server authenticates/transports; Dioxus requests explicit analysis and invokes the existing root player. No decoder runs on the stream path or an async executor core.

Migration0009 caches a small versioned derived feature vector against immutable asset identity (whose BLAKE3 is unique). JSON is an opaque bounded16KiB algorithm result, not a denormalized user/catalog database. Job status, requester, version and timestamps remain relational/indexed. Current recommendation reads inspect at most500 vectors; a future large-scale similarity index will require measured normalized projections. Reusing equal content never grants another owner's track or reveals another owner's identity.

Explicit batches contain1–50 owned tracks, at most50 requester/128 global active jobs. One supervised serial worker decodes bounded prefixes on the blocking pool, checks cooperative limits/cancellation, drains on shutdown and requeues interruptions. Per-file decode failures expose fixed safe text; database infrastructure failure stops supervision. Completed features remain versioned; extractor upgrades retire obsolete unfinished jobs and must not let old work consume current admission quotas.

The initial extractor measures PCM RMS/peak/ZCR and phase-safe Hann FFT spectral descriptors/bands. It does not infer mood, genre, key, tempo, LUFS, mastering quality or native playback correctness. Radio reauthorizes the seed and all candidates, compares measured spectrum/signal level when available, otherwise uses actual catalog tags and an explicit library fallback. Deterministic diversity penalties and UUID ties avoid fabricated popularity. Candidate coverage is the500 newest other owned tracks, not an exhaustive whole-library nearest-neighbor index. This is track radio, not a personalized weekly/daily mix engine.

Features and radio never modify originals, invoke paid/cloud inference or publish content. Federation, public rights provenance and native engines retain their separate acceptance milestones.
