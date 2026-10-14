# ADR0002 — Public import transport has no trusted-local bypass

Accepted2026-10-13. All HTTP/S imports and redirects use syntax/address validation and per-hop pinned DNS, with proxies, ambient credentials, response decompression and automatic redirects disabled. Enforce bounded file/queue/time/disk usage and durable serial jobs. Uploaded and downloaded bytes enter one media validation/publication service.

Private/loopback/metadata/special endpoints cannot be supplied by ordinary users. A fixture hostname routing capability is compiled only under cfg(test) in the imports crate. Future administrator-trusted WebDAV/S3/storage integrations require a separate policy and credential model; they must not weaken the public direct source.

Tradeoffs: conservative destination policy and ports may reject legitimate unusual hosts. No browser-dependent CORS fetch, SSRF-capable unrestricted subprocess, arbitrary extraction utility or commercial DRM download. Preview avoids downloading whole media. Resumable ranges and collection adapters will need separate security/recovery acceptance tests.
