# Approved visual specification

All dimensions below are logical CSS pixels at 100% app/browser scale unless a
platform-specific test says otherwise. They come from the user's approved brief and
published CSS, not newly measured rendered rectangles. Preserve the exact visual
direction; apply only the explicit accessibility/data corrections in 01_REFERENCE_AUDIT.

## 1. Visual identity and prohibited substitutions

Vela is a private personal music room: charcoal with green undertones, warm white,
quiet sage captions, pistachio actions, large honest artwork and compact editorial
type. A muted lilac/peach ribbon is local to My Wave. It is not a global purple theme.

Do not introduce Spotify trademarks/green, a black-only canvas, neon, blue action
colors, all-screen frosted glass, glowing borders, giant rounded panels, a tile for
every paragraph, emoji icons, fake metrics, marketing dashboards or subscription ads.
Use rows, artwork, negative space and typography. Flat surfaces differ by tone rather
than repeated heavy shadows. A reference hero is a specific layered photograph, not
permission to add arbitrary gradient banners to every route.

## 2. Approved roles and palette

This is the visual source of truth. Use the collision-safe mapping in 04 before
applying these roles to Vela's existing CSS. Do not paste names over current roles.

| Role | Exact dark value |
| --- | --- |
| Background | #131513 |
| Foreground | #f3f3ed |
| Card | #1b1e1b |
| Popover | #1c201d |
| Primary/action | #d3e9a5 |
| Primary foreground | #1b2511 |
| Secondary surface | #272c27 |
| Muted surface | #232823 |
| Muted foreground | #9aa296 |
| Accent surface | #293025 |
| Destructive | #ef9290 |
| Border | #2a2f2a |
| Input border | #383f37 |
| Sidebar background | #0e100f |
| Quiet | #798372 |
| Desktop player / top border | #10130f /#343b2e |
| Search field | #1d211c |
| Inactive filter | #22271f |
| Active navigation fill / text | #d3e9a50c /#d3e9a5 |
| Track hover / current background | #ffffff04 /#d3e9a505 |
| Seek rail / filled portion | #343d2c /#bbcda8 |
| Mobile player / navigation | #1c2318 /#10140e |

Thin borders are 1px. Radius scale: 6,8,12,16,20px; individual reference elements
explicitly use 4/5/7/9/10px where stated. Keep those deliberate component values
rather than rounding every control to 20px. Dialog shadow:`0 20px 100px #0009`.
No persistent shadow on ordinary album cards. Colors with alpha are distinct state
roles, never a translucent replacement for the opaque primary button.

Required small text on raised surfaces uses a contrast-qualified text role; quiet
on card fails the calculated normal-text threshold. Check actual mixed/image colors.
Preserve current functional Light/System and save/recovery behavior. Light is a
separate validated adaptation; do not apply dark hex values to the light selector.

## 3. Typography and icon family

Self-host real Manrope variable WOFF2 with Cyrillic/Latin and required punctuation.
Fallback:`Arial, sans-serif`. Use browser antialiasing where supported. No font CDN
at runtime; no Inter substitution simply because it was the old default.

| Element | Reference type |
| --- | --- |
| Body | 14px |
| vela wordmark | 34px,750 weight, −1.8px tracking |
| Page h1 | clamp(26px,2.5vw,37px),650,1.35 line height, −1.25px |
| Section heading | 19px,600, −0.55px |
| Album title | 13px,650, −0.15px |
| Track title | 12px,550 |
| Secondary captions | 10–11px |
| Eyebrow | 8px,650, uppercase,1.8px letter spacing |
| Sidebar navigation | 12px |
| Sidebar group heading | 9px, uppercase,1.6px tracking |
| Player times | 9–10px, tabular numbers |

Keep the compact desktop composition. Do not bulk-replace all typography with 16px.
Allow actual app/text enlargement, useful wrapping, translated labels and larger
touch hit regions. Essential text cannot become decorative simply because it is small.
Content titles may truncate with an accessible full-name path; action labels remain
understandable and must not be reduced to ambiguous ellipses.

