# Public-network rights boundary

Status2026-10-17: design requirements only. There are no federated catalog/media endpoints and no public library publication. Existing uploads, owned original streams and direct authorized HTTP downloads remain the implemented paths. A URL being reachable is not a redistribution license.

Before publication exists, each published asset must record an explicit owner action, origin identity, rights basis, license identifier/URL, attribution, provenance, whether metadata/stream/download/republish are independently allowed, publication revision and revocation time. Unknown rights never default to public streaming. Private collections, listening events, taste profiles, source URLs and credentials must never enter peer payloads. Public-domain and open-license claims need retained source evidence; do not infer them from file tags or MIME.

Future authorizations must recheck current publication and peer/user policy at the origin. Metadata discovery does not authorize artwork, audio or downloads. A remote cache cannot become locally owned audio. Revocation prevents new authorized transfers and delivers bounded tombstones; it cannot erase bytes already delivered or guarantee deletion from remote administrators/backups. Block/mute and peer limits must apply independently of licensing.

Acceptance G11/G14/G15/G22 under the latest mandate needs two separate instances with a deliberately published licensed fixture subset, private negative controls, a malicious/slow third peer, signature/replay tests, removal and download-policy tests. None of these network scenarios is currently marked passing. Generated sine fixtures are lawful development inputs, not a shipped catalog or real-world license-verification service.
