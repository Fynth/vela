# Private recommendations and track radio

The first implementation is deterministic track radio over the authenticated
user's owned library. It does not query a commercial catalog, remote peers, other
accounts' private listening, or a cloud model. Candidate IDs are actual owned
tracks joined to their media assets; playlist visibility grants no eligibility.

`crates/media/src/discovery.rs` reads the authorized seed and at most500 newest
other owned tracks in one SQLite snapshot. Output is limited to1–50 tracks. This
deliberately bounded baseline does not claim coverage of a million-track catalog;
an indexed similarity candidate stage needs separate measurements and acceptance.
There is no automatic scan, decoding or network fetch on a radio request.

The measured-content path consumes only completed, version-matching
`audio_analysis` cache rows. Valid finite descriptors compare log-frequency
centroid and rolloff, eight fixed-Hertz energy bands and digital RMS amplitude.
The normalized distance weights are0.25,0.20,0.40 and0.15 respectively. Frequency
terms span four octaves and the level term spans48dB. This is a transparent
descriptor distance; it is not calibrated perceptual similarity, loudness in
LUFS, a tempo/key model, or a genre/mood classifier. A bounded analyzed prefix can
describe that prefix, not every passage in a recording. Silence, missing fields,
corrupt JSON, incompatible versions and nonfinite/impossible values fall back to
metadata rather than inventing a spectrum.

Metadata affinity uses the actual complete artist and genre tags (case-insensitive
after trimming), plus known nearby release years. Genre strings are not split or
inferred. Audio score is75% measured similarity plus25% metadata affinity;
metadata-only fallback is40% metadata affinity. Candidate selection subtracts
repeat penalties for already selected artists and albums, then uses stable track
UUID ties. Sparse libraries return the available eligible subset, including an
honest empty result for a seed with no other owned track. Reasons name only real
evidence: measured spectrum/level, shared artist, identical whole genre tag,
nearby release year or another available library track.

The service has deterministic metric tests and real migrated-SQLite fixtures for
ordering, finite-value rejection, silent/missing/corrupt caches, owner isolation,
diversity, sparse libraries and the500-row bound. Execution evidence belongs in
`docs/PROGRESS.md`; source presence alone does not mark these checks passing.
Real decoded-audio fixtures are exercised separately by the analysis extractor.

Personal daily/weekly mixes, feedback suppression, consented listening-history
ranking, durable refresh, recommendation quality evaluations, remote rights-aware
candidates and native offline recommendations remain separate incomplete work.
Private history is never a public popularity metric or proof of media rights.
No recommendation or cached feature grants streaming, download or republication
permission; normal authenticated media authorization remains authoritative.
