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

## Personal exploration and rotation0.10

`crates/media/src/recommendations.rs` adds algorithm `vela-personal-1` as an
on-demand read model, not a scheduled daily/weekly snapshot. No new dependency,
network access, cloud inference or audio decoding is introduced. API contracts
are in CONTRACTS; source/command/platform evidence must be recorded in PROGRESS.

One SQLite read transaction first captures the current owner preference revision,
account history consent, retention and database clock. If disabled/absent, the
service does not query listening_sessions at all: retained-but-disabled history
cannot supply anchors, rotation or played suppression. When enabled only qualified
sessions started within retention and not in the future count; partial events and
repeated cumulative updates never become independent listens. No derived taste
cache survives an erase/opt-out. Device ingestion consent remains independently
validated by ListeningService.

Candidate generation examines at most500 newest positive-duration/positive-size
owned tracks, applying hidden choices BEFORE LIMIT. `candidate_count` is that
examined window before anchor/qualified-played suppression. Anchors alternate up
to12 latest owned favorites with12 recent qualified tracks, deduplicating to12
TOTAL. Hidden anchors are excluded. Discovery suppresses anchors and qualified
played IDs from the current retained history; an exhausted library returns an
honest empty set. With no anchors the result is diverse library exploration, not
inferred taste or an assertion that a track has never been heard anywhere.

Each candidate takes its strongest actual anchor's existing measured/tag radio
score (above); repeated sessions do not multiply similarity. Reasons add whether
that anchor was a favorite or a qualified private listen. No absent acoustic
field, tag, public statistic or remote right is invented. The same stable artist
and album diversity penalties apply. The audio flag is true only when a returned
discovery item actually uses measured compatible descriptors.

Rotation aggregates owned nonhidden qualified sessions across the whole retention
window, including old tracks outside the newest500. Its bounded shortlist caps
at500 with score `0.65*n/(n+3)+0.35*604800/(604800+age_seconds)` before diversity.
This saturates frequency and uses a7day recency scale, rather than awarding
unbounded preference to repeat loops. These are transparent engineering weights,
not calibrated human-quality metrics or learned Spotify-equivalent models.

Explicit hidden-track choices are separate owner preferences in migration0010,
at most1000 per account. Desired-state hide/unhide is idempotent; BEGIN IMMEDIATE
protects ownership/quota under concurrent requests. Exclusions change no files,
stream rights, favorites, playlists, radio eligibility or collection consent.
Home provides Hide, successful-operation Undo, confirmed restore-all and a private
UUID-only JSON export (`vela-private-recommendation-exclusions.json`). Erasing
listening history does not silently erase these explicit preferences.

Dioxus PersonalMixes is mounted inside Home after its hero, with one root player.
Read/action/render guards include session, component lifetime, request generation,
privacy epoch/revision, pending privacy changes and history/catalog freshness.
Actual HTTP controllers are aborted on supersession/unmount. Data-only freshness
changes defer until an in-flight feedback/export finishes; true privacy changes
invalidate immediately. Failed feedback leaves no stale actionable results and
supports explicit refresh. Every request has an exact captured10s deadline.

The standalone `benchmark_recommendations` example uses ONLY synthetic metadata
in an isolated temporary SQLite database, no media/user library. It can measure
query/rank latency, eligible coverage, output artist diversity and stable repeated
IDs at10k/100k/1m entries; those are structural measures, not listening satisfaction,
play-through, real audio quality, peer latency or production concurrency. Exact
executed measurements belong in PERFORMANCE, never inferred from code.

Still absent: scheduled/materialized daily/weekly mixes, calibrated preference
quality, explicit artist exclusions, collaborative learning, remote rights-aware
candidates, listening-context/skip models, native/offline recommendations and
large-library similarity indexing. Public popularity remains separate from private
self-reported listening. Stream authorization is rechecked independently.

Measured SQL follow-up: sparse favorites and small history aggregates previously
caused full owner-catalog scans. Selection now starts from owner favorites or
qualified retained history, explicitly seeks eligible tracks, and hydrates only
the bounded selected IDs in the same snapshot. IN hydration restores the exact
source order; rotation reuses its selected aggregate evidence. This changes no
weights, candidates, reasons or rights policy. Source-derived EXPLAIN and full
projection comparisons against the preserved baseline cover all four selectors;
actual release measurements are in PERFORMANCE. Existing mixed timestamp/UUID
order can still require sorting large equal-timestamp groups; that cost and high
retained-history cardinality are not established by unique-timestamp benchmarks.
