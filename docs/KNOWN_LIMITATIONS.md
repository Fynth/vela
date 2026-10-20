# Unsupported and unverified functionality

## Interaction/window source repair2026-10-19

The newer [interaction review](UX_INTERACTION_REVIEW_2026-10-19.md) supersedes the
earlier NOT_RUN statement below only for the tested normal tiled Hyprland Wayland
subset: frameless window, reachable controls, row pause/resume, seek/source continuity
and no horizontal overflow. Full peers/offline and responsive native checks remain
separate X11 fixtures with a null sink. Physical sound, other compositors, screen
readers and non-Linux platforms remain unverified.

Web/native current-row transport, pending native intent and native per-entry group/
playlist playback are repaired. Context Play remains separate. Native public sources
still have no shuffle/repeat; offline supports sequential playback and repeat-one,
not shuffle/repeat-all. Controls/MPRIS reflect these limits. Public MPRIS metadata/
transport remains unsupported. Stable duplicate playlist-entry provenance during
concurrent reorder needs further work.

The frozen server and already running user desktop were not upgraded. Restart the
updated desktop binary to see the frame change. Progressive native playback,
next-track prefetch, durable native queue restore, artwork and handoff remain work.
Exact passing runs are in PROGRESS; this is not whole-product UX certification.

## Contrast UI verification2026-10-19

The newer Better Design implementation replaces the earlier muted current-source
palette and improves layout/control sizing; see [dated review](DESIGN_REVIEW_2026-10-19.md).
This is source/debug fixture verification, not a live frozen-runtime upgrade.
Measured text/control contrast and DOM geometry are scoped regressions, not a WCAG
or screen-reader certificate. Native artwork still uses neutral placeholders;
native audio remains complete-original preparation rather than progressive playback.
Native QA uses X11/XWayland under Hyprland with an owned floated window and null
sink; native-Wayland tiling, physical audio, iOS/Android/Windows/macOS and Docker
were not validated by this design pass. Device handoff/prefetch remain roadmap work.

Source audit2026-10-19: [outstanding findings](PLANS/2026-10-19/01_AUDIT.md) and
[ordered repair regressions](PLANS/2026-10-19/02_REPAIR_PLAN.md) include native
keyring/lifecycle/offline issues still awaiting fixes. Documentation reconciliation
2026-10-19 (docs-only): use [CURRENT_STATE.md](CURRENT_STATE.md) for the
source/deployed/design split. Historical statements below that conflicted with the
implemented native offline copies, opt-in Secret Service session and playlist folders
are corrected in place; older sections remain as dated history. Proposed fixes/design
are not completed features, and no code defect found by the audit is fixed by this
reconciliation.

Current0.9 adds explicit private account/device listening consent, bounded cumulative events/history/export/delete, optional durable local PCM/FFT analysis and bounded owner-only track radio. Exact executed checks are in PROGRESS. Collection is conservative and can undercount short/throttled/background playback; it cannot verify physical sound. Pending browser summaries are plaintext, capped100/128KiB, may drop the oldest at capacity and clear on logout/opt-out. History caps10000 retained sessions/account and32 device grants; Settings lists active grants and revokes one with privacy-revision CAS (deleting its sessions), while bulk device management is only the existing delete-all-and-disable path. Reads currently take a short SQLite writer lock to prune; contention is not benchmarked. DSP uses at most a120-second prefix by default, cooperative deadlines and no process memory sandbox; unsupported decoders fail safely. There is no per-job analysis cancel/pause UI; shutdown cooperatively cancels/drains and requeues interrupted work. The unified Jobs page still projects imports/scans only; analysis has its own real status/counts on Radio. Real decoder fixtures cover WAV/FLAC/MP3; Opus analysis is unsupported even where a browser plays it. Tempo is a bounded onset-regularity estimate, not a validated beat annotation; chroma is nearest-semitone pitch-class energy; key is an uncalibrated Krumhansl-profile heuristic (no tuning/chord transcription), and there is no genre/mood inference, LUFS/normalization or exhaustive million-track similarity index. Radio considers at most500 newest other owned tracks, with metadata fallback; personal daily/weekly mixes and history-based ranking remain absent. These additions do not establish native or federated functionality.

