# Threat model and security architecture

## Private listening and analysis0.9

Listening is disabled globally and per device by default, including upgrades. Each BEGIN IMMEDIATE event transaction independently checks current privacy revision, device consent and track ownership; composite foreign keys enforce owner references. Bounded cumulative progress/sequence IDs deduplicate retries. Delete disables every device and increments revision atomically, so delayed old batches cannot resurrect erased data. Reads/exports are owner-only even for administrators, no-store, bounded and retention-filtered. All writes retain CSRF/Origin and16KiB body limits. Browser outboxes are private plaintext local metadata, capped100 summaries/128KiB and cleared on logout/opt-out; no URLs/tokens enter them. Conservative audible-time signals are self-reported personal statistics, not proof of physical sound or public popularity. Privacy epochs also suppress held exports after erasure.

Analysis requires explicit owner selection. Shared physical cache identity never supplies another user's track or history. One serial blocking decoder has cooperative byte/frame/packet/channel/duration/deadline/cancellation limits; this is not a subprocess memory sandbox or a guarantee against all parser bugs. Fixed error DTOs omit paths/parser details. Current-version quotas cap128 global/50 requester active jobs; obsolete unfinished extractor jobs are retired without deleting completed results. Radio authorizes every seed/candidate and reads at most500 cached vectors, never decodes on GET or grants media through playlist metadata. New peers/publication/native credentials do not exist in this slice; NETWORK_RIGHTS/FEDERATION_PROTOCOL remain explicit requirements, not implemented protections for a nonexistent public network.

## Local search and listener state (0.5.0)

GET /api/search requires authentication, denies unknown/malformed query fields, caps raw query text at256 bytes and rejects empty/control input. Parameterized owner constraints apply to every catalog result; playlist names require ownership or a current named collaborator grant. Grant revocation immediately removes a playlist from new searches. A result never grants audio/artwork/download access. Each category is bounded8, private no-store applies and request query values are not traced by the server. Real TCP tests cover private same-file catalogs, revoked grants, hidden empty groups, literal hostile text, bounds and invalid queries.

Home/Search physically abort superseded page requests and reject stale session/query/generation/lifetime replies. Browser regressions hold actual backend responses across query/account changes and assert no private data returns. Logout clears active audio source, pending play identity and OS metadata. A late failed play promise cannot report over a newer selection. These controls do not constitute offline token storage, cross-server credentials or an independent security audit.

Assets: user audio, private catalog metadata, account credentials, session/CSRF secrets, share tokens, import source URLs, filesystem capacity, server network reachability and availability. Trust boundaries: browser/API caller, administrator configuration, embedded media tags/artwork, public download servers and future federation peers. Files supplied by users and URLs/redirects/DNS/metadata are untrusted.

## Implemented controls

