# Review of delivered plans — 2026-10-20

This is an implementation review of the existing repository, starting at `77c4763`
on `review/plan-completion-2026-10-20`. It is not completion of the entire product
mandate. The approved charcoal/pistachio/Manrope design remains the reference.
No database migration, live-runtime replacement, media edit, publication or push
is part of this review.

## Better Design connection and evidence

The skill was installed and the configured server/key were present. The current
thread had cached `MCP startup failed: timed out awaiting tools/list after 30s`;
therefore its tool catalog omitted Better Design. A fresh, ordinary Codex client
initialized BetterDesign `0.46.0` and listed42 tools in2.76s. The documented
`config/mcpServer/reload` request through the existing local App Server restored
this same thread to `connected`,42 tools, no tools error. No configuration/key
change or agent restart was needed. A separate Python HTTP probe received a
Cloudflare signature rejection; it was not evidence that Codex credentials failed.

Actual MCP calls: `get-ui-principle`, `get-ux-principle`, `get-review-rules`,
`check-comprehension`, `inspect-spacing`. Home and the real playlist picker were
captured at390/1440/1728px against isolated Rust servers and generated fixture audio.
Queue captures and a coarse-pointer1024px regression extend that scope. The MCP's
initial capture-script response is not a measurement: only subsequent
`inspect-spacing` responses marked `completed` are measurements. Local artifacts
live under `target/checks/review-plans-2026-10-20/`.

Two focused reviewers inspected web accessibility/content and native behavior;
a third reviewer independently challenged the geometry flags. Accepted findings
included tiny carousel buttons, Home/Queue touch targets and notification dismiss.
Rejected flags have concrete exceptions: `.sr-only` deliberately clips accessible
labels; sidebar sections have different semantic gaps; Hide uses intentional auto
spacing; native modal inertness does not require an `inert` attribute. Fine-pointer
desktop controls do not automatically fail a touch-only rule. Tool scores are not
visual approval. The whole-screen results were capped, and are not a complete
accessibility certification or a Better Design dashboard `VALID` receipt.

## Repairs and acceptance boundaries

| ID | Defect in delivered implementation | Repair | Acceptance evidence |
| --- | --- | --- | --- |
| RV01/P1 | Clearing picker error also cleared busy, enabling duplicate Add commands | Separate error reset; synchronous busy guard | REVIEW01 holds a response after the actual commit and attempts a second click |
| RV02/P1 | An old picker response/focus task could close or alter a newer picker | Per-open generation, physically canceled baseline reads, generation-bound focus | REVIEW01 dismisses/reopens while the old committed response is held |
| RV03/P1 | Retrying an uncertain Add generated a new revision/key and could duplicate the track | Retain one `AddIntent` envelope; only a known pre-commit revision conflict starts another intent | REVIEW01 drops the committed response, retries the identical body, checks actual entry count |
| RV04/P1 | Last matching track ID could give Undo another collaborator's entry | Prove one-revision/single-entry ID delta; otherwise acknowledge success without an unsafe inverse | Four `playlist_command` tests; REVIEW01 introduces an extra real same-track mutation |
| RV05/P1 | Undo could double-submit or erase a newer Add receipt on late completion | Single flight, stable inverse mutation ID, receipt ownership, inline retry error | REVIEW02 holds real DELETE, adds another track, then drops/retries another committed DELETE |
| RV06/P1 | Receipt text intercepted an open track-menu action | Menus above ordinary notification layer | REVIEW01 opens another real menu with the previous receipt still visible |
| RV07/P1 | Saved candidate deck was labelled as guaranteed next playback, including past linear rows and undrawn shuffle | Shared pure `scheduled_next`; both clients use it, saved queue has an honest label/count | Five order tests and REVIEW03; existing shuffle/history tests remain applicable |
| RV08/P1 | Raising Queue volume produced sound while Mute stayed pressed and displayed zero | Use the same root unmute+volume action as the other player surfaces | REVIEW03 checks actual audio volume, both sliders and pressed state |
| RV09/P1 | Browser `play` event claimed Playing before source bytes were ready | Observe `playing`/`waiting`, retain requested intent, reject readiness-incompatible events | REVIEW03 holds an actual source and verifies Connecting, error and recovery inside the modal |
| RV10/P1 | Native full-original preparation had no visible progress/cancel despite the contract | Real received/total bytes, honest checking/indeterminate state, physical cancellation and generation invalidation, Play retry | Native `--test-preparation`: real partial source socket, late completion and retry assertions |
| RV11/P2 | Carousel controls were14×4px and several Home/Queue controls were too small for touch | Small indicator glyph inside a real44px button; mobile/coarse controls have nonoverlapping actual bounds | REVIEW03/REVIEW04 and rendered geometry; no invisible overlapping hit halos |
| RV12/P2 | Full-width `CatalogCover` stretched a compact Home playlist row into a giant square | Explicit56px square thumbnail in that component only | TR04 and REVIEW04 actual rendered rectangles |

Important trade-offs: uncertain writes keep their destination locked until the
same command is reconciled or the picker is dismissed; this prevents an accidental
second add. Undo is deliberately unavailable when the returned snapshot cannot
prove the inserted entry. Shuffle still chooses from its fair deck at advance
time; the preview no longer pretends to know that random choice. The native
preparation repair improves control/feedback, not its whole-file transport model.

