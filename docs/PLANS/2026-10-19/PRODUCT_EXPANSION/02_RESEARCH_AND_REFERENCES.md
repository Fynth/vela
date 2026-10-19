# Primary-source research and implementation decisions

Reviewed on 2026-10-19. This is a selected primary-source survey, not an assertion
that every music application or every page on the Internet was inspected. Product
documentation establishes patterns and advertised capabilities, not comparative
reliability. Vela proposals and architectural judgments below are our decisions.
Revalidate provider terms, API status and pinned platform support before integration.

## Music product patterns

| Ref | Source reviewed | Relevant evidence and Vela decision |
| --- | --- | --- |
| S01 | [Spotify keyboard shortcuts](https://support.spotify.com/us/article/keyboard-shortcuts/) | Discoverable keyboard commands support frequent tasks. Provide a command list and remapping without intercepting text editing. |
| S02 | [Spotify Play Queue](https://support.spotify.com/us/article/play-queue/) | Current/upcoming context deserves an explicit surface. Separate saved order, actual shuffle-next and manual next priority. |
| S03 | [Spotify Jam](https://support.spotify.com/us/article/jam/) | Shared queues and host permissions are distinct controls. Vela rooms require each participant's actual media entitlement. |
| S04 | [Apple Music playback settings](https://support.apple.com/en-nz/guide/music/musdf855a1b/mac) | Audio transitions and leveling have separate options. Use capability-gated settings rather than copying subscription-only format claims. |
| S05 | [Apple Smart Playlists](https://support.apple.com/en-gb/guide/music/-mus1712973f4/mac) | Rules, live updates and limits need an editor. Vela stores a typed bounded expression, never user-authored SQL. |
| S06 | [Navidrome features](https://www.navidrome.org/docs/usage/features/) | A lightweight server can integrate with existing clients. Keep native Vela capabilities distinct from protocol compatibility. |
| S07 | [Navidrome multi-library](https://www.navidrome.org/docs/usage/features/multi-library/) | Library selection is also an authorization scope. Show accessible libraries and preserve origin identities in search/playlists. |
| S08 | [Navidrome security](https://navidrome.org/docs/usage/admin/security/) | UI-editable transcoder commands have server-execution risks. Vela offers reviewed presets, never arbitrary shell templates. |
| S09 | [Navidrome configuration](https://www.navidrome.org/docs/usage/configuration/options/) | Many operational knobs exist. Translate useful ones into policy-aware controls rather than dumping raw keys on listeners. |
| S10 | [Symfonium settings](https://docs.symfonium.app/wiki/settings/symfonium-settings/) | Settings span application, interface, playback, providers, offline and profiles. Organize Vela by user task and scope. |
| S11 | [Symfonium product](https://www.symfonium.app/) | Customization, multiple sources, local audio and offline workflows illustrate breadth. Do not assume its native capabilities transfer to a browser. |
| S12 | [Finamp upstream](https://github.com/finamp-app/finamp) | Downloaded media, lyrics and native system integration need a real client. Study workflows without replacing Dioxus with Flutter. |
| S13 | [Jellyfin music](https://jellyfin.org/docs/general/server/media/music/) | Embedded tags, discs, artwork and local lyric sidecars are practical ownership features. Preserve originals and record metadata provenance. |
| S14 | [Funkwhale federation](https://docs.funkwhale.audio/developer/federation/index.html) | Follow/approval and library visibility affect remote access. Specify Vela interoperability and authorization independently of public metadata. |
| S15 | [ListenBrainz recommendations](https://listenbrainz.readthedocs.io/en/latest/users/api/recommendation.html) | External recommendations return identifiers and may be experimental. Resolve IDs to authorized content and show unavailable matches. |
| S16 | [ListenBrainz export](https://listenbrainz.readthedocs.io/en/latest/users/api/export.html) | Private exports are account-authenticated background operations. Keep tokens out of URLs and exports; bound preparation and downloads. |
| S17 | [Music Assistant player providers](https://www.music-assistant.io/player-support/) | Device protocols vary by provider. Model capabilities instead of advertising every speaker as an identical player. |
| S18 | [Music Assistant playback destinations](https://www.music-assistant.io/faq/stream-to/) | Multiroom synchronization and codec support vary. Vela casting/room sync requires provider and hardware evidence. |

Plexamp sonic exploration was investigated through search-visible official material,
but the direct [sonic-analysis article](https://support.plex.tv/articles/sonic-analysis-music/)
returned HTTP403 and the Plexamp landing page failed to open. Therefore detailed
current implementation/availability was not verified. Similarity journeys and
analysis-aware transitions below are original conditional Vela proposals, not
reverse-engineered Plex algorithms or a promise of current provider integrations.
Forum feature requests are not treated as proof of shipped features.

## Technical and accessibility sources

| Ref | Primary source | Constraint for implementation |
| --- | --- | --- |
| S19 | [Pinned Dioxus DesktopService0.7.10](https://docs.rs/dioxus-desktop/0.7.10/dioxus_desktop/struct.DesktopService.html) | set_zoom_level exists; local implementation logs an error but returns no Result. A preference adapter needs explicit error handling/verification. |
| S20 | [Wry0.53.5 source](https://docs.rs/crate/wry/0.53.5/source/src/lib.rs) | Local pinned WebView.zoom returns Result; Android is unsupported, Apple minimum OS limits apply. Direct method-page fetch failed; local source was inspected. |
| S21 | [WCAG Resize Text](https://www.w3.org/WAI/WCAG22/Understanding/resize-text.html) | Preserve content/function with text enlargement to200%; an app scale slider alone is insufficient. |
| S22 | [WCAG Reflow](https://www.w3.org/WAI/WCAG21/Understanding/reflow) | Test320 CSSpx-equivalent content and400% browser zoom, including settings recovery. Handle genuine two-dimensional tables narrowly. |
| S23 | [MDN zoom](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/zoom) | CSS zoom affects layout; transform scale is not an interchangeable layout solution. Validate exact browsers and WebKit versions. |
| S24 | [Android MediaSessionService](https://developer.android.com/media/media3/session/background-playback) | The service/media session must outlive the activity. Background audio is not proven by compiling shared UI. |
| S25 | [Apple AVAudioSession](https://developer.apple.com/documentation/avfaudio/avaudiosession) | Configure playback category, activation, interruptions and background mode deliberately. Actual device behavior requires testing. |
| S26 | [OpenSubsonic overview](https://opensubsonic.netlify.app/docs/) | A client/server API with negotiated extensions; not a federation protocol or automatic remote media grant. |
| S27 | [ActivityPub](https://www.w3.org/TR/activitypub/) | Use standard actor/object/delivery concepts where compatible; define a bounded music profile and independent stream authorization. |
| S28 | [RFC9110 HTTP Semantics](https://www.rfc-editor.org/rfc/rfc9110.html) | Preserve HEAD/GET, validators,206/416 and If-Range when changing preparation or prefetch. |
| S29 | [RFC9111 HTTP Caching](https://www.rfc-editor.org/rfc/rfc9111.html) | Define a compatible authenticated media-cache policy; never silently ignore no-store or confuse cache with offline entitlement. |
| S30 | [OpenID Connect Core](https://openid.net/specs/openid-connect-core-1_0.html) | Optional portable identity binds issuer+subject, not matching email addresses. It does not carry music permissions. |
| S31 | [OAuth security BCP RFC9700](https://www.rfc-editor.org/rfc/rfc9700.html) | Use reviewed native/web flows and least-privilege tokens; no homegrown global password broker. |
| S32 | [MusicBrainz API](https://musicbrainz.org/doc/MusicBrainz_API) | Identify the application, obey rate limits and store provenance; provider policy must be checked again at implementation. |
| S33 | [MusicBrainz data license](https://musicbrainz.org/doc/About/Data_License) | Metadata licensing differs by dataset. Metadata access does not license audio or cover imagery. |
| S34 | [Cover Art Archive API](https://musicbrainz.org/doc/Cover_Art_Archive/API) | Art resolution, redirects and attribution need controlled fetching. Availability is not blanket permission to republish all images. |

Better Design MCP UI/UX principles were loaded for accessible scale, progressive
settings disclosure, stateful forms, failure/recovery and keeping the original design.
They guide proposed interactions, not an executed visual approval. No rendered UI is
created here, so DOM spacing measurement is an implementation acceptance requirement,
not a passing result of this plan.

## What to adopt and what to avoid

Adopt predictable transport, contextual libraries, clear queues, real offline
management, searchable preferences, explainable recommendations and scoped social
sharing. Add ownership-specific advantages: exact source quality, edition choice,
portable metadata, private DSP, policy transparency, multiple authorized catalogs
and no mandatory cloud identity.

Avoid a subscription catalog illusion, copied brand assets, chat-first AI navigation,
uniform card boxes around every setting, raw administration in the listener's Home,
settings that do nothing, silent history collection and universal “bit-perfect” claims.

Feature breadth costs testing, discoverability and maintenance. The accepted tradeoff
is deep optional capability behind a small stable listening surface, with searchable
advanced preferences and platform-specific availability. Defaults must work without
configuration; configuration must not hide broken default behavior.