The0.5 listener increment adds a real Home, separate local Search, five-tab phone navigation with a keyboard-controlled More sheet, and next/last queue actions. This is not complete consumer-app parity. Home uses current additions/favorites/albums/playlists, not listening history or personalized mixes. Search returns at most8 per category without pagination; group/playlist matching is ASCII case-insensitive literal substring, track matching is FTS. No typo/transliteration/Unicode collation, search history, remote search or group-query latency guarantee exists. The sidebar shows at most6 playlist shortcuts. Version0.7 adds paused account-bound context/position/preferences restoration and bounded browser recovery; route changes retain active audio. Browser seek precision and autoplay policies still apply. Version0.6 adds bounded fair shuffle traversal/history, explicit-next priority and repeat/previous transitions; transient shuffle history rebuilds on reload, while version0.7 retains per-account shuffle/repeat preferences; this is not a durable device journal. During shuffle the visible queue retains saved order with an explicit explanation, not a randomized upcoming preview. General right-click menus, a persistent right Now Playing panel and a complete design-system component suite remain partial/absent. Version0.8 adds the full root-coordinated Now Playing modal.

Chromium checks cover320/768/1440/1920px dark/light and keyboard More dismissal/containment, not screen-reader certification or all browser/OS combinations. The in-app browser runtime reported no available browser; isolated Playwright Chromium was used against actual native/WASM builds. A later0.8 QA follow-up also ran all33 scenarios in Playwright Firefox157 on Linux; see PROGRESS for the precise scope. One requested parallel reviewer hit the environment usage limit without delivering code; implementation/integration continued in the main agent.

Version0.6.1 verifies playlist route/object/session generation guards, post-mutation revision monotonicity and share-link view isolation; this does not complete invitations/media grants or all legacy client workflows.

This is an early functional slice. The complete master roadmap is not delivered. Consult PROGRESS for executed checks; source/build recipes are not proof of platform runtime success.