- Argon2id password hashes with per-account random salts; bounded two-worker password hashing, equivalent verification work for missing accounts; passwords12–128 bytes at creation. ASCII usernames3–32 characters, case-insensitive uniqueness. Administrator-created accounts only.
- Random256-bit sessions and shares, BLAKE3 token digests persisted. Seven-day session expiration and at most20 sessions/account; logout deletes session. Session cookie HttpOnly/SameSite=Strict, configurable Secure for HTTPS. Per-session random CSRF secret required as X-CSRF-Token for authenticated writes; no URL-based credentials.
- Write requests reject cross-site Fetch Metadata and invalid/mismatched Origin. Login/setup JSON requests also use origin checks. No permissive CORS. Bind to loopback by default; first administrator claims the server once, atomically. Finish setup before external exposure.
- Login rate limiting uses actual peer addresses, never untrusted forwarded headers:15 attempts/address/15min and100 global/min, bounded address map. At most64 active response-generation handlers, two uploads, one ingestion parser, one import worker, one scanner worker and16 streamed audio/artwork bodies. Body/transfer limits and staging cleanup apply.
- Every catalog, favorite, artwork and original-media route verifies ownership; a shared playlist exposes permitted titles, never a stream grant. Local collaborator view and edit permissions are explicit; only owners manage collaborators/links. Random expiring link secrets are hashed and revocable. Browser share links carry fragments, not initial-page query credentials. Private data receives no-store; no-referrer, nosniff and restrictive CSP are applied.
- SQL parameters, foreign keys, explicit migrations, FTS literal token quoting, bounded pagination. Playlist mutations use transactional revision CAS and actor/request-bound idempotency UUIDs; exact reorder sets are validated. Unknown/private media is404.
- Upload paths are random and configured; original names are display metadata only. Staging symlinks are rejected. Exact hash-derived managed storage paths are validated. Ingestion parses actual bytes, bounds metadata read/allocation, decodes only bounded JPEG/PNG/WebP, discards bad artwork and emits JPEG thumbnails. No original tag write-back, shell media execution or automatic duplicate deletion.
- Multipart part headers are capped at8KiB before Multer parsing; additional parts are rejected before publication. Input frames are split at16KiB with a scheduler yield to bound parser read-ahead. Album/artist summaries and track pages enforce owner scope; associations reject foreign owners and group cursors cannot resolve another user's track.
- Controlled public URL imports accept HTTP/S and ports80/443, reject credentials/control characters/long URLs, validate all resolved addresses, pin actual destinations, disable environment proxies and compression, bound redirects and validate every hop. Reject loopback, private/shared/reserved/link-local/metadata ranges, IPv4-mapped/transition IPv6 and Azure's special168.63.129.16 host-service address. Native IPv6 global unicast policy is conservative. HTTPS cannot redirect to HTTP. DNS/read/connect/total deadlines, transfer limits and disk reserve are enforced.
- SQLite and immutable content persist private-by-default. Single-server OS file locking prevents competing recovery/cleanup in the same data directory. Data directory permissions are0700 on Unix. Startup cleans only recognized unpublished regular temp files. Media/assets remain separate from per-owner tracks; shared physical bytes do not grant shared access.
- No arbitrary executables/plugins, URL-to-shell utilities, remote metadata enrichment, public directory, API-key implementation or federation. No remote peer can fetch private data through a nonexistent federation endpoint. Infrastructure import/scanner worker failure stops the process; graceful shutdown cancels receiving uploads, completes processing and drains before closing SQLite.

## Known threats and limits

This has regression coverage and peer review within development; it has not received an independent security audit. TLS terminates at an administrator-managed reverse proxy; production requires HTTPS/secure cookies. Hard-coded login limits treat all clients behind an unconfigured reverse proxy as one peer; trusted proxy support needs an explicit allowlist design.

Metadata parsing validates container structure/properties, not every decoded audio frame. File/metadata/image bounds reduce risk but cannot establish absence of parser vulnerabilities. Managed quota counts physical referenced audio; artwork/staging/orphan blobs are bounded per operation but not fully included in that global budget. Unexpected crashes between durable publication and SQLite commit retain unreferenced blobs. Backups include password hashes/session state/source URLs and must remain private; backup checksums provide integrity detection, not encryption or signature authenticity.

Private browser metadata/queue state is scoped by account but still resides in that browser's local storage. Browser/session compromise is outside server authorization protections. Query strings in download URLs may be sensitive; URLs are needed for durable retries and are stored privately, never logged or reflected in shared data. The public downloader cannot prove redistribution rights; users must use available authorized downloadable content. Commercial playlist metadata adapters and native credential storage are not implemented.

Private playlist sharing has no stream/download grant: those playlist entries remain owner-only. Explicit public publications use the separate0.11 policy below. Named playlist viewers can see titles their owner explicitly shares; artwork may remain unauthorized and falls back to a neutral UI cover. All future federation data leaving a server must be separately opted in, signed/origin-verified, limited and authorized; private libraries must never become public merely because federation is enabled. Remote administrators/backups cannot be guaranteed to retract disclosed content.

## Regression requirements

Tests cover malicious numeric/IP/IPv6/credential URL forms, mixed public/private DNS, private redirects, redirect loops, compressed/HTML/non-audio rejection, size/queue/storage quotas, exact dedup privacy, symlink paths, image dimension/allocation budgets, CSRF/origins, unauthorized catalogs/artwork/streams, stale concurrent playlist edits, idempotency, expired/revoked links, account isolation, persistent recovery/cancellation and real original-byte streaming. Keep tests deterministic; network-only browser import is separately opted in and documented.

## Dependency review checkpoint

