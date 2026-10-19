# Listening interaction and Hyprland review — 2026-10-19

Implementation baseline: `91867f2`. Existing repository, original mint design and
Rust/Dioxus audio owners retained. Initial tree contained only the unrelated
untracked `prompt.txt`; it was not read, staged or changed. No user data, schema,
dependencies, frozen server/runtime, release archives or compositor configuration
were changed. No subagents were used.

## Repaired interaction defects

| ID | Observed cause | Result | Main implementation |
| --- | --- | --- | --- |
| UX-01 | Native library selection always reset order and called start_source; web rows always rebuilt the queue | Current track row pauses/resumes the active source and holds seek position; another track still starts its displayed context | desktop/ui.rs play_catalog, play_owned_context; web/main.rs play_or_toggle_queue |
| UX-02 | Selecting the current queue occurrence reset the shuffle deck/history and restarted audio | Current occurrence toggles transport without structural queue edits; a different duplicate occurrence remains a new selection | select_queue/select_queued |
| UX-03 | Native preparing transport was disabled and load completion captured unconditional autoplay | Requested transport state is explicit; rapid second click pauses intent; completion loads according to the current intent instead of starting late | desktop/ui.rs wants_play, set_playing, start_source/start_remote |
| UX-04 | Row icons/accessible names always said Play, irrespective of transport | Library/Home/Search/group/history/radio/mix rows observe root transport state; native offline/public/playlist actions expose their real action | web BrowserPlayer View.requested and Actions; native row rendering |
| UX-05 | Native public source start required the Public page to remain open | Remote Next/Previous work from Library and preserve the remote queue/origin | desktop/ui.rs start_remote and native QA |
| UX-06 | Offline queue selection fell through to an online source | Another saved occurrence resolves the device copy; current saved occurrence pauses/resumes its verified descriptor | select_queue/start_offline_at/play_offline |
| UX-07 | Failed preparation could retain a requested-playing flag; failed remote selection was discarded | Failure clears play intent; retained authorized remote reference can be retried through the real source adapter | start_source/start_remote/set_playing |
| UX-08 | Clearing the queue left remote/offline selection projections behind | Clear stops playback and clears all active queue/source projections; saved offline files remain | clear_queue |
| UX-09 | Native album/artist rows were text despite whole-context playback support | Individual owned rows are playable and toggle the active track | browse_view/play_owned_context |
| UX-10 | Native playlist entries exposed edit controls but no per-entry playback | Streamable entry buttons map entry ID to its position in the permitted subset; metadata-only entries stay nonplayable | playlists_view/play_owned_context |
| UX-11 | Offline/public controls advertised ignored shuffle/repeat-all modes | Unsupported modes are disabled/explained; offline cycles off/one and retains local routing at EOF; MPRIS follows the same capability rules | playback_modes/set_shuffle/set_repeat/advance |
| WIN-01 | with_menu(None) removed the menu, but WindowBuilder still requested GTK decorations | Linux builder disables decorations; existing account toolbar contains window controls and a drag region, with no additional titlebar strip | desktop/window.rs; main.rs; ui.rs; shared QA builder |

Track identity and queue occurrence identity are separate. A unique catalog row can
control the already active owned recording across pages without replacing its queue.
For a duplicate in the same candidate context, ordered IDs and the exact cursor must
match before treating it as the current occurrence. Native public identity includes
canonical origin and object ID. There is no new permission inferred from a title,
playlist membership, cached metadata or currently displayed page.

Explicit Play album/playlist/radio/mix actions start that context. Row toggles do not
silently replace it. The implementation separates those callbacks; sharing a toggle
callback with Play radio initially retained the old one-track queue. The real DSP
browser regression caught this, and the explicit context-start callback fixed it.

## Reviewed product journeys and limits

| Journey | Actual review/verification boundary | Outstanding scope |
| --- | --- | --- |
| Setup/sign-in/sign-out/session replacement | Existing real HTTP/browser tests; native private fixture; stale-account state guards retained | Complete screen-reader/localization review |
| Library/search/favorites | Actual playback/seek, repeated row action, filtering and stale replies; source and position preservation | Native upload/import administration still relies on the web app |
| Albums/artists | Native per-row buttons and context start; existing web grouped playback tests | Native artwork and rich edition/credits pages |
| Playlists/folders/sharing | Existing API/CAS/sharing browser regressions and native editing/row playback fixture | Stable origin playlist-entry keys across every live reorder/duplicate edit; native sharing invitation flows |
| Queue/shuffle/repeat | Current-occurrence toggle; different duplicates remain distinct; existing PLAY/RES/NET/NP regressions | Native durable queue restore, randomized upcoming preview and device sync |
| Recovery and pending work | Current intent read at completion; stale-source/session cancellation remains; browser held-response and retry tests retained | Progressive native preparation, next-source prefetch and broader bandwidth simulation |
| Private history/discovery | Existing consent, radio, recommendation exclusion and privacy race regressions | Native private history collection and scheduled mixes are not added here |
| Public sources/offline | Origin-aware toggles, route-independent remote transport, saved-source dispatch and server-offline fixture | Full Wayland peer/offline/hardware coverage beyond the tested subset |
| Window/responsive interaction | Shared frameless builder; actual normal tiled native Wayland and separate X11 size fixtures; web320/390/1440/1728 themes | Other compositors/OS, physical media keys, touch hardware and full accessibility certification |
| Upload/import/jobs/settings | Existing actual server/browser regressions retained | No claim that all administration forms or unsupported native flows were redesigned |

