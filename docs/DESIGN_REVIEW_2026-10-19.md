# Vela contrast and listening-layout review — 2026-10-19

This is an implemented, source-tested UI increment requested with Better Design.
Baseline HEAD was `0170ec5` on the existing repository; the only pre-existing
untracked file was `prompt.txt`, left untouched. No schema, original media,
credentials, user library, frozen runtime, release archive or production service
was changed. No subagents or external publishing were used.

## What changed

| Area | Implemented change | Source |
| --- | --- | --- |
| Identity/contrast | Original mint becomes a decisive action color; near-black dark surfaces, readable secondary text, visible input borders, explicit disabled tokens; complete light palette | apps/web/style.css; apps/desktop/style.css |
| Native shell | One grid owns sidebar/content/player geometry; content scrolls independently; no duplicate sidebar inset or empty reserved status row | apps/desktop/style.css |
| Native library | Readable titles/metadata; explicit176px action column; duration and four actions no longer overlap | apps/desktop/style.css |
| Native playlists | Concise title, honest edit/shared state, collapsed native New folder/New playlist disclosures, useful empty state | apps/desktop/src/ui.rs |
| Notices | Native dismiss action; fixed success notices do not shift the collection | native ui.rs and both stylesheets |
| Web listening | Actual library starts below a concise page title and existing Play action; large decorative library banner removed; artwork shelves have restrained framing | apps/web/src/main.rs; apps/web/style.css |
| Navigation | Readable active states; entire desktop sidebar scrolls without overlapping groups; phone mini-player/tabs retain safe space | apps/web/style.css |
| Controls |44px primary transport, sliders, track actions and compact native queue controls; readable input text; atomic theme color changes | both stylesheets |
| Evidence | Independent browser design fixture; actual native DOM layouts, requested/actual widths and PNG-dimension validation | tests/browser/design.spec.mjs; native-ui-qa.rs/js |

No audio adapter or server contract was replaced. Whole-original native preparation,
the reusable complete-file cache and native menu removal already existed at baseline;
they are not achievements of this design change.

## Design and Better Design process

Preserved Vela's existing theme/components. `find-design-system` returned audio/design
references including Velour and Sonic; none was installed. The existing light/dark
tokens were extracted for review. Color/high-contrast and hierarchy UI principles,
navigation/keyboard UX principles, review rules, feature mapping, comprehension and
rendered-DOM spacing tools were used. `.better-design/feature-map.json` maps actual
Rust/CSS flows; `.better-design/review.json` contains a concise review record.

Library header/search, playlist overview and web library action copy each passed
the text-only comprehension check. That check does not inspect screenshots or
establish usability/accessibility certification. Final visual inspection uses actual
fixture captures, not generated mockups or a fabricated commercial catalog.

Spacing measurement excludes decorative SVG primitives and intentionally clipped
screen-reader-only labels. Native measurement is restricted to `.native-app` to
exclude Dioxus's off-canvas development chrome. Real controls are retained. The
native desktop minimum fixture width is640px; the spacing tool's missing390px
native width is recorded as unsupported scope, while the web UI is tested at390px
and320px. Moderate inherited spacing measurements remain recorded.

## Repeatable commands

Run one Cargo process at a time, with `CARGO_BUILD_JOBS=2`. From the repository root:

```bash
cargo fmt --all -- --check
CARGO_BUILD_JOBS=2 cargo clippy --locked --workspace --all-targets -- -D warnings
CARGO_BUILD_JOBS=2 cargo clippy --locked -p vela-web --target wasm32-unknown-unknown -- -D warnings
CARGO_BUILD_JOBS=2 cargo test --locked --workspace -- --test-threads=2
python3 -m unittest discover -s tests -p 'test_*.py'
node --check tests/browser/design.spec.mjs
node --check apps/desktop/examples/native-ui-qa.js
VELA_WEB_PROFILE=dev VELA_WASM_BINDGEN="$PWD/target/tools/wasm-bindgen-0.2.129-x86_64-unknown-linux-musl/wasm-bindgen" bash scripts/build-web.sh
CARGO_BUILD_JOBS=2 cargo build --locked -p vela-desktop
CARGO_BUILD_JOBS=2 cargo build --locked -p vela-desktop --example native-ui-qa --features native-qa
python3 scripts/verify_native_client.py --test-peer --evidence target/checks/design-contrast-native-final-evidence
VELA_DESIGN_EVIDENCE="$PWD/target/checks/design-contrast-chromium-final/matched" npm --prefix tests/browser test -- --reporter=line
```

