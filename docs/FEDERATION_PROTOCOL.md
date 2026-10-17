# Federation integration checkpoint

Status2026-10-17: NOT_IMPLEMENTED, disabled. This is an implementation boundary for the next network slice, not a claim of ActivityPub or Funkwhale interoperability.

Preserve three separate layers: standards-based identity/social delivery; versioned catalog capability negotiation; origin-authorized media access. [ActivityPub](https://www.w3.org/TR/activitypub/) defines actor/inbox/outbox and activity delivery; it does not supply a music redistribution grant. [HTTP Message Signatures, RFC9421](https://www.rfc-editor.org/rfc/rfc9421.html) defines signed message components; a selected interoperability profile must also fix key discovery, covered components, digest handling, freshness, replay storage and rotation. Both primary specifications were checked this session. WebFinger discovery and existing Funkwhale profiles require a separate compatibility review before implementation.

Do not enable anonymous outbound crawling or reuse browser session cookies for peers. Reuse the importer destination-policy concepts with explicit per-hop DNS pinning, public-address policy, no ambient proxies/credentials, bounded responses/redirects/deadlines and no recursive object fetching. Trusted-local two-instance test routing must be test-only, never an ordinary-user SSRF bypass.

Objects need canonical HTTPS origin plus stable origin object IDs, independent of local cache IDs. Cache remote metadata separately from owned tracks/assets. Process idempotent event IDs, reject foreign object updates, use durable bounded deliveries, and reauthorize playback at origin. Enabling federation must publish nothing until separately authorized publication records and NETWORK_RIGHTS acceptance exist.

Next executable network slice: opt-in stable identity and capabilities, explicit per-object rights/publication model, then actual two-instance discovery plus private negative controls. Inbox/signature/replay and lawful audio delivery are independent acceptance steps; do not advertise them early. Public directory/central service and private-history disclosure are prohibited defaults.
