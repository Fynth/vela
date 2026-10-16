# Vela listening product

Vela starts with the listener's own authorized music. Home, Search and Library are everyday destinations; storage/users/jobs are secondary administration. All visible music resolves to actual private catalog objects or explicitly labeled playlist metadata access. No fabricated recommendations, history or global catalog.

The0.5 increment evolves the existing Rust/Dioxus app. Home presents bounded new additions/favorites/albums/playlists and honest onboarding. Search queries tracks, artists, albums and permitted playlist names; each category is capped at8 and can be filtered. Opening collections/playlist details uses existing routes/permissions. Track actions play the result context, insert next or append without interrupting current playback.

Desktop keeps a left library/navigation panel, central canvas and persistent bottom player. Phone has Home/Search/Library/Playlists/More; the More sheet exposes existing collection/import/settings destinations with keyboard focus and Escape dismissal. Management Jobs navigation is administrator-only; personal import progress stays accessible through Add music. The authenticated root owns the single audio element, queue and cursor; views cannot create another engine. Navigation/theme/menu changes preserve actual src/time.

First-run setup and sign-in retain existing auth/CSRF protections. A new account sees actual import/upload actions; existing catalog data is never replaced with a demonstration dataset. Loading uses skeletons/status; failures provide retry, and changed search queries discard obsolete data immediately. Every asynchronous response must match its session, query/request generation and component lifetime. Shared playlist metadata never authorizes its audio/artwork.

Golden demos are tracked in SPOTIFY_PARITY_MATRIX.0.5 targets G02/G03/G13 and preserves existing G01/G04/G05/G10/G14 subsets. History/radio/handoff/offline/native/federation remain future complete vertical slices; the page does not expose fake buttons for them. See ROADMAP for ordered follow-ups and PROGRESS for executed evidence.

## Playback ordering0.6

Shuffle visits each occurrence once per cycle, keeping real previous/forward traversal. Repeated copies of the same song are separate queue entries. Play next takes priority over shuffle; later Play next requests go first. Move-up/remove operations preserve the active audio and remap every traversal reference. The queue keeps saved visible order and explains this while shuffle is on. Previous after3s restarts the current track without resuming a paused player; otherwise it returns to the previous occurrence/history entry. Manual Next skips even in repeat-one, while natural EOF repeats that track. Repeat-off stops at exhaustion; repeat-all starts a full new cycle and avoids immediate last-to-first repetition when possible. Reload restores bounded order/cursor with a new traversal. Version0.7 additionally restores that account’s position, context, volume and modes, paused until explicit Play. A different account gets its own bookmark or fresh defaults. Listening-event history, gapless, radio and native background playback remain separate features.

## Resume and connection handling0.7

The queue displays its actual originating section, album, artist or playlist, retained through navigation/reload. Choosing a playlist row starts the full authorized playlist at that entry, including repeated tracks. Reordering/removing earlier queue entries leaves the current source/time intact. Reload restores an honest paused bookmark; Play resolves media and applies the saved seek after metadata. New selections start at0.

Connection interruption shows bounded reconnect progress and a manual Retry playback action. Pause cancels outstanding recovery. Decode/unavailable/autoplay failures give actionable explanations; an expired session returns to sign-in and clears the private player while retaining only the original account’s local bookmark. This is streaming recovery, not offline media availability.
