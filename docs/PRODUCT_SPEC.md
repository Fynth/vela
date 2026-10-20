# Vela listening product

## Private listening and local radio0.9

Settings offers separate account/device consent, retention7–365days, real synced JSON export and explicit delete-and-disable. Collection starts off. Listening history lists qualified actual playback sessions and private listened totals; Home shows Recently played only when real entries exist. Seeking/muting/buffering do not invent listened time. The root audio continues through history/settings navigation; repeat after natural EOF creates a fresh logical listen, while pause/retry retain the current one.

Library rows and Home/Search audio-details menus open Track radio from an actual seed. The page offers real Play radio, per-item play/next/last, Refresh, explicit seed/bounded-library analysis, private queue counts and measured audio details. Every result has a measured-signal/tag/library explanation. An unanalyzed or sparse library has an honest metadata/one-track fallback. Analysis remains durable after leaving; page polling stops on departure and cancels stale reads. No cloud inference, fabricated genre/mood prediction, global music catalog or automatic public sharing is implied.

Vela starts with the listener's own authorized music. Home, Search and Library are everyday destinations; storage/users/jobs are secondary administration. All visible music resolves to actual private catalog objects or explicitly labeled playlist metadata access. No fabricated recommendations, history or global catalog.

The0.5 increment evolves the existing Rust/Dioxus app. Home presents bounded new additions/favorites/albums/playlists and honest onboarding. Search queries tracks, artists, albums and permitted playlist names; each category is capped at8 and can be filtered. Opening collections/playlist details uses existing routes/permissions. Track actions play the result context, insert next or append without interrupting current playback.

Desktop keeps a left library/navigation panel, central canvas and persistent bottom player. Phone has Home/Search/Library/Playlists/More; More opens Settings, whose More from Vela list exposes the existing collection/import/network/administrator destinations with keyboard focus and Escape dismissal. Management Jobs navigation is administrator-only; personal import progress stays accessible through Add music. The authenticated root owns the single audio element, queue and cursor; views cannot create another engine. Navigation/theme/menu changes preserve actual src/time.

First-run setup and sign-in retain existing auth/CSRF protections. A new account sees actual import/upload actions; existing catalog data is never replaced with a demonstration dataset. Loading uses skeletons/status; failures provide retry, and changed search queries discard obsolete data immediately. Every asynchronous response must match its session, query/request generation and component lifetime. Shared playlist metadata never authorizes its audio/artwork.

Golden demos are tracked in SPOTIFY_PARITY_MATRIX.0.5 targets G02/G03/G13 and preserves existing G01/G04/G05/G10/G14 subsets. History/radio/handoff/offline/native/federation remain future complete vertical slices; the page does not expose fake buttons for them. See ROADMAP for ordered follow-ups and PROGRESS for executed evidence.

## Playback ordering0.6

Shuffle visits each occurrence once per cycle, keeping real previous/forward traversal. Repeated copies of the same song are separate queue entries. Play next takes priority over shuffle; later Play next requests go first. Move-up/remove operations preserve the active audio and remap every traversal reference. The queue keeps saved visible order and explains this while shuffle is on. Previous after3s restarts the current track without resuming a paused player; otherwise it returns to the previous occurrence/history entry. Manual Next skips even in repeat-one, while natural EOF repeats that track. Repeat-off stops at exhaustion; repeat-all starts a full new cycle and avoids immediate last-to-first repetition when possible. Reload restores bounded order/cursor with a new traversal. Version0.7 additionally restores that account’s position, context, volume and modes, paused until explicit Play. A different account gets its own bookmark or fresh defaults. Listening-event history, gapless, radio and native background playback remain separate features.

## Resume and connection handling0.7

The queue displays its actual originating section, album, artist or playlist, retained through navigation/reload. Choosing a playlist row starts the full authorized playlist at that entry, including repeated tracks. Reordering/removing earlier queue entries leaves the current source/time intact. Reload restores an honest paused bookmark; Play resolves media and applies the saved seek after metadata. New selections start at0.

Connection interruption shows bounded reconnect progress and a manual Retry playback action. Pause cancels outstanding recovery. Decode/unavailable/autoplay failures give actionable explanations; an expired session returns to sign-in and clears the private player while retaining only the original account’s local bookmark. This is streaming recovery, not offline media availability.

## Now Playing0.8

Select the current track in the persistent player to open Now Playing. Desktop presents the cover, title/artist/album, seek, transport and volume beside actual original-file details and a bounded saved-order queue preview. Phones use a full-height scrollable view with the same real controls. Inspection of a queued/paused track does not fetch or start its audio. Closing restores the trigger; Escape and keyboard focus follow native dialog semantics. No decorative lyrics/device/effect controls are exposed.

Play/pause, previous/next, shuffle/repeat, volume, seeking and Retry use the root player. Favorite changes agree across Library, collection rows and queued duplicates. View full queue closes this view and opens the existing queue controls; preview selection selects the exact occurrence. Track changes retain the same dialog/audio element, while navigation/sign-out/expiry close private details. Recovery messages describe the real engine; source properties are shown only when known. Shuffle preview explicitly says saved order rather than predicting the random traversal.

## Personal Home0.10

After the welcome hero, Exploration plays real eligible owned suggestions with
favorite/listening/tag/measured explanations; Rotation plays actual qualified
private listens. Unavailable evidence produces honest cold/empty states. Hidden
choices offer successful-operation Undo, confirmed restore-all and private export.
Turning account history off removes its ranking/suppression influence while
retaining the journal until explicit erase. All playback and queue commands use
root actions; navigation preserves the active audio occurrence. Coverage details
are optional disclosure. These are on-demand sets; no daily/weekly promise.


## Network public-catalog workflow0.11

Network is a real navigation destination on desktop and mobile More. Members see
discovery state, own publication form/records and public-server preview; only admins
edit name/origin/enablement. Enabling alone reveals no music. Owner searches existing
tracks, selects one, declares license/attribution/private evidence, chooses separate
audio grants and confirms rights. Successful create refreshes owner records; stale
CAS or uncertain timeout offers explicit refresh. Original preview uses the same
root player. Revocation keeps original music/audit metadata, retires public ID and
blocks new public transfers. Previously delivered bytes/licenses cannot be recalled.

Preview accepts one public HTTPS DNS origin and presents bounded actual catalog
metadata with source/declared-license/grant labels; absent audio grants have no
active stream/download link. This is external-origin access, not saved connections,
federated social search or remote tracks in the private root queue. Unsupported
features have no decorative controls. No automatic publish, evidence fetch, remote
audio import or private history disclosure is part of this flow.
