# 0020: Nearest-semitone chroma

Status: accepted (2026-10-18)

## Context

Tempo landed in spectral-2, but `chroma` remained absent. A chroma field must not be
presented as key detection, chord recognition or tuning analysis. The immutable
extractor cache requires a version boundary for any algorithmic change.

## Decision

`vela-pcm-spectral-3` maps accumulated spectral power between55Hz and5kHz to12
equal-tempered pitch classes relative to A4=440Hz by nearest semitone, then normalizes
the12 values to sum1. It performs no tuning estimate and no key/chord label. Silence
or absent power returns `None`. The extractor version changes so spectral-2 caches are
retained unchanged.

## Consequences

A440 resolves to A and C#4 to C# in synthetic tests; two simultaneous tones expose both
classes. Retuned or inharmonic material can smear across adjacent classes. Chroma is
descriptive only and is not used by radio ranking. A calibrated key/chord model and
real-music accuracy remain future measured work.
