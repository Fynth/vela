# ADR0009: Evolve the listener shell around the existing player

Date:2026-10-16. Status:accepted.

## Decision

Preserve the seven-crate modular monolith, immutable user media, schema7 and existing auth/ingestion/sharing APIs. Add bounded owner-authorized local search to the existing media/core services and a typed transport module to the existing Dioxus client. Use original SVG assets and the established forest/cream palette. Home composes actual accessible additions/favorites/albums/playlists rather than imaginary history or a licensed global catalog.

Keep the root HTMLMediaElement and queue/cursor mounted across Home/Search/Library/group/playlist navigation. Child pages invoke explicit root Actions. Cancel obsolete Home/Search requests with browser AbortController and independently verify session, query, generation and mounted lifetime before applying results. Closed hash routes provide browser history without adding a framework dependency. Keep public-share fragments compatible. Mobile primary tabs use a focused More sheet for secondary destinations; service Jobs navigation is administrator-only.

Play next inserts after the current occurrence; Add to queue appends, with a1000-item bound. Pending play results carry element/source/request identity. Explicit pause/logout invalidates pending play errors; queue cursor changes persist as well as order. This is a bounded reliability increment, not a claim that fair shuffle, reconnect, exact resume or native background audio is complete.

## Compatibility and evidence

No migration/storage semantics or session credentials change. Before development, the clean0.4 checkpoint was compiled/tested and existing schema7 data (one user) was backed up and restored into a separate fixture. Two real TCP tests cover aggregate-search authorization/limits/invalid fields and shared metadata without audio.20 Chromium scenarios pass against actual native/WASM builds, including live licensed import, private races, decoded audio/navigation continuity, duplicate next/last order, old play rejection and eight responsive/theme screenshots.99 Rust tests, strict native/WASM Clippy,9 tooling tests and20 clean restarts with both worker-failure probes pass. See PROGRESS for exact logs/artifacts and platform limitations.

## Consequences

Search is local,8 results per category; group/playlist substring queries need profiling before a scale claim. Home has no fabricated recents/recommendations. The existing queue will next gain a tested playback transition model, fair shuffle history and explicit recovery. Multi-server, native engines, offline and federation remain distinct future slices.

Follow-up0.6.1: a real held-response browser regression reproduced playlist detail reappearing after navigation away/back under0.6.0. Centralize a PlaylistView context over exact session, route, object and generation. Apply the same contract to post-mutation refresh, share-link presentation and collaborator changes; preserve committed server actions and separately enforced media rights. New detail revisions may not move backwards. PLNAV01 covers actual navigation, deletion refresh and share response races.