- A Linux native desktop package exists with explicit private offline copies (max100/2GiB,0600, account-scoped), an offline Previous/Next queue and confirmed per-copy removal; saved originals play with the server stopped. Offline v2 manifests verify BLAKE3 before playback and a bounded startup sweep flags
corrupt copies; there is no content/entitlement sync, automatic download, bulk storage
manager, Android/iOS hardware validation, device transfer, remote control, PWA cache or background audio. Linux supports explicit opt-in Secret Service session persistence (opaque token only); Windows Credential Manager, macOS Keychain, Android Keystore and biometric unlock are absent.
- Explicit public publication and read-only public-catalog consumption exist (0.11 profile; see "Public catalog0.11 scope"). The native client saves canonical origins, searches them per-origin with cursors and browse-only cached fallback, and plays granted public streams; that remote path is verified only over loopback/private test CA, not Internet public-CA or two physical hosts. The web Network view is a single-origin first20 preview. There is still no ActivityPub-style federation, profiles/follows/activity feed/rooms, automatic peer discovery/WebFinger, HTTP signatures, merged global cursor, connected-server library adapter or OpenSubsonic routes. The live0.12.6 runtime has discovery disabled and0publications; no private content is public without an explicit per-track declaration.
- No direct-reference external libraries, WebDAV/S3/ZIP/Archive.org/feed adapters, cloud playlist metadata adapters or arbitrary commercial-service downloading. Direct HTTP/S audio, browser batch uploads and administrator-configured read-only folder snapshots are implemented ingestion paths. Browser preview knows filename/MIME/size, not pre-downloaded embedded album tags/artwork/license information.
- No FFmpeg transcoding, quality presets, ReplayGain, verified gapless/crossfade, playback-speed control or cross-device session-position synchronization. The web sleep timer pauses this tab’s player only; it is not an OS-level or cross-device timer. Uploaded uncommon containers/codecs may not play in the browser. No full-frame decode verification; later corrupt frames may survive metadata validation.
- Persisted album/artist groups and dedicated screens use trimmed names with SQLite ASCII NOCASE and album-artist/year keys (ADR0004). Unknown years remain provisional separate groups; Unicode case folding, verified editions/recordings, multiple encodings, genres/composer catalog, batch metadata editing and the remaining sort/filter dimensions (duration, size, last-played/play-count) remain planned. Genre/format/year/technical minimums and title/newest orders are implemented for Library/Favorites. Catalog pages50/max100 use UUID keyset cursors; new randomUUID additions may appear before a cursor. Album pages follow disc/track order but edits between pages can move tracks. Queue contains selected pages, not the whole catalog. Missing album tags remain available through Library rather than an invented album.
- Playlists are capped at1000 entries and200 per owner. Named viewer/editor grants and expiring metadata-only links exist; owners can CAS-rename or permanently delete a playlist (entries, collaborator grants and share links cascade; tracks/assets/streams remain) with inline confirmation. But there is no separate media sharing, invitations/acceptance, owner transfer, smart-rule model, activity/revision feed or SSE updates. Owner collaborator list/revocation and member-leave controls exist; stream access remains owner-only. Queue/playlist order uses accessible move-up and move-down controls and Clear-upcoming semantics, not drag/drop. A committed delete cannot be replayed back into existence and delivered bytes/licenses cannot be recalled.
- Exact BLAKE3 duplicates are handled; logical recording/master/live/remix duplicate grouping, acoustic fingerprints, enrichment/ambiguous review, smart playlists, personal recurring recommendations, lyrics, aggregated listening dashboards, general user export/collections/podcasts/internet radio are not implemented. Private listening export/totals and local track radio now exist separately as described above.
- Imports restart from zero rather than resuming byte offsets. Processing cannot cancel once atomic publication starts; automatic retries are limited to transient failures. Pause is unsupported. Conservative pre-download quota checks can reject a duplicate at a completely full budget. Progress uses active-job polling, not SSE.
- Jobs combines only actual durable imports and administrator-owned scans; browser uploads remain in the transfer dialog. No all-user admin queue, task pause, scheduler, SSE or scan retry-in-place exists. Scan again creates a new pass and source removal removes its history while retaining music. Idle pages need Refresh to discover work started elsewhere; hidden pages suspend network polling. Action flags are snapshots and mutations may reject a changed state/quota/source. State-filtered histories are live, so an explicit retry can move a row between filter groups while paging. No task metadata is an audio-access grant.
- Managed budget accounts referenced physical audio only; per-file limits bound staging/artwork, but global thumbnail/orphan accounting and reconciliation remain work. A hard crash between publication and DB commit can leave retained orphan content. No garbage collector deletes source files.
- Current database layout supports one application process and one local SQLite data directory. An OS lock enforces this. Network filesystems/cluster modes are unsupported. Fixed resource limits have not established safe user counts; benchmark scope is explicitly documented.
- Container runtime/build, ARM64, WebKit/Safari, stock-browser/OS combinations and OS system controls and actual mobile hardware must be validated separately; only checks in PROGRESS count. CI has been configured but is not remotely executed/published by this session.
- No independent security audit, backup encryption, encrypted local browser catalog or public production deployment. No software updates performed on external servers.