## Remaining plan gaps, in delivery order

These are open work, not hidden success claims. Each slice must preserve the
account/origin/generation guards and add real acceptance evidence.

1. **P1 native playlist busy-state ownership.** `ui.rs` create/rename/delete/item
   completions can return on view mismatch before releasing `playlist_busy`.
   Give each write a mutation owner separate from its view receipt. Hold a real
   mutation, navigate away/back, confirm the old acknowledgment cannot change the
   new detail and that current controls recover without signing out. Preserve
   CAS/idempotency and reauthorize at the service boundary.
2. **P1 native catalog pagination.** `load_groups`/`open_group` discard response
   cursors. Store opaque cursors per account/query/group generation, expose real
   Load more, cancel replaced reads and append deduplicated stable IDs. Test more
   than50 albums/artists and more than50 tracks in one album; Play must state the
   loaded context honestly or resolve the complete bounded queue explicitly.
3. **P1 progressive native transport/cache.** The current source adapter still
   downloads and checks the original before decode, capped at250MiB. Implement
   seekable bounded range-backed storage and a versioned, origin/account/content
   scoped cache before promising immediate local playback, durable cache reuse
   or automatic look-ahead. Test cancellation, eviction leases, hash failures,
   server restarts and duplicate queue occurrences. Do not replace this with an
   unbounded in-memory download or a second audio authority.
4. **P1 device queue/progress synchronization and explicit handoff.** Add a
   consent-scoped device/session protocol with sequence/idempotency/expiry;
   acknowledge the destination before stopping the source device. Verify two
   independently authenticated real clients and reconnect, not two mocked states.
5. **P1 navigation history/topbar.** Complete approved Search/Back/Forward and
   route-state restoration (group identity, applied filters, scroll, focus).
   Current hash-only navigation does not restore that complete context. Abort
   obsolete reads; changing route must not replace the active source.
6. **P2 localization and content hierarchy.** Centralize visible strings and
   make the approved Russian copy consistent with settings; remove repeated
   visible page titles on Favorites/Albums/Artists, simplify Settings route-hub
   introduction. Do not hide real destinations merely to satisfy a generic
   three-action heuristic intended for a decision screen.
7. **P2 native navigation accessibility.** Add semantic current-route state and
   a non-color visual marker in compact navigation. Validate actual keyboard
   events and screen readers; DOM `.click()` and counting tabbable controls do
   not certify keyboard/screen-reader operation.
8. **P1 native mobile clients/offline.** Android Media3 service/audio focus and
   iOS AVFoundation/AVAudioSession remain distinct platform work. Build on the
   actual SDKs, use platform secure stores and test background/lock-screen/offline
   behavior on available devices. Linux/WebAssembly success is not mobile proof.
9. **P2 shared rooms and lawful public network.** Keep per-participant streaming
   rights separate from room membership and public metadata; add bounded signed
   events, replay defense and opt-in disclosure. Existing public peer fixtures
   are not full ActivityPub/inter-host interoperability or redistribution rights.
10. **P2 account portability, discovery and richer library.** Continue the
    roadmap's opt-in identity/account linking, smart playlists, scheduled mixes,
    lyrics, rich contributors/editions, accessible context menus across every
    collection, and offline journal/sync. No centralized account service or
    commercial global catalog is implied by a UI control.

## Verification and reproducibility

New deterministic browser IDs: REVIEW01–REVIEW04 in
`tests/browser/prototype-transfer.spec.mjs`. Existing named PLAY/RES/NET/AUTH/NP,
TR and G scenarios remain intact. Browser tests use isolated SQLite/media roots;
MCP captures contain fixture content only. Run with the pinned wasm-bindgen binary,
`CARGO_BUILD_JOBS=2`, then the existing serial Chromium/Firefox suites.

The initial workspace baseline was356 passed/0 failed/3 ignored. New unit, browser,
native and final lint evidence is indexed in PROGRESS/QA_EVIDENCE after integration.
Intermediate failures are preserved, including the original REVIEW01 red,
the discovered menu layering/readiness regressions and native QA investigations.

An unrelated concurrent diagnostic process created an untracked server `build.rs`
that invoked Cargo recursively in the same target directory. Its original bytes
were preserved in `concurrent-probe/server-build.rs.original` (SHA256 recorded in
`record.json`); only the identified deadlocked build process group was stopped.
The experiment was removed from Cargo autodetection, not incorporated as product
code. This is separate from the musical/runtime data and must not be packaged.

Frozen runtime remains `target/runtime/vela-0.12.6`, source `7fe1ca0`, original
data directory. User service was read-only checked active, PID185315, NRestarts0.
The desktop archive built from `4ceafdc` is historical and lacks later source
repairs. No new package launch, public deployment, hardware audio, physical touch,
screen-reader, full Wayland or non-Linux native approval is inferred.

References: [Codex MCP](https://developers.openai.com/codex/mcp/),
[App Server reload protocol](https://developers.openai.com/codex/app-server/),
[Better Design setup](https://github.com/marvkr/better-design#setup).