Use the existing original outline SVG family, close to Lucide: 18–19px navigation,
stroke 1.6–1.7, coherent caps/joins. Extend that family for missing symbols. Play/Pause
may be filled. Do not add a React icon runtime or mix arbitrary packs. Accessible
names belong to controls; purely decorative SVG is hidden from assistive technology.

## 4. Desktop shell and scrolling

Reference baseline: 1284×856, dark,100% app/browser scale. The application occupies
100dvh. Body is not the primary catalog scroll container.

```text
┌──────────────────┬─────────────────────────────────────────────┐
│ sidebar216       │ fixed topbar75                              │
│ own overflow     ├─────────────────────────────────────────────┤
│                  │ central content: the main vertical scroller │
│                  │ bounded max-width, artwork shelves and rows │
├──────────────────┴─────────────────────────────────────────────┤
│ persistent full-width player88                                 │
└────────────────────────────────────────────────────────────────┘
```

The outer shell offsets the main column exactly once. Do not combine an old app-shell
left padding with a new main-shell margin and another sidebar inset on content.
Main shell height is the available viewport minus the actual player reservation.
Flex children that own scrolling need `min-height:0`; grid/flex text children need
`min-width:0`. A max-width 1560 inner content wrapper is centered inside the main
column, never across the entire window including the sidebar.

| Region | Desktop baseline geometry |
| --- | --- |
| Sidebar | 216px wide; fixed from top to player; padding 29px 17px 0; own vertical overflow |
| Topbar | at least 75px high; horizontal padding 33px; weak #ffffff05 bottom border; stays while content scrolls |
| Content | max-width 1560px; centered in main column; padding 24px 33px 0 |
| Central scrollbar | thin #3b4335 on transparent track |
| Player | fixed bottom across entire window; 88px; padding 10px 26px |

No visible horizontal body overflow. Long sidebars must scroll without covering the
player; hidden scrollbar styling cannot remove wheel/keyboard scrolling. Avoid a
hard 400px app min-height that clips controls under an onscreen keyboard/short viewport.
Signed-out/setup/auth remain usable outside the authenticated listening shell.

## 5. Sidebar composition

Top: existing Vela mark plus `vela.`. The approved reference mark is a small two-sail
symbol with a base line around 29×32; reuse the existing owned logo where available
and document any approved asset difference rather than inventing a new logo family.
Warm white wordmark, pistachio mark/dot. Top brand is a real Home action.

Primary destinations, in order: Главная/House, Поиск/Search, Радио/Radio. Radio must
open a usable real seed selection when no current seed exists, not an empty route.
Rows: min-height 41px,13px icon gap,14px horizontal padding,7px radius. Active row uses
the exact low-alpha primary fill, primary icon/text and a 5px dot at the right. Hover
is quiet and does not override active/focus. Avoid color-only active state.

Next: МОЯ МЕДИАТЕКА with a real Add Music plus. All tracks, Albums, Artists,
Favorites with actual count. Divider, then ПЛЕЙЛИСТЫ with Create plus. The reference's
three named playlists are examples; display actual eligible playlists and counts.
Playlist row cover 34×34, radius 5, gap 11; title 11px and count 9px. Ellipsis is permitted
for long names with accessible full name. Missing/mixed art uses the real permitted
mosaic/fallback policy, never arbitrary atlas art presented as the user's cover.

Bottom area: Recently listened, optional quiet ownership/about block, then Settings
and an external repository link. The info block contains the approved lines from 06
and ShieldCheck, and is hidden at desktop height≤900px. Hiding it must not hide
Settings or require overflowing navigation. Other Vela destinations stay reachable
through a compact More/destination surface; user/admin authorization remains intact.

The GitHub/Code icon is an actual reviewed external link, not a fake button. Public
links must not include source/server tokens or leak private current-track URLs.

## 6. Topbar

Left: compact Back/Forward, with actual navigation availability; disabled opacity
approximately 0.25. They are app navigation, not queue Previous/Next or a substitute
for browser history that exits to a previous origin unexpectedly.

