# 0021: Heuristic Krumhansl key estimate

Status: accepted (2026-10-18)

## Context

Chroma landed in spectral-3 but no key field existed. Any key label must be bounded,
must not fabricate a key from a single pitch class, and must not be presented as
calibrated detection. Changing the extractor requires a version boundary.

## Decision

`vela-pcm-spectral-4` estimates a key from normalized chroma when at least three pitch
classes exceed5%. It Pearson-correlates all12 rotations of the Krumhansl–Kessler major
and minor profiles, picks the best of24 keys, and reports the clamped margin over the
next-best profile as confidence. A single pitch class, silence or missing chroma returns
`None`. The extractor version changes so spectral-3 caches are retained unchanged.

## Consequences

Synthetic C-major and A-minor triads resolve correctly; a single A440 tone yields no key.
Real-music accuracy, tuning, modal/key ambiguity and chord transcription are unverified.
The label is descriptive only and is not used by radio ranking.
