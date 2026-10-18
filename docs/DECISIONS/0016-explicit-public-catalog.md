# ADR0016: explicit publication before social federation

Accepted2026-10-18 for0.11. This evolves the existing seven-crate modular monolith.

Private upload, listening and DSP pipelines already work independently. Making an
asset discoverable must never follow from importing it, enabling a server, sharing
a playlist, collecting history or possessing identical bytes on another account.

Store an additive, owner-bound publication ledger and stable server UUID. A server
administrator must explicitly enable a canonical HTTPS DNS origin; each owner must
separately declare one of three supported open licenses, attribution, private HTTPS
provenance and confirm rights. Metadata is public only for these records. Streaming
and download grants are separate, default false; download requires streaming.
Publication IDs differ from local track/asset IDs. Revocation retains the owner's
record, retires that public ID and reauthorizes every new GET/HEAD/range/download.
Original music, metadata, favorites, history and analysis remain unchanged.

The first network feature is an explicitly versioned, read-only public catalog
preview, not an ActivityPub implementation. ActivityPub remains the intended basis
for future actors/follows/events; WebFinger, signed inbox delivery, replay defense,
peer blocks and Funkwhale interoperability are unfinished. W3C ActivityStreams can
represent Audio, but an Audio object's metadata is not an audio access grant.
Implementing a small discovery profile before social delivery lets actual two-node
HTTP and rights-negative tests establish that boundary without claiming standards
compatibility or introducing an invented full federation transport.

Origin + publication ID identifies remote content; a peer-supplied UUID is not a
globally trusted identity. Production peers require normal HTTPS certificate
verification, a DNS hostname, port443 and root origin. Every request and redirect
pins a validated all-public DNS answer set; redirects stay on the original origin.
Preview admits2 concurrent operations, fetches2 bounded JSON documents under5s,
and never stores remote credentials or mirrors audio. A process-wide4-slot DNS
budget is held inside blocking resolver tasks until they actually finish, even
when callers time out. Existing import download redirect behavior is preserved.
Anonymous media has an8-stream sub-budget within the existing16-stream total;
body completion/cancellation releases both permits, preserving owner capacity.

Artwork and private profiles/history/provenance/paths/hashes are absent from the
public DTO. There is no anonymous account enumeration, write API, proxy, import,
automatic crawling or enabled directory. Preview links open an origin's explicitly
granted public media; this is not yet a unified cross-server queue or client.

License declarations are uploader assertions, not verified copyright provenance.
Disabling access cannot recall delivered bytes or revoke licenses already granted;
streaming permission cannot technically prevent a recipient saving those bytes.
The application must explain both constraints before the owner confirms a grant.

References checked: [ActivityPub](https://www.w3.org/TR/activitypub/),
[WebFinger](https://www.rfc-editor.org/rfc/rfc7033), and
[Creative Commons BY4.0](https://creativecommons.org/licenses/by/4.0/).
Exact executed acceptance belongs in QA_EVIDENCE/PROGRESS, not this design record.