Search: 370×36 at baseline, radius 7, padding 0 13, gap 10; Search icon 16; text 11px;
placeholder “Найти то, что звучит внутри”; compact platform shortcut hint at the
right. Ctrl+K on conventional desktop/Linux/Windows, ⌘K on macOS. Focus-within has a
thin pistachio ring; the control also needs an accessible name independent of placeholder.

Right: actual Add Music/upload action,1×20 divider,31px circular avatar. Avatar
background #32392b, text #d0d9be, quiet border. Show the actual user's initial/avatar;
V is a fallback, not every user's identity. No Демо badge or invented online/quality
dot on production. Connection error/state is a truthful separate projection.

Topbar query and Search page share state. Opening Search must not accidentally clear
the query, trigger two clients or remount the player.

Native integrated Linux window controls reserve their own real topbar space. They
are additional native chrome, not imported browser decorations. The search width is
a baseline preference that can shrink with min-width:0; do not let avatar/upload/
window buttons overlap it on640–900px windows. Preserve drag regions without making
search, sliders or buttons draggable. Other desktop systems keep their established
OS decoration policy unless separately validated.

## 7. Home header and filters

Eyebrow “МУЗЫКА. ТОЛЬКО ВАША.”; h1 “В своём ритме” with a pistachio dot. On the right
a ShieldCheck and “Личное пространство” when it accurately describes this scope.
Keep the dot decorative. Use one h1, not a second introduction announcing the same page.

Filters: Для вас/Альбомы/Плейлисты; min-height 30, padding 0 14, radius 5, gap 7. Active
primary fill/dark text; inactive #22271f. This is a filter group or tabs according to
its actual implementation, with keyboard state and an accessible name. Opposite
caption: “Знакомое. Любимое. Неожиданное.” Secondary copy hides at mobile widths.

| Selected view | Real content composition |
| --- | --- |
| Для вас | Editorial/wave pair, real album shelf and recently added rows |
| Альбомы | No features; “Ваши альбомы”, the approved subtitle and real album shelf; recent rows retained |
| Плейлисты | No features/albums; “Для каждого настроения”, Create and real playlist grid |

Do not download the whole library to implement these filters. Do not show a no-results
state while a real request is still pending or mix old-account content into a new view.

## 8. Editorial feature and My Wave

Baseline grid 1.95fr 1fr, gap 17; both 250px high, radius 10, overflow hidden. Each card
has a defined stacking context. Images sit behind the shading and text; interactive
controls stay above, without a negative layer disappearing behind another ancestor.

Editorial paint order:

1. Original coast.png, cover, position60% 53%, reserved complete card bounds.
2. Horizontal dark gradient from #071e1be8 through #071e1b6a at 48% to transparent 85%.
3. Bottom gradient from #081a17af to transparent by 40% from the bottom.
4. Copy and controls with 24px 29px padding.

Eyebrow with small primary dot; title in two semantic lines, clamp(33px,3.5vw,47px),
550,1.04, −1.7px; subtitle 11px/1.7, #c0cfc2. Listen button about 33px high, radius 5,
primary/filled Play. A30px circular outline arrow opens the actual collection.
Upper-right carousel controls: three 4px dots; active 14px short bar. Each is a named
button with selected state. No autoplay carousel, timer or hidden gesture requirement.
Three approved stories/copy are in 06. Production stories require actual target refs.

My Wave uses original your-wave.png, cover, center 32%; bottom shade
`linear-gradient(0deg,#241d2deb,#241d2d10 90%)`. Top outline AudioLines/uppercase
ПЕРСОНАЛЬНЫЙ МИКС/open arrow. Lower-left title 29px 600, −1.1px; subtitle and quiet
outlined “Настроена на вас” only when true. Lower-right 38px Play, #ede1ef/#332739;
fine-pointer hover scale 1.08 without glow. The button dispatches a real mix preview/
context; it cannot pretend to play before eligible candidates resolve.

