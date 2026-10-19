# Reference audit and conflict decisions

Date: 2026-10-19. Application source anchor: `4f16a32`. No Vela implementation changes.

## 1. Evidence boundaries

USER_SPEC means the user's detailed 16-section brief. SOURCE_OBSERVED means public
HTML/CSS/JavaScript or actual repository source was inspected. IMAGE_OBSERVED means
an original image was opened. DERIVED means a calculation from stated source values.
PROPOSED means a transfer decision. NOT_RUN means no executed acceptance evidence.

Do not relabel CSS declarations as viewport measurements. Breakpoints and dimensions
below are source declarations plus the approved brief, not measured browser rectangles.
The three images were visually inspected; they are not screenshots of the application.

Public retrieval returned 200 for the page, stylesheet, eight linked script chunks and
three PNGs. The browser runtime initialized but returned “No browser is available”;
read-only discovery then returned an empty browser list. No browser was selected,
no clicks were executed and no screenshots were captured. The standard web reader
failed for this URL; direct HTTPS and Better Design extraction provided source data.
No authentication restriction was encountered or bypassed.

## 2. Content-addressed reference

| Artifact | Inspected size | SHA256 |
| --- | --- | --- |
| Public index HTML | 75,277 bytes | 3e9a604df273197b67310c46556d6fd37e2405a2c3984558c348e625d7b5b619 |
| Linked 12owycp0mudf-.css | 77,750 bytes | 9d79666a361160ad0fc543a6841c7beba30451f28fdaebc9e7bacd756634af6a |
| App-containing 3s24beqvdrck6.js | 256,584 bytes | 10c601f15a3dfe43dd3cd463210d2647b4438b5d91b32e5e436c8bd3045041fb |