Remaining work is explicit, rather than calling this a universal perfect UX. Highest
priorities after these repairs are authenticated native artwork, stable playlist
entry provenance during concurrent edits, native Add Music flows, durable native
queue restoration, accessible contextual menus, progressive preparation/prefetch,
and acknowledged cross-device handoff. These require complete independent slices.

## Window diagnosis and evidence

The inspected user surface was native Wayland (`xwayland:false`), normally tiled in
Hyprland. The titlebar was a GTK decoration, not an application content margin.
The shared builder uses Linux-only `with_decorations(false)`; non-Linux targets keep
their existing system frame and do not render duplicate custom window controls.
Minimize/maximize/close delegate to Dioxus/Tao, with keyboard-focusable44px buttons;
pointer drag/double click belongs only to the existing account region, not controls.
The compositor still owns tiling/borders/window policy. No Hyprland configuration
or unrelated window was modified.

Upstream API references: [Tao WindowBuilder decorations](https://docs.rs/tao/latest/tao/window/struct.WindowBuilder.html#method.with_decorations),
[Dioxus DesktopService](https://docs.rs/dioxus-desktop/latest/dioxus_desktop/struct.DesktopService.html).
The installed pinned crate implementation was inspected as well; no dependency
upgrade or custom window manager command was introduced in the shipping app.

Native Wayland QA launches a separate real GTK surface, verifies its owning PID and
`xwayland:false` in Hyprland, then captures only that surface with grim. PNG dimensions
must match DOM width/height multiplied by device pixel ratio. This host's measured
tiled viewport was1278×1030 at DPR2, producing2556×2060 PNG. It is not a fabricated
fixed-width screenshot or a claim of physical sound: audio uses an owned null sink.

Initial captures failed because Dioxus resets GDK_BACKEND to x11 after GTK startup,
although its existing surface is Wayland. The harness now carries its requested
backend in a separate test-only variable and independently verifies actual surface
protocol before capture. Earlier failed captures remain preserved. QA CLI rejects
unsupported backend/scenario combinations before allocating a server or sink.

## Reproduction and verification

Use one Cargo process at a time, max2 jobs. Build current dev WASM assets with the
existing pinned script and native binary/example, then run:

```bash
CARGO_BUILD_JOBS=2 cargo test --locked --workspace -- --test-threads=2
CARGO_BUILD_JOBS=2 cargo clippy --locked --workspace --all-targets -- -D warnings
CARGO_BUILD_JOBS=2 cargo clippy --locked -p vela-web --target wasm32-unknown-unknown -- -D warnings
CARGO_BUILD_JOBS=2 cargo clippy --locked -p vela-desktop --features native-qa --all-targets -- -D warnings
python3 -m unittest discover -s tests -p 'test_*.py'
npm --prefix tests/browser test -- --reporter=line
npm --prefix tests/browser test -- --browser=firefox --reporter=line
python3 scripts/verify_native_client.py --test-peer --evidence target/checks/ux-modes-native-x11
python3 scripts/verify_native_client.py --backend wayland --window-only --evidence target/checks/ux-modes-native-wayland
```

Use new evidence directories: the native driver refuses overwriting earlier runs.
UX01 owns an isolated real server/session and legitimate generated PCM. It seeks to
7s, pauses/resumes the library row, repeats that from Home and from the current queue
row, and checks actual HTMLMediaElement pause/time/source. NET02 now deliberately
adds and selects a different duplicate occurrence, rather than relying on a current
library row restarting to zero. Existing denied/stale response assertions remain.

The follow-up also removes false mode affordances: public shuffle/repeat are
unavailable; offline is sequential with repeat-one. Disabled buttons have source
explanations, MPRIS uses the same policy, and private online preferences remain.
Actual server-stopped EOF repeat is tested. Final native X11 scope is89 checks/10
captures; normal tiled Wayland is5 checks/1 capture. Final workspace tests293/0/3
ignored; browser43 pass/1 live-import skip per engine. Later native-only changes
do not alter the browser suite's source hashes.

Exact executed final counts, paths and limitations are in PROGRESS/QA_EVIDENCE and
`.better-design/ux-review.json`. Source/debug verification is separate from release
packaging or upgrading the frozen user service. An already running desktop process
continues to use its old in-memory executable/frame until restarted. The initial
user desktopPID115855 was not killed by this task; at final read-only inspection
it was no longer present. The service remained active0.12.6/PID1178/0restarts.
