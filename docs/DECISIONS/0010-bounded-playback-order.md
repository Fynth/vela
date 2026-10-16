# ADR0010: Pure occurrence-based playback order

Date:2026-10-16. Status:accepted.

## Decision

Add one framework-free playback.rs module inside the existing web crate; no new crate, dependency, server protocol, schema or audio decoder. PlaybackOrder owns only bounded occurrence indices, shuffle deck, actual traversal history, explicit next priorities, repeat mode and transitions. The Dioxus root PlaybackUi remains responsible for track metadata, account-scoped queue storage, the single HTMLMediaElement and applying Play/SeekStart/Stop/None. Future native adapters can reuse the pure ordering contract without pretending browser playback validates native engines.

Manual Next bypasses repeat-one. Natural EOF under repeat-one seeks/replays the current item; repeat-off exhausts/stops; repeat-all refills a full shuffle permutation. Exclude the last item of the previous cycle only from the next first draw, retaining it for later in that cycle. Previous after3s seeks without changing pause state; otherwise shuffled Previous/Next follow actual history. At the beginning with no previous item, seek the current one. History is bounded1000 and distinct from private listened-event history, which remains absent.

Play next uses a LIFO priority list even when shuffle is enabled. Inserting, swapping and removing queue entries remaps current/history/deck/priorities, preserving repeated copies of the same track as separate occurrences. Current removal is rejected. A new explicit playback context or selection resets traversal; metadata changes do not. Shuffle-mode changes reset traversal while retaining explicit next priorities. Reload restores at most1000 existing account-scoped tracks and cursor, starting a fresh traversal; shuffle/repeat reset across account lifecycle. Exact position, reconnect and cross-device journals remain separate work.

## Verification and limits

Eight native unit tests exercise2048 length/seed permutations, repeated-cycle coverage,10000 repeat-all steps/history bounds, previous/forward, repeat-one vs manual commands, empty/singleton cases, priority and occurrence remapping under edits. Real browser tests additionally exercise genuine decoder EOF, seek while paused, actual Next/Previous/Stop, shuffle traversal, priority across move/removal, duplicate rows and reload cursor. Final executed evidence is recorded in PROGRESS after integration.

The visible queue retains saved order during shuffle and explicitly says so; it is not a precomputed randomized next-items preview. Transient shuffle history and modes are not persisted across reload, and no gapless/crossfade/physical output guarantee follows from this model.