`cargo-deny0.20.2` checked the locked dependency graph. The initial run rejected the unlisted CDLA-Permissive2.0 certificate-root license, versionless local path dependencies and Lofty's transitive unmaintained `paste` macro advisory. Local paths carry the workspace version; CDLA-Permissive2.0 is explicitly allowed and its upstream text is included in generated notices.

[RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436) is an unmaintained compile-time macro notice with no patched release, rather than a reported runtime vulnerability. `deny.toml` records this single reasoned exception through current Lofty0.25.4; review/remove it on every Lofty update. No runtime vulnerability advisory is ignored. Duplicate major dependency generations remain warnings. The resolved graph subsequently passed advisory/bans/license/source policy checks with this documented exception. This is an automated dependency policy check, not an independent legal/security audit.

Generated notices preserve crate-provided license texts and public upstream license files at packaged VCS commits where available, including source URL/SHA256 records. Metadata-only notices remain explicitly reported; review them before external redistribution. Nothing was externally published by this session.

Public/unlisted views disclose only the playlist owner's track metadata. Private contributor entries remain available only through authenticated playlist grants and are omitted from link views. Diagnostic Debug implementations redact passwords, session/CSRF/share tokens and import URLs; serialization still carries the appropriate authorized API fields. Arbitrary static filenames and unknown API paths receive404 even if a web-root configuration points at additional files.

Metadata edits are owner-only, independently of playlist edit grants. Unknown update fields and omitted nullable fields are rejected; text/control-character/numeric and16KiB transport limits apply. Transactional revision checks prevent stale concurrent overwrites; audit rows contain fixed actions and IDs, never raw tag values. Catalog edits cannot modify source bytes, codec/duration, artwork, asset paths or another owner's same-file catalog.

## Configured folder boundary (0.2.0)

Only administrators can list configured roots/create sources/enqueue or cancel scans; every write requires session CSRF/Origin checks and every stored job/source is owner-scoped. JSON APIs accept opaque root IDs, never filesystem paths. Operator configuration rejects system/managed/overlapping roots and their aliases. On Linux, descriptor-relative no-follow opens reject symlink ancestors/entries; files are opened nonblocking before validating regular-file/single-link metadata. Hashed path keys and compact signatures stay in private SQLite, never public DTOs/logs. Source/track ownership is also guarded by database triggers.

Traversal depth/entries/files, persistent fingerprints, source/job queues, copy size/free-space/time and shared parsing concurrency are bounded. Cancellation is checked between blocking I/O operations; atomic ingestion may finish before cancellation completes. Snapshots are fsynced and validated through the same real-media pipeline as uploads. Source directories are never written to; removed sources/missing files retain managed content. Only recognized scanner temporary files are cleaned. Root configuration and source storage are installation trust; an administrator must not expose sensitive host folders. Linux mounts should be read-only at the OS/container boundary too. Local/network kernel I/O cannot be forcibly terminated and network source filesystems are unsupported.

