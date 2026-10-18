# 0019: Bounded onset-interval tempo estimation

Status: accepted (2026-10-18)

## Context

`vela-pcm-spectral-1` deliberately left `tempo_bpm`, `tempo_confidence` and
`chroma` absent. A real implementation must not invent a BPM from a steady tone or
a short prefix, must not reinterpret cached vectors silently, and must remain a
descriptive signal rather than a calibrated genre/mood/beat product.

## Decision

`vela-pcm-spectral-2` measures a normalized half-wave-rectified spectral-flux
envelope (the first window is ignored as a start transient), detects local peaks at
least twice the envelope mean with a refractory gap, filters inter-onset intervals
to 60–200 BPM, and uses the median interval for BPM. Confidence is the
onset-interval regularity (`1 − MAD/median`, scaled by support), bounded to 0..1.
At least 64 analysis windows, 4 onsets and 3 valid intervals are required; flat,
too-short or irregular envelopes return `None`. Chroma was added later in spectral-3/ADR0020; spectral-2 caches are retained unchanged.

## Consequences

Tempo is honest on periodic material but is neither beat-tracked nor validated
against annotated music. Radio ranking does not use it. Synthetic tests cover an
8-second 120 BPM click train, steady/silent/impulse material remaining `None`, and
finite/bounded descriptors. Real-music tempo accuracy and chroma require a later
measured milestone.
