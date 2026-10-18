# Network interoperability boundary

0.11 implements an optional read-only public catalog profile. Full ActivityPub,
WebFinger, HTTP message signatures, signed inbox/outbox, replay-safe events,
follows, peer blocks, durable delivery and Funkwhale interoperability remain
NOT_IMPLEMENTED. Do not advertise federation compatibility from these GET routes.

The server is private by default. Enabling public discovery publishes no tracks.
The owner must separately create an explicit licensed publication; metadata,
streaming and downloads are distinct grants. See NETWORK_RIGHTS and ADR0016.

## Read-only public catalog profile1

`GET /.well-known/vela` returns protocol_version1, stable server_id, configured
canonical origin, name, software_version and public_artworkfalse. Origin is an
HTTPS DNS name on443 with root path, never derived from incoming Host. UUID is a
peer claim; remote identity is scoped by canonical origin, not UUID alone.

`GET /api/public/catalog?limit=20&cursor=<publication UUID>&q=<literal text>`
returns server_id/origin/items/next_cursor. Default20, max50; bounded literal
text filter and ascending publication UUID keyset. A nonnull next_cursor is the
last returned ID and means another page existed in that read snapshot.
`GET /api/public/catalog/{id}` returns one current PublicTrack: public ID/revision,
bounded title/artist/album/genre/year/duration/codec, declared license/attribution,
and can_stream/can_download. No owned track IDs, artwork, URLs, provenance,
physical hashes, user identity, favorites, private history or analysis descriptors.

`GET/HEAD /api/public/media/{id}/stream` requires that current public record's
stream grant. `/download` additionally requires its download grant. Standard
Range/If-Range/ETag behavior reuses original streaming with bounded file buffers;
attachment names use only the canonical public UUID. Disabled/revoked/foreign/
missing/metadata-only records give404. Every response is no-store; previously
received bytes cannot be recalled. Public reads confer no authority on private
`/api/tracks/{local-id}/stream` or artwork endpoints.

Authenticated/CSRF-protected `POST /api/network/preview {origin}` fetches only
well-known + first20 catalog records. Max2 concurrent previews/5s; bodies16KiB and
128KiB. Validate same origin/server identity, version, sorted unique canonical IDs,
matching cursor, text/numeric/license/grant bounds. DNS answers must all be public,
addresses pinned for actual connection; each redirect revalidated and same-origin.
No ambient proxy/cookies/authorization/referrer; no automatic recursive fetch,
remote writes, audio proxy/download, metadata cache, saved peer or credentials.
System DNS workers have a separate4-task admission budget held through actual
blocking completion after timeout; HTTP futures cancel on deadline.

Tests may route exact fixture hostnames to real independent loopback Axum servers
through a `cfg(test)`-only pin. No production flag/API/environment permits private
peers. This proves the HTTP/catalog/rights boundary; it does not validate Internet
TLS, a public deployment, signatures or a standards-based federation network.

Next network acceptance: durable origin-scoped peer records/blocks, standards-based
actor identity/signature verification and replay-safe delivery, separate authorized
remote catalog adapters and a root-coordinated remote queue. No public directory or
central service is mandatory; private history must never enter public popularity.