The2026-10-15 dependency review detected four runtime advisories in rustls0.23.45 ([RUSTSEC-2026-0346](https://rustsec.org/advisories/RUSTSEC-2026-0346), [0347](https://rustsec.org/advisories/RUSTSEC-2026-0347), [0348](https://rustsec.org/advisories/RUSTSEC-2026-0348), [0349](https://rustsec.org/advisories/RUSTSEC-2026-0349)). The lockfile now uses patched rustls0.23.46; none were ignored. The new capability library's Windows dependency winx uses Apache-2.0 WITH LLVM-exception; the exact SPDX expression is allowed and its shipped license is preserved in generated notices. The existing compile-time paste exception remains the only ignored advisory.

## Typed filter boundary (0.3.0)

Catalog facets require authentication and carry no-store headers just like private tracks. Every genre/codec/year/quality/search/favorite clause remains conjunctive with ownership; cross-owner physical dedup never shares tag suggestions. Genre/format are complete literals, never SQL fragments/LIKE patterns. Sorting is a closed enum with static SQL expressions. Text byte limits, controls, numeric ranges, bounded facets/pages and UUID validation apply inside MediaService as well as HTTP; malformed transport values receive fixed JSON400 without reflecting raw values. Sorted cursor lookups are owner-scoped in the page's read snapshot and reject foreign/missing IDs identically before FTS early returns. No new sharing or filesystem permission is introduced.

Dioxus keeps unsaved fields separate from successful applied requests; session/page/generation guards protect both tracks and suggestions. More recent requests win, invalid drafts cancel obsolete results, and account changes reset state. Facets expose at most100 genres/32 formats with truncation flags. SQL work can still depend on selectivity/distribution; bounded result sizes are not a guarantee of constant CPU or query latency. Independent development review is distinct from an external security audit.

## Background-task boundary (0.4.0)

The authenticated no-store task read model is owner-private, including its all-filter activity flag and action hints. Administrator status grants access to one's own scans, never another account's task history. Ordinary users cannot explicitly request scan history. Typed bounded queries/canonical cursors reject unknown fields, enum values, overflows and malformed input with fixed JSON errors.

Queries never select source URLs or raw worker diagnostics. Completed-import track joins require matching owners; source joins likewise require matching ownership. Fixed error text avoids exposing worker internals in general history. Task IDs/counters/title metadata are escaped by Dioxus. The cursor is a paging position, never an access credential; changing it cannot change the owner scope.

Existing cancel/retry/scan APIs retain CSRF/Origin/ownership/state/queue checks; advertised availability is not authorization. Core reads rows/activity/action hints in one transaction. UI guards exact session, generation, selected filters/page and mounted lifetime after awaits; logout clears history and late responses cannot repopulate it. Hidden/idle Jobs views stop network polling. A failed refresh visibly marks the last confirmed data stale and disables its actions. No new remote egress policy or media/share privilege is introduced.

## Notice provenance hardening (0.4.0)

The release notice generator now discovers shipped nested license directories and the explicitly reviewed r-efi AUTHORS grants. Pinned wasite texts preserve original bytes; both winapi architecture packages were matched file-by-file to their upstream commit before root license texts were retained. Cached texts require an HTTPS source record and matching SHA256, with bounded reads; missing/tampered provenance fails packaging. Nine Python tooling regressions include these integrity cases. Metadata-only gaps decreased12→6; exact inspected archive hashes and honest missing-grant findings are retained in [notice provenance](third-party-license-texts/README.md). This supplies build evidence, not an independent legal review or authorization to publish externally.

## Playback-order isolation0.6

The pure client order model is capped1000 entries/history states and never supplies authorization. Legacy local queue restoration is capped1000; original HTTP routes still authorize every stream. Shuffle/repeat reset across account changes; metadata changes retain occurrence traversal without widening access. The browser account-change regression explicitly verifies new-user modes as well as stale data/queue isolation. No credentials, media paths, listening events or remote control channel were added to this model.

## Playlist response context0.6.1

Private playlist opens, mutation-refresh snapshots, collaborator updates and returned share-link UI are bound to exact session/view generation and object. Navigation invalidates old requests, including leave-and-return to the same route. Opening a new playlist clears the preceding link; an old created token cannot populate the newly selected playlist form. Already committed server actions remain independently authorized; dropping a stale response is not a revocation or rollback. Refreshes cannot lower the displayed playlist revision, and collaborator list responses must match the current collaborator revision. The PLNAV01 regression holds actual API responses; source tokens are not logged.

## Browser bookmark and recovery0.7

A versioned atomic account-local bookmark validates owner, current occurrence, UUID paths, byte/count/position/preference limits before use. It contains only catalog metadata and local playback state, no cookies/CSRF/import URLs or bearer/share credentials. Browser localStorage is plaintext and accessible to same-origin script; account scoping prevents accidental state mixing, not malicious browser owners or XSS. HTTP authorization still applies to every media/artwork request. Legacy rollback/fallback remains bounded.

Deferred media/retry/probe work checks the exact active session, source generation, audio element and expected same-origin UUID stream route. Pause/selection/logout cancel timers/probes and invalidate play promises; duplicate URLs are not occurrence identities. HEAD uses normal same-origin credentials with a10s abort deadline.401 releases the source and private root state; successful probes never replace authentication. Three retry delays and a progress watchdog bound consecutive failures. Existing delivered/buffered bytes cannot be revoked retroactively. RES/NET/AUTH browser scenarios use real fixture media, actual held/aborted transfers and genuine server-session revocation; no production credentials are read.

## Now Playing isolation0.8

The full player reveals only metadata already in the authenticated root queue. No new media grant, endpoint, credential or path is introduced; favorite writes retain session/CSRF authorization. Root feedback also matches open generation and current occurrence so a held reply cannot overwrite a new track, reopened view or account. A genuine401 still expires the session and clears private player details. Native dialog background inertness is a UI property, never an authorization mechanism. The server permits exactly /now-playing.css; neighboring CSS, SQLite files and traversal remain denied by the static-asset regression.

The0.8 cross-browser follow-up uses fresh isolated Firefox/Chromium profiles and separate temporary server databases. It retains real CSRF/session/revocation/favorite-race checks and never imports a signed-in user profile. Cold-network setup releases only the test element's buffered resource before reload; no production cache/authorization policy, credentials or storage is changed.

## Personal mixes and feedback0.10

Recommendations capture current owner consent/revision/retention and actual media
in one read snapshot. Disabled history is not queried, including for suppression;
expired and unqualified sessions supply no signal. No private events leave the
process or become public popularity. Returned metadata grants no stream right.
Migration0010 stores owner/track exclusions under a composite FK; desired-state
mutations use serialized ownership+quota checks and CSRF/Origin protection.
Export is capped at1000 own UUIDs without paths/secrets. Clear exclusions and erase
history are separate explicit operations. No implicit collection or automatic
analysis is introduced. Dioxus cancels obsolete requests and checks privacy
epoch/revision/busy, session and lifetime before rendering or acting on a mix.
Ordinary history/catalog freshness cannot discard an unsettled feedback receipt.
Tests and exact executed evidence are in PROGRESS/QA_EVIDENCE.


## Explicit publication and controlled discovery0.11

No upload, scan, recommendation, listening consent, collaboration or server-enable
action implicitly creates a public record. Owner publication is an authenticated
CSRF/Origin-protected mutation with explicit unchecked rights confirmation; the
administrator independently controls discovery. Selected supported licenses and
private evidence are declarations, never copyright verification. Public artwork
is absent. Public DTO construction selects only allowlisted metadata: no owner IDs,
paths, physical hashes, private provenance, history or taste vectors. Text is bounded,
controls removed in public catalog projections and escaped in Dioxus; owner tags stay
unchanged. Duration/year/grant/license bounds are enforced at both producer and peer.

Serialized transactions enforce1000 retained publications per owner,10000 global,
one active record per owner/track, composite owner foreign keys and revisioned
revocation. Metadata access cannot authorize audio. Every new GET/HEAD/range/download
checks current enablement and grant; revoked IDs404 even if later republished under
a new ID. Ordinary private streams/artwork keep existing authentication. Revocation
cannot erase delivered bytes or undo licenses already granted; stream recipients can
retain bytes even without a separate download endpoint permission.

Anonymous transfers admit at most8 inside the existing16-stream total. Body-drop
releases both permits; owner capacity remains available under anonymous admission
saturation. General request admission stays64. Public endpoints return no-store;
only the exact registered well-known route passes the static allowlist.

Peer preview requires authentication+CSRF and canonical HTTPS DNS origin on443,
max2 concurrent/5s, two documents16KiB/128KiB, max20 records. Every request/redirect
validates all DNS addresses, pins actual dialing, rejects special/private targets
and cross-origin/downgrade redirects, and sends no ambient proxies/cookies/auth/
referrer. Global4-slot DNS admission is held inside the real blocking resolver even
if its caller cancels. A stalled OS resolver cannot be forcibly stopped; admission
remains occupied until it finishes. HTTP futures cancel on timeout. No production
private-network escape flag exists. Test hostname pins are cfg(test)-only.

The current read-only profile has no signed activity/inbox/cache. Peer origin+ID is
remote identity scope; a claimed UUID is not trusted global identity. Full signature,
replay, blocklist, social delivery and Internet interoperability acceptance remain
unfinished. A local two-server fixture is not a public deployment or license proof.

The user-service installer verifies same-user frozen binaries/assets and exact
source migration checksums before touching supervision. No symlink/control paths,
unrelated/drop-in services or schema upgrades are accepted. Private unit write,
readiness and previous-unit rollback are tested separately from actual installation;
user-manager start behavior is not reboot/logout/device validation.