Sources: [prototype](https://prototyp-opal.vercel.app/),
[published stylesheet](https://prototyp-opal.vercel.app/_next/static/immutable/chunks/12owycp0mudf-.css),
[published app-containing bundle](https://prototyp-opal.vercel.app/_next/static/immutable/chunks/3s24beqvdrck6.js).
Asset-specific URLs/hashes are in [the asset manifest](06_ASSETS_AND_COPY.md).
These deployment paths may change. A changed hash is a new reference revision, not
permission to overwrite the approved plan silently. Never execute the bundle locally.

Temporary source inspection lives in `/tmp/vela-prototype-audit-l8ecsitz/` for this
session only. That path is not a portable dependency and may disappear. Source bytes
and font/image files were not copied into application assets by this task.

## 3. What the live publication actually establishes

| ID | Observation | Evidence and transfer consequence |
| --- | --- | --- |
| RA01 | Root HTML explicitly selects Russian and `.dark` | SOURCE_OBSERVED; dark is the approved measured-source palette, not proof of a polished light prototype |
| RA02 | Manrope variable faces cover 200–800 with Latin/Cyrillic subsets | SOURCE_OBSERVED in font-face rules; preserve real files, not a font-family label without assets |
| RA03 | 216px sidebar,75px topbar,88px player declarations match the brief | SOURCE_OBSERVED; actual visual rectangles still need browser/native measurement |
| RA04 | Home contains the approved editorial/wave pair and real referenced PNGs | SOURCE_OBSERVED + IMAGE_OBSERVED; transfer layered art/crop, not a flat green substitute |
| RA05 | Six concept album cells supply demo covers | IMAGE_OBSERVED; they are not evidence of owned commercial recordings or official album art |
| RA06 | Demo track records share one SoundHelix source | SOURCE_OBSERVED in public client; production must resolve each real authorized asset |
| RA07 | Playlist/favorite changes live in client memory | SOURCE_OBSERVED; no claim of durable collaboration/account sync follows |
| RA08 | A play event appends a unique track to prototype history | SOURCE_OBSERVED; do not port as qualified private listening history |
| RA09 | Queue add rejects an already-present track ID | SOURCE_OBSERVED; Vela occurrence-based duplicates must remain supported |
| RA10 | Queue view slices after the current track's ID match | SOURCE_OBSERVED; production needs authoritative next/shuffle/explicit-next semantics |
| RA11 | Album cards use a representative track and can search its album text | SOURCE_OBSERVED; production album IDs/pages and album-save semantics remain separate |
| RA12 | File import creates object URLs and reports session-only local use | SOURCE_OBSERVED; Vela server-upload/storage copy must describe actual server ownership |
| RA13 | Responsive CSS hides the sixth/fifth Home cards at intermediate widths | SOURCE_OBSERVED; finite preview rows can do this only with a clear full-catalog route |
| RA14 | Mobile CSS restores all cards to a horizontal strip | SOURCE_OBSERVED; no permanently hidden real catalog entries |
| RA15 | Reference app has a 400px minimum viewport container height | SOURCE_OBSERVED; do not copy it if it makes controls unreachable under a short native window/keyboard |
| RA16 | Actual current Vela has art, scale, native chrome and newer skeleton slices | SOURCE_OBSERVED; these foundations are preserved and restyled, not reimplemented |

No RA row is a click-test result. Published event handlers can contain bugs and do
not prove network, accessibility, focus or physical audio correctness.

## 4. Decisions resolving ambiguous or unsafe transfers

| Conflict | Required implementation decision |
| --- | --- |
| Extractor reports a translucent primary in `:root`, opaque primary in `.dark` | Keep the user's opaque action pair #d3e9a5/#1b2511; translucent active-navigation fill is a distinct role |
| Extractor says dual theme but HTML chooses dark and no separate approved light design is established | Keep current working Light/System; migrate its roles and validate it separately; do not pretend the prototype supplied it |
| Prototype `--sidebar` means color; existing Vela `--sidebar` means length | Use the namespaced role map and migrate each reader; no global paste collision |
| Prototype muted/accent surfaces collide with current text/action roles | Explicit alias migration, not a color-variable rename alone |
| Six demo records versus arbitrary real libraries | Render actual available cards; fewer tracks do not become fabricated albums |
| Editorial stories refer to fixed prototype playlists | Bind stories to real selected curated refs or present an honest selection/empty action; do not create music on upgrade |
| “My Wave” suggests personalization | Use the real accessible private mix/radio service with actual provenance; no fake daily refresh or invented mood |
| Sidebar favorites count and album hearts | Count actual saved entities; album save must not toggle one arbitrary track favorite |
| Library offers Artist sort but current TrackSort has Library/Title/Newest | Add a genuine indexed, cursor-safe server sort if accepted; otherwise omit the unsupported option and record the remaining gap |
| Portrait circles versus representative album artwork | Use real portraits only when present; label/decorate representative art honestly and keep neutral fallback |
| “Only for you” on shared playlists | Project real visibility/collaborator/media rights; never hard-code a private claim on a shared item |
| Reference More opens Settings; current More lists many product routes | More opens Settings with a real destination list for other authorized Vela routes |
| Prototype Settings is mostly informational | Keep functional app scale/theme/privacy/offline/server settings; restyle rows and group them, do not remove capability |
| Reference history is session-only; Vela has qualified private history | Show actual consent/retention, pause/erase/export; no opt-in or play-event counting on redesign |
| Full-height Queue drawer overlays the player | Treat the reference drawer as a modal; provide Close/Escape and needed local controls, keep the root engine alive underneath |
| Compact visual icons versus touch usability | Expand hit regions independently of icon size; use compact-desktop and touch layouts rather than overlapping invisible buttons |
| User asked for exact design, but literal transfer contains an accessibility failure | Make the smallest local correction and document it; preserve composition, palette family and hierarchy |

The last row is a quality constraint, not a license to propose another dashboard.
Any genuinely new feature remains behind its real implementation/capability boundary.

## 5. Derived color checks

Calculated from opaque sRGB values with linearized relative luminance and
`(lighter +0.05)/(darker +0.05)`. Values are rounded here for readability; acceptance
uses the unrounded ratio. No transparency, image overlay or browser anti-aliasing is
modeled by these numbers.

| Foreground | Background | Calculated ratio | Interpretation |
| --- | --- | --- | --- |
| #f3f3ed | #131513 | 16.476 | Strong body text pair |
| #9aa296 | #131513 | 6.976 | Secondary text on base |
| #798372 | #131513 | 4.634 | Quiet text on base, little margin |
| #798372 | #1b1e1b | 4.248 | Do not use for required small text on raised cards |
| #d3e9a5 | #1b2511 | 12.104 | Same ratio as the action label/fill pair |
| #bbcda8 | #343d2c | 6.709 | Fill against rail; not the complete slider audit |
| #343d2c | #10130f | 1.649 | Rail boundary requires actual non-text-control review |
| #9aa296 | #1b1e1b | 6.396 | Suitable replacement role for required raised small text |

Use the minimum necessary role correction, e.g. muted-foreground for essential
small copy on a card. Do not solve this by globally making every label white.
Normal-sized text generally needs 4.5:1; genuine large text has a different threshold.
See [W3C contrast guidance](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html).
For compact hit regions review actual spacing/size against
[W3C target-size guidance](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html);
the proposed 44px touch policy is Vela's design decision, not a quote of that minimum.

## 6. Planning validation receipt

Executed documentation checks PASS: nine Markdown files; 13 unique ordered TR package
definitions; 48 PT acceptance rows; 16 RA observation rows; 60 valid relative links across
the package and four updated indexes; 28 consistent tables within the package. Full
TR/PT/RA references and code-fence closure were checked. Nine source/image/font/license
SHA256 values match the inspected manifest. These are document checks, not UI tests.

Better Design extraction, UI/UX/reference-reading guidance and final review rules were
loaded. Final scoped copy checks PASS with zero findings: editorial card 12 words/two
actions, server-upload introduction 17 words/one action, disabled-history explanation 13
words/one action. These are text heuristics, not a whole-screen comprehension test,
user study or rendered design approval.

Inspect-spacing returned needs-capture and a capture script; no states were submitted
because no browser was connected. Therefore spacing/screenshot/click/focus/browser/
native/audio/hardware/application PT checks are NOT_RUN. No build, linter, application
unit/API/browser suite or benchmark was run for this Markdown-only task.

Preservation PASS: all 447 initially tracked non-Markdown files included in the source
fingerprint have identical before/after SHA256. The unrelated prompt.txt was excluded
from content reads and was not modified or staged. All tracked changes in this task
are Markdown: the nine handoff files plus PROGRESS, ROADMAP, DESIGN_SYSTEM and the dated
plan index. No app assets/dependencies/schema/media/data/runtime were changed; temporary
public-reference downloads are separate from production assets. No subagents,
external publication, push or deployment occurred.

Work branch: docs/prototype-transfer-2026-10-19. The source anchor remains 4f16a32;
the final local documentation commit adds no implementation evidence. A receiving
agent must record actual new execution in QA_EVIDENCE with source/artifact/platform
boundaries intact and complete the PT cases for the scope it ships.
