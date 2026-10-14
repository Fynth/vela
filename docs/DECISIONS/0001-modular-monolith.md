# ADR0001 — Six cohesive Rust crates and a single-process SQLite store

Accepted2026-10-13. Keep domain contracts independent; combine closely related foundation services in core, isolate media ingestion/controlled imports, and keep server/web adapters at the edge. One process, four WAL connections and explicit migrations preserve lightweight self-hosting. Shared exact assets and private per-user track IDs avoid conflating ownership and physical bytes. No empty subsystem crates.

Use current verified Rust1.99.0/Axum0.8.9/Dioxus0.7.10/Lofty0.25.4; use established SQLx0.8.6, reqwest0.12.28 and Argon2 crate0.5.3 APIs for this slice with a pinned lockfile. Newer stable upstream versions were checked; updating these minor API generations is separate validated work, not an assertion they are unavailable. No migration to newer versions without equivalent recovery/security tests.

Tradeoffs: simple deployment and explicit interfaces; SQLite serialize-writer constraints, single-process directory ownership and album/artist string model limit this initial slice. Normalize richer catalog data incrementally and measure queries rather than introducing services. UI remains Dioxus rather than a disconnected JS prototype; narrow browser File/audio bridges use platform APIs.