Source artwork does not establish personalized content. Empty/small catalog fallbacks
retain the overall rhythm with useful selection/import actions and fewer real rows.
Do not auto-create three sample playlists or mislabel random library order as a mix.

## 9. Album shelves and recent tracks

Album section margin-top 25; heading “Снова в наушниках”, subtitle and “Все альбомы →”.
At baseline six equal columns, gap 17. Each card is primarily art, not a raised square
container. Square cover, radius 7; title margin-top 12,13px 650; secondary artist
margin-top 5,10–11px; “· Альбом” only when space permits.

Artwork enlarges inside its clipping bounds to 1.035 over 350ms on eligible hover.
Floating Play 35px at lower-right: 150ms opacity/short translate, primary fill, small
shadow; visible on focus and while that exact context is active. Save heart 28px at
upper-right on a subtle dark surface. It requires real album-save semantics; a
representative-track favorite is not an album save. Mobile Play is always visible;
the hover heart is omitted there, with equivalent accessible action elsewhere.

Use the real authorized cover first, then the owned neutral fallback. Album name,
artist, duration and grouping come from actual DTOs. Do not split one song into a
fake album, merge editions by text or use concept atlas covers as catalog data.

Recent section: “Недавно в коллекции”, approved subtitle and “Вся медиатека →”; show
up to five actual newly added rows, with a full-library path. Sparse libraries show
their actual count. Independent sections settle without moving a focused control.

Footer inside central scroll: thin top border, vela., “Ваша музыка. Без лишнего шума.”,
and About link. It must not add another fixed bottom region competing with the player.

## 10. Universal track list

Desktop columns:
`25px minmax(0,1.9fr) minmax(0,1fr) 36px 55px 36px`;
gap 10, padding 9px 12px. Order: index/transport; art+title+artist; album; favorite; duration;
queue/playlist/More action according to context. Header: #, Название, Альбом, clock;
10px muted, bottom border. Keep header/row/skeleton column geometry identical.

Rows: min-height 63, radius 6; cover 37×37, radius 4, gap 12; title 12px 550, artist 10px with 5px
gap; tabular time. Index formatted 01,02,03 but not treated as durable identity.
Hover uses #ffffff04; current #d3e9a505 with primary title and an independent current
label/icon. Current playing marker: four thin equalizer bars; pause stops them;
reduced motion uses a static useful marker. A pending source shows pending intent,
not a playing equalizer or a whole-table skeleton.

Title/index activation uses the displayed list's real context, with current-occurrence
toggle semantics. Separate artist/album links navigate. Child actions must not bubble
into a second source start. A plus icon's accessible label states whether it means
Add to queue or Add to playlist; do not use an ambiguous context-dependent mystery.
More/PKM reuse the contextual command model, with no hidden playback side effect.

## 11. Persistent desktop player

Player 88px, full-window width, #10130f,1px #343b2e top border, padding 10px 26px.
Three unboxed zones; allow bounded responsive shrink without losing transport.

| Zone | Composition |
| --- | --- |
| Left≈28% | 51px cover/radius 6, title 12px semibold, artist 10px, real favorite; cover/title opens expanded player |
| Center≈38%, max 470 | Shuffle/Previous/Play-Pause/Next/Repeat, gap 17; 32px main circle #e4ebd9, dark 14px icon; regular controls≈27px |
| Center lower row | Seek with elapsed/total time, gap 9 from transport; rail 3px, thumb 8px visible on hover/focus |
| Right≈28% | Actual source/quality disclosure, Queue, Mute,65px volume, Expand |

Visual small controls have sufficient separate hit boxes. Shuffle/Repeat use primary
and a small active dot; Repeat cycles Off→All→One→Off. Actual unsupported native
modes must remain explicitly unavailable rather than faking highlighted state.

Do not display DEMO/LOSSLESS/bit-perfect unless the actual source/adapter justifies
the information. Original format and output device format are different facts.
Pause intent remains actionable while source preparation is pending. Spinner/status
reflect actual loading; source errors expose recovery and never masquerade as play.

## 12. Secondary routes

