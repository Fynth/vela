# Vela listening product

Vela starts with the listener's own authorized music. Home, Search and Library are everyday destinations; storage/users/jobs are secondary administration. All visible music resolves to actual private catalog objects or explicitly labeled playlist metadata access. No fabricated recommendations, history or global catalog.

The0.5 increment evolves the existing Rust/Dioxus app. Home presents bounded new additions/favorites/albums/playlists and honest onboarding. Search queries tracks, artists, albums and permitted playlist names; each category is capped at8 and can be filtered. Opening collections/playlist details uses existing routes/permissions. Track actions play the result context, insert next or append without interrupting current playback.

Desktop keeps a left library/navigation panel, central canvas and persistent bottom player. Phone has Home/Search/Library/Playlists/More; the More sheet exposes existing collection/import/settings destinations with keyboard focus and Escape dismissal. Management Jobs navigation is administrator-only; personal import progress stays accessible through Add music. The authenticated root owns the single audio element, queue and cursor; views cannot create another engine. Navigation/theme/menu changes preserve actual src/time.

First-run setup and sign-in retain existing auth/CSRF protections. A new account sees actual import/upload actions; existing catalog data is never replaced with a demonstration dataset. Loading uses skeletons/status; failures provide retry, and changed search queries discard obsolete data immediately. Every asynchronous response must match its session, query/request generation and component lifetime. Shared playlist metadata never authorizes its audio/artwork.

Golden demos are tracked in SPOTIFY_PARITY_MATRIX.0.5 targets G02/G03/G13 and preserves existing G01/G04/G05/G10/G14 subsets. History/radio/handoff/offline/native/federation remain future complete vertical slices; the page does not expose fake buttons for them. See ROADMAP for ordered follow-ups and PROGRESS for executed evidence.
