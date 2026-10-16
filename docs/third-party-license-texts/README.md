# Dependency notice provenance

These are actual upstream texts, preserved with their original bytes. Each cached text has a neighboring `.source.json` containing its immutable source URL and SHA256. `scripts/generate_notices.py` verifies those bytes before including a cached text in local review packages. Its normal offline path reads the resolved Cargo graph, shipped crate files and this cache. `--fetch-missing` performs explicit bounded public upstream reads at packaged VCS commits; it does not run during normal server use. No generic license template is substituted for a missing package-specific grant.

The0.4.0 review reduced metadata-only gaps from12 to6:

| Package | Actual source and applicability |
| --- | --- |
| crc-catalog2.5.0 | The shipped LICENSES/Apache-2.0.txt and LICENSES/MIT.txt are referenced by its README. The generator now discovers that directory. Exact commit [ed4ad631](https://github.com/akhilles/crc-catalog/tree/ed4ad631f22b05055c21a3a4127eb7cf6d75bb62/LICENSES). |
| r-efi5.3.0 and6.0.0 | The packaged AUTHORS files contain the full MIT grant/copyright and are explicitly identified by README. These exact versions are reviewed exceptions to conventional filename discovery: [5.3.0 AUTHORS](https://raw.githubusercontent.com/r-efi/r-efi/97b55bed1c2c91dcbf787674849f05337ff80b33/AUTHORS), [6.0.0 AUTHORS](https://raw.githubusercontent.com/r-efi/r-efi/7e1b0322d31d625f81a5656096330934f9cd835d/AUTHORS). |
| wasite0.1.0 | Three underscore-named files at its packaged commit [0c72934a](https://github.com/ardaku/wasite/tree/0c72934ad329ab4670eb017581640c2d6eceb289) were missed by conventional fetch names. Their exact bytes/provenance are cached here. |
| winapi-i686-pc-windows-gnu0.4.0 and winapi-x86_64-pc-windows-gnu0.4.0 | These old packages omit packaged VCS metadata. Every upstream-authored shipped file, including original manifests and import libraries, was matched to Git blob identities under [9497609e](https://github.com/retep998/winapi-rs/tree/9497609ef44cc9bcd16cd2411c0ee6ccaf5483aa), whose manifests specify those exact versions and license expression. Root MIT/Apache texts from that commit are cached for each package. |

The independent repeat of the winapi comparison matched1390 i686 files/50,930,577 bytes and1419 x86_64 files/53,157,228 bytes, with no unmatched or changed file. Only Cargo-generated normalized Cargo.toml and .cargo-ok were excluded; Cargo.toml.orig was compared with upstream Cargo.toml. [The report](winapi-provenance.json) retains pinned subtree IDs and counts. [The read-only helper](verify_winapi_provenance.py) reproduces the comparison using bounded GitHub tree requests and registry sources named by saved Cargo metadata:

```bash
cargo metadata --locked --offline --format-version=1 > target/checks/jobs-cargo-metadata.json
python3 docs/third-party-license-texts/verify_winapi_provenance.py --metadata target/checks/jobs-cargo-metadata.json
```

Git blob identity is SHA1 over `blob <byte-count>\0<file-bytes>`. This establishes content linkage; cache files independently use SHA256. It is not an external security/legal audit.

The remaining metadata-only packages are content_disposition0.4.0, sledgehammer_bindgen0.6.0, sledgehammer_bindgen_macro0.6.5, sledgehammer_utils0.3.1, warnings0.2.1 and warnings-macro0.2.0. Full exact-commit archives were inspected, including README/source files, without finding a standalone text or full inline grant. Their declared SPDX expressions remain in notices. [Pinned archive evidence](remaining-six-provenance.json) records URLs, hashes, VCS subdirectories and findings. Review those outstanding grants before external redistribution; this session neither contacted maintainers nor published a release. Package updates require renewed provenance review.
