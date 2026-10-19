# Everyday product scenarios and completion criteria

Proposed user acceptance, not executed evidence. These scenario IDs supplement,
not renumber, the mandate's G01–G24 and existing named regression tests.

| ID | User journey | What a complete implementation demonstrates |
| --- | --- | --- |
| JOURNEY01 | Open native Vela with real music | Legitimate artwork/title/artist, useful Library, persistent player and Settings; no blank admin landing |
| JOURNEY02 | Make the desktop easier to read | Find scale by search, change125/150%, preserve playback, restart with saved size, recover100% |
| JOURNEY03 | Use a HiDPI/Hyprland desktop | Normally tiled correct frame/scroll/pointer/covers, independent monitor and app scaling, exact screenshot evidence |
| JOURNEY04 | Upload a tagged album | Native/web file picker or drag/drop, accurate per-file progress/dedup/art/discs, Play actual ingested tracks |
| JOURNEY05 | Add an authorized URL | Actual preview→job→metadata/art→library→play, useful cancel/retry and SSRF rejection |
| JOURNEY06 | Listen to a whole album | Ordered discs/tracks and edition, no page-navigation interruption or silent queue replacement |
| JOURNEY07 | Rapidly pause/resume/select music | Current row toggles source; delayed prepare obeys latest intent; different duplicate remains a separate occurrence |
| JOURNEY08 | Build an evening queue | Add next/last, move groups/duplicates, preview actual shuffle-next, search, save, Undo safely |
| JOURNEY09 | Come back tomorrow | Native/web paused context/queue/cursor/position restore per account+origin, no unauthorized media fetch on startup |
| JOURNEY10 | Replay recently heard music | Authorized warm cache hit reuses bytes, meaningful prepare stages, no extra full body without a real reason |
| JOURNEY11 | Let the queue continue smoothly | Actual next is prepared under quota/network limits; mutations cancel stale plan; measured Next behavior |
| JOURNEY12 | Take an album offline | Real aggregate/per-item download progress, cover/metadata, integrity, cold airplane-mode playback |
| JOURNEY13 | Free storage | Measured cache versus pins, safe reclaim preview, current source unaffected, original server files preserved |
| JOURNEY14 | Recover from interruption | Network/expired session/unsupported codec/disk-full failures have accurate causes and bounded recovery |
| JOURNEY15 | Find something in a large multilingual collection | Exact/normalized search, responsive suggestions/filters, correct origin, virtualized keyboard focus |
| JOURNEY16 | Repair an untidy library | Edition/duplicate/missing-art/tag review with provenance; no automatic original deletion or uncertain overwrite |
| JOURNEY17 | Make a smart playlist | Typed rules/count/limit preview, efficient evaluation, edit rules, static copy and portable export |
| JOURNEY18 | Collaborate without losing changes | Two real accounts edit stable entry IDs, stale revisions resolve visibly, revocation/media boundaries hold |
| JOURNEY19 | Discover music already accessible | Real cold-start choices, actual radio/mixes/reasons, reversible feedback and no fabricated global catalog |
| JOURNEY20 | Understand personal listening | Explicit account/device consent, accurate qualified totals, private export/delete, no unwanted collection |
| JOURNEY21 | Move playback between two devices | Explicit destination, actual prepare/ack/position, source stops only after accepted handoff, failure leaves A usable |
| JOURNEY22 | Listen with a friend | Expiring invitation, actual roles/participants/queue, rights-per-person and tested reconnect/drift |
| JOURNEY23 | Use two servers | Distinct credentials/IDs, partial search/origin labels, correct media resolver and honest offline peer state |
| JOURNEY24 | Use native Android | Background/lock/headset/call/offline on actual device, same queue semantics and artwork, no WebView-audio substitute |
| JOURNEY25 | Use native iOS | AVAudioSession/background/Control Center/offline/interruptions with actual Apple evidence |
| JOURNEY26 | Configure the app without editing files | Searchable implemented settings, scope/effective/locked status, safe reset/import; operator boundaries explained |
| JOURNEY27 | Use only keyboard or assistive technology | Complete focus/menus/sheets, text enlargement/reflow, useful announcements and reduced motion |
| JOURNEY28 | Change devices/accounts privately | No stale art/history/queue from previous identity, secure credentials and explicit local retention policy |
| JOURNEY29 | Upgrade or restore safely | Exact tested package, isolated restore verification, old library/playlist/media IDs intact and rollback-ready |
| JOURNEY30 | Compare with another mature music app | Same lawful catalog/devices/tasks, measured success/latency/accessibility and honest catalog/service differences |

## Whole-flow review discipline

For each journey test first use, repeat use, empty/partial data, slow operation,
failure/retry, cancellation, keyboard, narrow/high-scale layout and account replacement.
Do not test only the button that opens a form. Record the final actual user outcome.

Every implementation release accepts a named subset of journeys. Declare the subset
and unsupported platforms; a Linux art/scale improvement cannot close Android/iOS
journeys. Optional network integrations do not excuse broken local/offline listening.