Common header: eyebrow “ВАШЕ МУЗЫКАЛЬНОЕ ПРОСТРАНСТВО”, one actual page title plus
decorative dot, secondary caption and a meaningful Add/Create action when relevant.
Routes retain the persistent shell and root player, not an embedded second player.

| Route | Target visual/behavior composition |
| --- | --- |
| Library | Медиатека; actual genre chips, sort select, Listen all with explicit loaded/query scope, real count and universal list |
| Search | Поиск; large field 17px, padding 15px 20px, radius 10, thin primary border, clear X, actual categorized results and honest empty states |
| Albums | Four-column grid, gap 30px vertical 24px horizontal; shared album card and cursor continuation |
| Artists | Four-column grid, gap 35px 25px; circle image, name centered, Исполнитель ↗; portrait provenance/fallback correct |
| Favorites | Любимые треки; actual list, unfavorite reconciles current view without disturbing audio |
| Playlists | Three-column grid, gap 25; square cover/radius 8, title 15 semibold, actual count/visibility; real dashed New story/Create action |
| Playlist detail | 160px hero cover, padding 27, gap 28, radius 12, quiet green wash, border; title 29px; real visibility/count/Listen; entries and actual add flow |
| Album detail | Reuse real album route; cover/title/artist/year/duration, Play/Shuffle, disc/track ordering; no new fake landing screen |
| Artist detail | Actual name/counts/available albums/tracks, radio/seed controls and representative-art disclosure where needed |
| Radio | Следуйте за любимым звуком; actual seed/catalog/radio flow, explanations and sparse fallback; no fictional ML capability |
| History | Real qualified listening list and actual consent/retention controls; separate disabled/empty/erased states |
| Imports/scanner/jobs | Existing complete capabilities in the same visual roles; listener Add Music stays simple, admin diagnostics stay separate |
| Network | Preserve origin labels/live-versus-cached state and media permissions; do not hide behind static “private” marketing |
| Settings | Searchable functional device/account sections; actual scale/theme/privacy/offline/server controls, not the prototype's static values |

Every page has pending, empty, error, unauthorized, partial and stale-context behavior.
Generic empty composition: min-height≈270, outline icon 38, title 20, body 12/1.8, max 390,
one real next action where useful. No enormous dashed card if ordinary whitespace
and the real action are clearer. Specific Russian examples are in 06.

## 13. Dialogs, Queue and expanded player

General dialog: width`min(480px,100vw−32px)`, padding 28, radius 16, shadow as above,
title 20px 650/−0.5px, description 11px/1.7. Dim overlay #050a07b3 with 8px blur where
supported; safe no-blur fallback. Close at upper-right, Escape, correct focus capture/
return and bounded internal scroll. No fixed-height content that hides Close/Submit.

Queue is a right modal drawer, not a little center box: 370px, max`100vw−16px`,100dvh,
radius 15px 0 0 15px; own list scroll; header/Close/footer stay reachable. Current art 57,
primary title, artist and actual state. Divider, Next heading/count,38px covers in rows,
title/artist/time and exact-occurrence Remove. Footer Choose more music. Empty copy
in 06. Show actual schedule, not index+1 under shuffle. Closing leaves audio unchanged.

Expanded player: width≈430, max viewport−32; Now playing title/quiet subtitle, cover up
to 320px/radius 9, track 24px, artist/album/favorite, seek, larger transport with 50px main
Play, truthful source detail. On short/mobile viewports cap cover by available height
and scroll useful details; never push seek or Close beyond reach. It is a presentation
of the root engine. Menu/picker children use the actual active dialog/top layer.

Upload dialog: real Add Music flow, approved dropzone composition, formats from actual
pipeline, batch/progress/error/cancel where supported; not object-URL-only fake upload.
Create playlist: real named form, trim validation, permission/error/conflict feedback;
success updates sidebar/list after actual acknowledgment. Settings/About retain the
palette but use real app data and the existing functional Settings screen.

## 14. Responsive layout: explicit conditions