- **Dependency gate2026-10-19: FAIL for the advisories check.** `cargo-deny` rejected three INFO Unmaintained notices — `zvariant5.15.1` RUSTSEC-2026-0370, `zbus_names4.3.5` RUSTSEC-2026-0371 and `zvariant_derive5.15.1` RUSTSEC-2026-0372 — through `vela-desktop → mpris-server0.10.0 → zbus5.19.0`. Bans/licenses/sources passed. No ignore/exception was added for these three; they are maintenance notices, not published runtime vulnerabilities, and no exploitation is established. Evidence: `target/checks/audit-20261019/dependencies.log`; `deny.toml` keeps its earlier reasoned ignores (paste/GTK/fxhash), none of which covers these notices. Older "dependency policy passes" sentences and dated logs are historical and predate this advisory database. Generated license notices include packaged texts and hash-verified pinned upstream texts for most crates;15 packages currently have metadata-only notices (down from12; provenance in docs/third-party-license-texts). Full grants were not found at their inspected exact commits. External redistribution needs those notice gaps reviewed and a resolved dependency gate. No release was externally published.

- Catalog-only per-track metadata editing is implemented; original tag write-back and bulk edits are unavailable. The editor uses native HTML dialog behavior; non-Chromium accessibility/platform behavior still needs separate validation. Metadata revisions are local-server records; cross-server/offline metadata editing is not supported.

- Folder scanning currently requires Linux and operator-configured local storage roots (default data/sources). It copies audio into managed storage, using additional disk space; it does not stream directly from external directories. There is no filesystem watcher/scheduled scan, arbitrary path browser, source write-back or deletion synchronization. Symlinks/hardlinks/special files are skipped; symlinked root ancestors are rejected. Limits are32 configured sources,32 directory levels,1M visited entries and100k candidate files per pass by default. File signatures are best-effort; deliberate same-signature changes are not cryptographically revalidated. Kernel I/O on broken/network filesystems cannot be forcibly cancelled. Folder scans are administrator-only; ordinary users can upload/import instead.

- Library genre filtering compares complete trimmed tags with ASCII NOCASE; it does not split multi-genre strings or implement Unicode collation. Format labels can identify containers rather than verified codecs; numeric thresholds do not prove losslessness. Unknown years/audio properties do not match explicit bounds. Facets suggest at most100 genres/32 formats and report truncation; an exact genre can still be typed. Title/newest cursors are live: concurrent edits/additions can move page boundaries. Applied conditions persist in the current app session and clear on account changes; they are not an offline sync feature. Album/artist musical ordering is unchanged. First-page year filtering, empty combined matches and grouped facets showed higher SQL costs in the measured100k-owner workload; native/concurrent profiling remains necessary.

- Browser0.7 bookmarks store private catalog metadata locally in plaintext, bounded4MiB/1000 entries, not audio/credentials. Storage denial/quota/clearing can prevent persistence; it does not stop playback. Time updates checkpoint about every5s, with pause/seek/hidden/pagehide flushes; abrupt OS/process termination can lose the latest checkpoint. Restore is paused and fetches no media until explicit action. Legacy fallback is bounded and may lose position/preferences when a new snapshot is invalid. No offline playback or cross-tab/device conflict journal is provided.
- Reconnect uses three delays,10s HEAD deadlines and an8s progress watchdog; final failure needs explicit retry. Unsupported files can exhaust recovery without a precise codec diagnosis. Session revocation is detected on denied requests, not by erasing already buffered bytes. Autoplay restrictions can require another Play gesture after asynchronous metadata/recovery. Physical audio output, Safari, background suspension and native lock-screen services remain unverified.

- Now Playing0.8 is a native browser modal, full-height/scrollable on phones, with a five-entry saved-order preview. It is not an always-visible right sidebar, randomized shuffle prediction, lyrics view, file download grant or native engine. Source properties reflect metadata, not measured decoder output; no new latency/gapless/audio-quality certification follows. Dialog/native-focus behavior and Media Session callbacks are tested in isolated Chromium and Playwright Firefox157/Linux, not Safari, screen readers or physical headset/lock-screen hardware. Playwright used its Ubuntu24.04 fallback Firefox on this CachyOS host; this does not certify all stock Firefox builds/OS combinations. Very long text, enlarged fonts and short viewports can require vertical scrolling; controls remain reachable.

