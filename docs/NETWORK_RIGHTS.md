# Public-network rights boundary

0.11 adds explicit optional publication and read-only public catalog inspection.
It does not automatically license uploads, verify copyright ownership, implement
ActivityPub or make private music public. Public discovery is disabled by default.
No original user tracks are published during development or upgrade.

An administrator configures the canonical HTTPS origin and enables discovery.
Disabling discovery pauses public access without revoking individual records;
reenabling restores access only to still-active explicit publications. Owners can
revoke records even while discovery is disabled.

Each owner then selects an owned track, declares CC0-1.0, CC-BY-4.0 or
CC-BY-SA-4.0, supplies public attribution and private HTTPS source evidence, and
confirms their right to grant the selected permissions. All rights/stream/download
checkboxes start unchecked; changing the declaration clears confirmation. Evidence
is retained owner-only plain text and never fetched automatically. Unknown rights
have no public path. Rights to music do not imply rights to cover art: public
artwork is unsupported. Uploader declarations need independent verification before
any real-world legal conclusion. Private listening/favorites/DSP are never proof.

Metadata publication does not grant streaming or downloads. Download requires
streaming plus a separate opt-in. Actual public streaming means recipients receive
audio bytes and can save them; a disabled download button cannot prevent copying.
License grants already made may be irrevocable even if the server later revokes
access. Owner confirmation must explain that distinction. Files remain unchanged.

Every new public media/HEAD/range/download checks current server enablement,
nonrevoked publication and its actual grant. Foreign ownership, private playlists,
shared physical assets and knowledge of a local track ID grant nothing. Revoked
public IDs remain retired; republishing creates a new public ID. Already buffered
or delivered bytes cannot be recalled. No removal-delivery/tombstone protocol or
promise of deletion by remote administrators is implemented yet.

The public catalog deliberately excludes ownership IDs, paths, physical hashes,
private provenance/source URLs, artwork, private events and taste profiles. Remote
metadata is untrusted, bounded and escaped. HTTPS origin + public object ID gives
the content's origin; a peer UUID is not proof of a user's rights or trustworthiness.

Acceptance PN01/PN02 and actual two-node TCP tests exercise generated project
fixtures with negative private/metadata-only/download/revocation controls. They
cover subsets of latest-mandate G11/G14/G15/G22 only; signed social federation,
identity spoof/replay controls and public TLS interoperability require later work.
Generated sine fixtures are lawful test inputs, never a shipped public catalog.
Executed results and any failures are recorded in QA_EVIDENCE/PROGRESS.