The optional `VELA_DESIGN_CAPTURE_SCRIPT` points at the first-party DOM capture
script returned by Better Design, saved locally as
`target/checks/design-contrast-capture.js`. It is not required to run DESIGN01.
Browser tests own temporary databases, sessions and legitimate generated PCM plus
committed FLAC/MP3 fixtures; production data is never reset or seeded.

## Verification record

- Final complete browser suites: Chromium42 passed/1 live-import skip in2.3m;
  Firefox42 passed/1 live-import skip in2.6m. Logs:
  `target/checks/design-contrast-{chromium,firefox}-final/run.log`.
- The final DESIGN01 measurement was strengthened to inspect every visible match
  of the selected text/control selectors, including both current and ordinary
  track rows. Separate reruns passed in Chromium5.8s and Firefox7.2s. Each engine
  measured38 pairs/theme across320/390/1440/1728px: dark minimum6.714:1, light
  minimum4.536:1. Logs/measurements/screenshots:
  `target/checks/design-contrast-all-rows-{chromium,firefox}/`.
- Native real-window QA:74 checks PASS,10 screenshots whose PNG dimensions match
  actual DOM viewports;640/1280/1440/1728px. Retained source-generation/navigation,
  authorized public-peer and genuine server-offline playback checks passed.
  `target/checks/design-contrast-native-final-evidence/report.json`.
- Better Design final spacing: web0 critical/0 serious/10 moderate; native0
  critical/0 serious/4 moderate. Native390px is outside the supported desktop
  fixture scope. The moderate findings are inherited off-scale measurements and
  small volume-group gap drift; they do not become a falsely perfect score.
- Final Rust291 passed/0 failed/3 ignored; strict workspace/WASM and explicit
  native-qa Clippy, fmt, dev builds,22 Python tooling tests, JS syntax and diff
  whitespace checks passed. `target/checks/design-contrast-source-manifest.json`
  fingerprints the tested explicit source paths and Cargo.lock. Physical devices,
  release packaging and Docker were not part of this design pass.

Final executed results and logs are recorded in PROGRESS and QA_EVIDENCE. During
review, actual DOM checks found and fixed undersized targets, overlapping track
actions, inconsistent search padding, excessive page insets and a sidebar clipping
regression. Intermediate failed runs are retained under ignored
`target/checks/design-contrast-*`; none is relabelled as passing.

The first added design scenario depended on sessions that the existing security
suite intentionally revokes and then reached the real per-IP login limit. It now
owns a separate temporary server/session; production rate limits and authentication
were unchanged. A duplicate Jobs heading introduced by shorter copy was corrected.
A native capture could silently crop an off-screen window even when DOM assertions
passed; the test now moves only its owned fixture window and validates PNG dimensions.

Selected rendered library/control text pairs must have contrast at least4.5:1.
This is a scoped regression measurement, not a full WCAG audit. Image text, all
hover/disabled states, assistive technology and every screen are outside that metric.

## Limits and next review

- The in-app browser runtime had no available browser; existing Playwright CLI
  and a real WebKitGTK native fixture were used. `review-ui-code` is not exposed
  by this Better Design installation; source review and executable tests substitute
  for that one unavailable tool, with no claim it ran.
- Native captures use X11/XWayland inside Hyprland, an explicitly floated owned QA
  window, and a null audio sink. This does not certify normal native-Wayland tiling,
  physical sound, background playback, mobile platforms, Windows or macOS.
- Source/debug builds are not an upgrade of the frozen running user installation.
  Existing release archives and the unrelated untracked file are preserved; no
  package-cleanliness guard was bypassed and no external release was published.
- Native authorized artwork, wider screen/state accessibility coverage, contextual
  action menus, right Now Playing panel, progressive preparation/prefetch and device
  handoff remain separate roadmap work. Do not call the whole product complete.