## On-demand personal mixes0.10 scope

Exploration caps at500 newest eligible nonhidden owned tracks and12 combined
favorite/recent-qualified anchors. It may return empty after anchor/played
suppression; it does not backfill with mislabeled repeats. Rotation shortlists
at most500 retained qualified tracks before diversity. Neither is a whole-
library similarity index, scheduled snapshot or calibrated preference model.
Explicit hidden-track choices cap at1000/account and are distinct from history:
history erasure does not reset them; Home has restore-all and UUID-only export.
Data-only updates may wait for one bounded10s feedback/export request; true
privacy/session changes invalidate immediately. Remote preference revisions
require refreshing privacy settings before another mismatched mix can display.
No public catalog, collaborative model, native client or offline entitlement is
implied by these server/browser features.

Personal candidate ordering uses added_at DESC/id ASC; the existing newest index
uses id DESC. Very large equal-timestamp batches may need extra sorting. The
current scale benchmark uses unique added timestamps and at most50 qualified
fixture sessions, so tied-time and10000-session latency remain unmeasured. Stable
repeated inputs return stable mixes; Refresh reloads current data rather than
pretending to regenerate an independently scheduled or random taste model.


## Public catalog0.11 scope

Public discovery is optional and off by default; no original account music is
published by upgrade. Only explicit per-track CC0/BY/BY-SA declarations are supported.
Evidence is owner-only and unverified; artwork is not public. Public tracks require
a positive known duration of at most7days; unusual years become unknown and public
text has bounded/control-free projections without changing owner tags. Stream/download grants
are separate API policy, not DRM or a guarantee recipients cannot save streamed
bytes. Revoking access cannot revoke received licenses or recall copies. Retained
revoked records count toward quotas; no purge/edit UI is implemented (revoke and
republish creates a new ID). Public metadata follows current owner catalog edits.

The Network view previews one HTTPS DNS origin's first20 published tracks with a
revalidated cursor "Load more" (server validates the cursor per page and the web client
dedupes by track ID); it does
not save peers, fan out search, follow users, mirror music, proxy audio or integrate
remote tracks into the root queue. Public links open the granted origin separately.
The native desktop Public view is separate: it saves origins, fans out search with
per-origin cursors/cache fallback and plays granted public streams through its own
engine; the web Network view remains one first20 preview.
No IP origins, custom HTTPS ports, signed inbox/activity, HTTP signatures, replay
protection, peer blocklist, WebFinger, remote cache/tombstones or Funkwhale compatibility
are implemented. Two-node tests use a production-absent exact test hostname pin;
Internet TLS/public deployments and standards interoperability remain unverified.
OS DNS resolution cannot be forcibly canceled, but occupied workers retain bounded
admission until actual completion. Public-catalog load/latency remains unmeasured.

Persistent user supervision starts with the existing Linux user manager, normally
at login; boot-before-login/logout behavior is unverified and no linger is enabled.
The installer is a same-build handover, not an updater; schema upgrades require a
verified backup and the separate upgrade process. Native clients/offline/device
handoff and Apple/Android hardware validation remain unfinished.

## Native Linux client0.12.x scope

**Updated2026-10-19:** this section previously described only the first0.12 slice;
the0.12.5/0.12.6 package adds explicit offline copies, opt-in Secret Service resume
and native playlist folders, documented below and in
[NATIVE_CLIENT_CONTRACTS.md](NATIVE_CLIENT_CONTRACTS.md). The audit's native
AUD-03/07/08/12 are fixed in source `e225b63` (off-thread cancellable prepare,
current-vs-other removal remap, per-await origin/epoch guards, quota/staging/
recoverable publication); concurrent two-process and disk-full/power-loss boundaries
plus explicit checking/Cancel harness assertions remain unexecuted. AUD-05 (cold
offline launch) remains open; AUD-06 (origin-scoped offline namespace) is fixed in
`a913574`, and queue snapshots canonicalize their origin in the adjacent fix.
AUD-01/02/04/11
are fixed in `3094754`, AUD-13 in `48b2fa4`.

