# Vela engineering conventions

Build complete, private-by-default vertical slices. Preserve user data. Never report unexecuted checks as passing. Read docs/PROGRESS.md and docs/CONTRACTS.md before changes.

Rust edition 2024; modular monolith. Domain DTOs have no framework dependencies. Core owns database initialization, configuration, authentication, and shared application services. Media owns file validation, ingestion, artwork, and range selection. Imports owns controlled egress and durable download jobs. Scanner owns capability-based read-only folder traversal, snapshots and durable scan jobs. Server owns HTTP transport. Web owns Dioxus rendering and browser playback.

Use parameterized SQL, migrations, bounded tasks, streaming file I/O, blocking pools for CPU/file metadata work. No media paths or secrets in public DTOs. Session tokens are random and hashed at rest. Cookie writes require the per-session CSRF header. No public registration; administrator creates users. Libraries are owned and private. Playlist metadata access never grants media access.

The task center projects existing durable workers; it does not duplicate job state or invent completed operations. Keep history/action/activity queries owner-scoped in one read snapshot. Source URLs and raw worker diagnostics never enter aggregate task DTOs. Action flags are hints; each mutation reauthorizes independently.

Web App owns one persistent HTMLMediaElement and the account-scoped queue/cursor. Child pages call root actions. Keep playback.rs framework-free: bounded occurrence indices, fair shuffle deck/history, explicit next priority and typed transitions. Every structural queue edit must synchronously update the order model; tag/favorite edits must not reset it. Natural EOF and manual Next are separate commands. New async page requests require physical cancellation plus independent session/query/generation/lifetime checks. Never call transient shuffle traversal a server listening history or native engine validation.

Verified persistent local runtimes use a frozen binary/asset snapshot with the original data directory. target/ contains private backups, release archives and active runtime snapshots; do not run cargo clean or delete target wholesale. Preserve these checkpoints across builds and re-inspect the live process/data before upgrades.

Keep Cargo.lock tracked. Build coordination: main agent owns workspace Cargo files, migrations and cross-module contracts. Parallel contributors edit only their assigned crate until integration. At most two Cargo compiler jobs on this host; avoid competing full builds. Run fmt, strict clippy, unit/API integration tests and wasm compilation before handoff. Browser, native and Docker validation must be identified separately.

Maintain docs/ARCHITECTURE.md, ROADMAP.md, PROGRESS.md, SECURITY.md, PERFORMANCE.md, KNOWN_LIMITATIONS.md and ADRs. Track unsupported functionality honestly; do not add empty crates. Never publish externally without explicit authorization.