Apply widest-to-narrowest overrides in one reviewed cascade. Test the exact boundary
widths plus±1; do not let a narrower max-width rule overwrite the mobile recovery.

| Width | Shell and content | Music composition |
| --- | --- | --- |
| ≥1500 | sidebar 240; topbar/content horizontal 48; content top 35 | features 330px; editorial title 60; wave 39; album gap 26 |
| 1101–1499 | sidebar 216; horizontal 33; top 24; topbar 75/player 88 | features 250; grid 1.95:1, gap 17; six Home album columns |
| 901–1100 | sidebar 190; horizontal 25; compact transport, volume 44 | features 1.7:1, gap 14; five preview cards; hide·Album |
| 761–900 | sidebar 175; horizontal 20; hide privacy caption/secondary topbar/quality | features≈255; editorial 36, wave 25; four Home preview cards |
| ≤760 | no sidebar/left offset; topbar 63; content 25px 20px | vertical features; horizontal Home albums; mobile player/navigation |
| ≤360 | horizontal 16; hide search shortcut; mini-player heart may hide | editorial title 43; playlist hero stacks vertically |

Fewer Home preview columns must not make catalog items permanently inaccessible;
All albums opens the complete real paginated route. Mobile restores every preview
card into the shelf. Real libraries are not constrained to six entities.

Mobile details:

- Editorial 288px high, padding 25, title 48; carousel indicators lower-left. My Wave
  becomes 160px banner, title 25, Play 39, hide tuned label. Mobile image cropcenter 39%
  and horizontal dark wash are published refinements; preserve their composition.
- Home albums 140px, gap 15, horizontal scroll, scrollbar visually quiet, Play always
  visible. All six reference examples would be reachable; production uses real items.
- Track header hidden; columns`17px minmax(0,1fr) 30px 30px`, row 66px, cover 39px;
  album/time hide only visually, with full details reachable. Do not remove favorite/
  queue functionality because one compact column is hidden.
- Albums/artists/playlists use two columns. Playlist hero 100px art, padding 18, gap 16,
  title 20; ≤360 stacks. Header Add/Create becomes a named compact plus where needed.
- Mini-player 69px, padding 9px 18px,42px cover/radius 5, title 11/artist 10, favorite,
  Play/Pause, Queue. Desktop transport/volume moves to expanded player.
- Navigation 62px plus safe area, #10140e, five destinations: Главная, Поиск, Медиатека,
  Плейлисты, Ещё. Icon 20, label 9, gap 5; primary active state. More opens Settings with
  the rest of Vela's destinations preserved there.
- Reserve exactly 131px+safe-area at the bottom; content's final focused row must
  scroll entirely above both bands. Count the safe area once. The mini-player sits
  above navigation, not behind it. Virtual keyboard, landscape and text enlargement
  require actual visible-viewport checks, not a screenshot at one 390px size.

## 15. Motion, input and accessible polish

Color/surface transitions 150–160ms; cover zoom 350ms; play reveal 150ms; four equalizer
bars≈800ms with different offsets, only while genuinely playing and visible. Tooltip
delay≈300ms. No autoplay carousel, parallax, constant background drift or audio preload
on hover. Reduced motion removes travel/looping/decorative scale while retaining state.

Use 2px primary focus-visible with offset; no geometry jump. Hover must have focus/touch
equivalents and live behind fine-pointer/hover capability. Native sliders keep keyboard
values/labels. Navigation usesaria-current; actual toggle modes usearia-pressed;
repeat-one also has a readable mode name. Icon-only controls have real names.

Skip link “К музыке”. Space transport only outside controls/editors/dialog key handling.
Ctrl/⌘K focuses real search. IME composition never submits premature Enter. Browser
zoom remains available. Empty/error/loading status uses one useful live message, not
announcements for every animation bar. Source failures do not become success toasts.

Skeletons inherit the exact final row/art geometry and existing reduced-motion/state
guards. Known authorized rows survive same-query refresh. New account/query/origin
clears stale private content. A subtle loading trick must describe real pending work,
not simulate network activity on already ready cached music.