Offline routing regressions observed during the repair are now fixed in source
`03b2523`: natural EOF advances the device queue instead of server prepare (Repeat-one
keeps replay), MPRIS Next/Previous use the offline-aware handlers, Play offline is no
longer restricted to Library/Favorites, and `cancel_all` cancels `folder_gate`.

The0.12.6 Linux x86_64 desktop application is a real Dioxus/WebKitGTK window with a
Rodio/CPAL output thread and MPRIS2 observer, verified against an isolated fixture
server/account/media. A portable x86_64 tar package has an exact-manifest/ldd/version check and a real
styled-window launch/close smoke. It is not an installer, AppImage/deb/rpm/Flatpak,
signed artifact, auto-updated client or release-ready desktop distribution. The window was
created only on this Hyprland/X11 host; GNOME/KDE/Wayland variants, screen readers,
physical audio output, headphone buttons and system sleep/resume are not certified.

Playback downloads the complete authorized original into a private temporary file
before decoding; there is no progressive streaming, gapless, crossfade, ReplayGain,
quality selection or measured time-to-audible claim. The250MiB/one-source bound and
preparation progress/cancel/retry are real, but large files incur preparation time
and temporary disk use. Explicit per-track offline copies **exist** in the0.12
Linux client, with v2 BLAKE3 manifests in0.12.6
(100 tracks/2GiB, account-scoped0600 audio, digests verified before
playback and by a bounded startup sweep, newest-saved offline Previous/Next queue,
confirmed per-copy removal; QA played with the fixture server stopped). There is
still no separate download-grant entitlement, background/automatic download or sync,
bulk storage manager, or verified cold launch/airplane-mode playback without a prior
in-process sign-in (AUD-05). By default sign-in is required each launch and
credentials/CSRF stay memory-only. An explicit opt-in **Remember this session**
stores only the opaque session token and canonical origin in the Linux freedesktop
Secret Service; password/username/CSRF are never stored, and resume still calls
`/api/me` (0.12.5+). Windows Credential Manager, macOS Keychain, Android Keystore and
biometrics are absent. Source commit `3094754` fixes AUD-01/02/04/11: sign-out stops
audio/clears private state first and deletes the keyring item off-thread with a
bounded deadline; transient/network/5xx/403 errors preserve the saved item (only a
genuine 401 rejection deletes it); secret-tool calls are byte/deadline bounded,
serialized and kill+reaped, and SavedSession Debug is redacted. The real two-process
Secret Service QA passed. The frozen live0.12.6 runtime predates this fix. Sign-out
clears the queue/catalog and stops native audio, but no cross-device session, device handoff or
remote-control protocol exists.

The native UI covers Library, Favorites, Albums, Artists, Playlists, Public and Queue,
transport, theme, sign-in and sign-out.
Favorites are implemented natively (filtered view, per-row favorite/unfavorite and
queue-state synchronization). Native playlists support real list/detail metadata, per-entry `can_stream`
labels, playback of the authorized subset, and owner create/CAS-rename/confirm-delete.
Albums and artists are browsable natively with read-only grouped detail and playback
of the owner-authorized tracks. Playlist entry add/reorder/removal is native; playlist folders are native since0.12.4/0.12.5
(owner-scoped create/filter/move/rename/delete with CAS), while album/artist
metadata editing, search history, settings/privacy, listening history and
recommendations remain browser-web features only. Native listening events
are not sent, so consent/history cannot be inferred from native preparation or
decoding. MPRIS requires a working session D-Bus and reports bounded public metadata
only; a failed bus degrades to in-app controls. Android Media3, iOS AVFoundation,
Windows SMTC and macOS Now Playing remain unimplemented/blocked on their real
toolchains, and Linux evidence cannot certify them.
