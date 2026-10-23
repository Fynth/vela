# Reproducible performance measurements

## DR-02 the glass frame budget (2026-10-22)

The two floating bands (the player and the topbar) are the only surfaces of
the product that carry a `backdrop-filter`. This entry is the local budget
for that filter: what the product costs when the catalog scrolls under the
two bands, against the same scroll with the opaque fallback.

**Stand.** Headless Chromium 156.0.8078.4 and Firefox on Linux x86_64,
viewport 1284×856, no display server. Chromium's compositor renderer is
`ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero)), SwiftShader
driver)` — **software rasterisation**; `/dev/dri` exists on this host but no
GPU-backed browser is available here, so **a GPU-backed stand is NOT_RUN**.
The account is real, the 24 tracks are real uploaded originals, and the page
is the real Library with the real shell.

**Method.** `tests/browser/dr02-frame-budget.mjs` launches the browser,
opens the Library and scrolls for ten seconds with the page's own animation
clock (a sine over the whole scroll range, so the backdrop under both bands
changes on every frame). It then repeats the same ten seconds with the
**opaque** roles painted on the two bands — `var(--vela-player-bg)` on the
player and `var(--vela-bg)` on the topbar, `backdrop-filter: none` and no
shadow — in the same viewport, on the same page, in the same browser. Each
run records the computed background, filter and shadow it really painted,
and the gate asserts that the two runs differ: a fallback that removed only
the blur would keep the 0.851 background and compare the glass with itself.
Measured: the glass run paints `rgba(19, 21, 19, 0.85)` with `blur(16px)` /
`blur(12px)`; the fallback run paints `rgb(16, 19, 15)` and `rgb(19, 21, 19)`
with `filter: none` and `shadow: none`.

**The measurement runs in a process of its own, and that matters.** The
Playwright test runner attaches to every browser it starts: its client, its
trace agent and its own machinery sit next to the renderer while a
measurement runs. On this host the *same* page, browser, driver, server and
session read (diagnostic run of 2026-10-22, `perf-diagnostics` /
`target/checks/dr02-glass/perf-diagnostics.json`):

| Where the measurement runs | glass: frames > 33ms | fallback: frames > 33ms | difference |
| --- | ---: | ---: | ---: |
| A plain process (the product's own environment) | 0 of 602 (0.00 %) | 0 of 602 (0.00 %) | 0.00 points |
| Inside the Playwright test runner | 96–122 of ~500 (15.7–24.4 %) | 0 of 602 (0.00 %) | 15.7–24.4 points |

The two rows are the same code, the same page (the same 1321px scroll range,
the same 888 elements, the same `blur(16px)` on the player and `blur(12px)`
on the topbar) and the same server; only the process that owns the browser
differs. **The runner row was taken before the fallback was opaque** — its
second column is the filter-only variant, so the 15.7–24.4 points are the
harness cost of the glass run, not a measured glass-vs-opaque delta. The
runner's cost is not the product's, so the budget is measured in the plain
process and the permanent gate (`tests/browser/dr02-glass.spec.mjs`) runs it
there, against the opaque roles.

**Result (the gate's own numbers, 2026-10-22, five runs of the same gate,
`target/checks/dr02-glass/budget-runs.json`).** Every run recorded the two
surfaces it painted (glass `rgba(19, 21, 19, 0.85)` + `blur(16px)`/`blur(12px)`
+ shadow vs fallback `rgb(16, 19, 15)`/`rgb(19, 21, 19)`, `filter: none`,
`shadow: none`), so the comparison is glass against opaque in all five.

| Run | Engine | Glass: frames / over 33 ms | Opaque: frames / over 33 ms | Difference |
| --- | --- | ---: | ---: | ---: |
| DR-02 spec alone | Chromium | 598 / 3 (0.50 %) | 602 / 0 | **+0.50 points** |
| DR-02 spec alone | Firefox | 602 / 0 | 603 / 0 | 0.00 points |
| whole suite | Chromium | 601 / 0 | 602 / 0 | 0.00 points |
| whole suite | Firefox | 601 / 0 | 602 / 0 | 0.00 points |
| whole suite (final) | Chromium | 602 / 0 | 602 / 0 | 0.00 points |

The median frame is 16.7 ms in Chromium and 17.06 ms in Firefox in both
modes; the longest frame is 16.8 ms everywhere except the one Chromium glass
run (33.4 ms, three frames). So the honest reading is **0.00 to +0.50 points
across five runs**, inside the 5-point budget the DR-02 plan sets — and the
single three-frame sample is not reproducible: three later runs of the same
gate read zero. On this stand the filter does not set the cadence of the
scroll; a single ten-second sample cannot resolve a half-point difference
from the software rasteriser's own noise, and this document does not claim
it does.

**What this is not.** It is a local budget on a software rasteriser, not a
promise about any other device: the number says the two glass bands cost at
most half a point here, and it says nothing about a GPU-backed
browser, a phone, a laptop on battery, or a machine whose raster threads
are busy. A GPU-backed stand is NOT_RUN on this host. The opaque fallback
stays implemented and reachable (`prefers-reduced-transparency: reduce`,
`forced-colors: active`, `print`, and any engine without
`backdrop-filter`), so a reader who cannot afford the blur gets the opaque
band.

**A diagnostic, not a gate.** The same ten-second scroll was run with the
two bands blurred, with no blur at all, with only the player blurred, with
only the topbar blurred and with both blurred at 4px and at 8px
(`target/checks/dr02-glass/perf-diagnostics.json`): all six read 602 frames,
a 16.7 ms median, a 16.8 ms longest frame and **0 %** frames over 33 ms.
That run also used the filter-only fallback, so it compares blur radii and
not glass against opaque. The blur radius is therefore not what costs
anything on this stand — the whole filter is free here — which is why the
shipped radii (16px for the desktop player, 12px for the topbar and the
mobile player) are the ones the DR-01 roles declare.

**Reproduce.**

```bash
cd tests/browser
npx playwright test dr02-glass.spec.mjs --grep "frame budget"            # chromium
npx playwright test dr02-glass.spec.mjs --grep "frame budget" --browser=firefox
# the raw numbers: target/checks/dr02-glass/<engine>/performance.json
```

## Distribution pipeline 2026-10-20

Distribution 0.12.8 limits Cargo builds to two jobs per runner and reuses Cargo caches, including after downstream failures. Browser quality uses the debug server; package verification independently starts the optimized extracted server. A pinned prebuilt wasm-bindgen avoids compiling a second CLI graph. This is build resource policy, with no new playback/startup benchmark claim. AppImage uses host GTK/WebKit libraries. Published byte sizes are recorded in the [release ledger](RELEASE_LEDGER_0.12.8.md); see [DISTRIBUTION](DISTRIBUTION.md).

## Personal recommendation query/ranking0.10 (2026-10-18)

Actual release example `target/release/examples/benchmark_recommendations --tracks N --iterations 5`, N=10000/100000/1000000. Same AMD Ryzen7 8845HS/16logicalCPU, Linux x86_64/CachyOS7.2.10-1/glibc2.44, approximately14.9GiB RAM, Rust1.99.0. Each workload creates its own temporary real migrated SQLite/WAL store with synthetic metadata only:64artists/eight genre tags/10tracks per album,12favorites, at most50 qualified sessions over20tracks,20exclusions. No audio or user library is accessed. Seeding, JSON serialization, HTTP/auth/UI, network and physical playback are excluded from per-call latency. One warmup+five sequential measurements; filesystem caches warm, normal browser/build activity may overlap. Different-date before/after runs are not dedicated isolated-host experiments.

| Synthetic tracks | Case | Baseline p50 ms | Final p50 ms | Final p95 ms | Final p99 ms |
| ---: | --- | ---: | ---: | ---: | ---: |
| 10,000 | cold-library-exploration | 14.563 | 4.840 | 5.982 | 5.982 |
| 10,000 | explicit-favorite-affinity | 16.267 | 6.343 | 7.114 | 7.114 |
| 10,000 | qualified-private-history-and-exclusions | 44.756 | 6.844 | 8.105 | 8.105 |
| 100,000 | cold-library-exploration | 104.441 | 5.934 | 8.562 | 8.562 |
| 100,000 | explicit-favorite-affinity | 109.345 | 7.409 | 9.196 | 9.196 |
| 100,000 | qualified-private-history-and-exclusions | 1200.184 | 8.149 | 8.951 | 8.951 |
| 1,000,000 | cold-library-exploration | 987.225 | 5.169 | 6.099 | 6.099 |
| 1,000,000 | explicit-favorite-affinity | 1048.412 | 7.123 | 7.783 | 7.783 |
| 1,000,000 | qualified-private-history-and-exclusions | 10995.440 | 7.282 | 7.882 | 7.882 |

Five samples cannot establish tail confidence; nearest-rank p95/p99 equal the maximum sample. Every case passed candidate<=500/anchors<=12, owner/hidden/qualified eligibility, actual fixture rotation, stable fixed-input IDs and diversity checks. Output30/500 is6% examined-window selection coverage; this is structural coverage, not whole-library/user satisfaction/play-through quality. Cold output covers30artists; favorite/history output18artists in30 suggestions. Fixed unchanged inputs deliberately repeat stable mixes; Refresh is not random regeneration.

Source-derived EXPLAIN identified baseline catalog scans while favorites/history were sparse. The final source selects eligible bounded IDs from favorites/qualified history first, hydrates only those IDs with the existing Track/descriptor projection, restores exact source order and reuses rotation evidence. All four selector IDs and full projections match the preserved baseline in the isolated10k equality fixture. Additional Rust guards verify keyed seeks and hydration order/authorization. No schema/index/weights/algorithm-version or rights-policy change was made for this optimization. Large equal-added_at groups (mixed index tie direction) and10000 retained sessions remain unmeasured.

Whole example process resource measurements include seeding and JSON output, not just ranking:

- 10,000: 0.705s wall, 0.499s user, 0.229s system, peak 17308KiB RSS.
- 100,000: 10.633s wall, 5.155s user, 5.384s system, peak 21164KiB RSS.
- 1,000,000: 148.208s wall, 74.551s user, 72.568s system, peak 21164KiB RSS.

Captured per child with os.wait4; not server RSS or streaming capacity. Raw before/after JSON, stderr and resources: target/checks/personal-{final-,}benchmark-*.json. Baseline exact source copies + SHA256 are personal-baseline-{recommendations,discovery}.rs and personal-benchmark-baseline-hashes.json. Final source/example/lock/binary SHA256: personal-final-benchmark-resources.json. Plans/equivalence: personal-explain.py/.json; it uses Python SQLite3.54, while actual performance runs the compiled SQLx service. Final binary hash: `9df06bc2f626d0f9ecf5dca482220ddd74ba7f6f015836819278fdc5fc8ffc7b`. No claim of superiority over another music application, recommendation relevance, native audio or real-time federation follows from these figures.

## Local PCM extractor measurement0.9 (2026-10-17)

Actual release example, Rust1.99.0, Symphonia0.6.1/RustFFT6.4.1, Linux x86_64/CachyOS7.2.10-1/glibc2.44, AMD Ryzen7 8845HS (16logical CPUs), approximately14.9GiB RAM. One serial extractor, five measured samples plus one untimed warmup per original generated WAV. Fixture creation, BLAKE3 verification and output serialization are excluded from per-extraction latency. Filesystem caches are warm, other development/browser/compiler activity may be present. No global cache drop or dedicated benchmark isolation was performed.

```bash
CARGO_BUILD_JOBS=2 cargo build --locked --release -p vela-media --example benchmark_analysis
target/release/examples/benchmark_analysis --generate --iterations 5
```

| Actual generated PCM source | p50 ms | p95 ms | p99 ms | Successful / failed |
| --- | ---: | ---: | ---: | ---: |
| generated-1s-mono-44100.wav | 0.949 | 0.952 | 0.952 | 5 / 0 |
| generated-20s-mono-48000.wav | 19.099 | 19.308 | 19.308 | 5 / 0 |
| generated-20s-8ch-48000.wav | 132.973 | 134.217 | 134.217 | 5 / 0 |
| generated-120s-mono-48000.wav | 114.081 | 115.283 | 115.283 | 5 / 0 |

All20 measured runs passed descriptor sanity, consistent output and unchanged source hashes. Five samples cannot establish reliable tail percentiles: nearest-rank p95/p99 are simply the maximum measured sample. The120s fixture may be reported as bounded-prefix coverage when the duration ceiling is reached before an EOF read; the result carries that distinction.

A separate Python subprocess/resource.getrusage wrapper measured the entire example process (including generation/warmups/hashing/JSON): 1.811s wall, 1.756s user CPU, 0.047s kernel CPU, peak RSS13164KiB on this Linux host. `/usr/bin/time` was absent; no successful time(1) invocation is claimed. Raw results: target/checks/listening-dsp-benchmark.json and listening-dsp-benchmark-resources.json. Example SHA256: `bc0a2ef3422cb58c1f90ebdff84d26aeb6fe38b9bf0347afbdde5d3830818ae5`; measured binary SHA256: `8799469aef90da79f7f7b1cb9340e28891a0f12c7a5d2ca0527f4678748b11ea`.

Measured engine source SHA256: `fff266b58cc8898f2b9037e9a004f28ec2bc3dce9a357decd5f9c8bc795ffae4`; Cargo.lock SHA256: `79615fff13ddebb74840cc1e7c83ad3892d6b55820884b682975bb47eb05bcf3`.

This measures sequential offline PCM analysis, not server RSS, compressed-codec throughput, audible playback latency, native engines, simultaneous streaming/import contention, radio relevance or superiority over another product. Current single-worker admission remains unchanged; no optimization was justified by these measurements. Existing catalog/job measurements below retain their historical source/hardware scope.


## Experience0.5 verification scope

No new latency/CPU/throughput benchmark is claimed for the0.5 UX slice. The new local aggregate search bounds each SQL result category at8 and cancels obsolete browser requests. Its album/artist/playlist substring predicates are not constant-time/indexed search guarantees and have not been measured at1M groups. Existing measurements below apply only to their recorded query/source scope. Real Chromium checks establish decoded audio/seek and unchanged active media node/source through navigation, not audible-start timing or gapless accuracy.20 production-process clean restarts plus independent worker-failure injections passed; they establish lifecycle regression coverage, not concurrent-user capacity.

Measured at 2026-10-13T19:02:43.194711+00:00 using `scripts/benchmark_catalog.py`.

These measurements exercise SQLite catalog queries over synthetic metadata. They are not HTTP latency, streaming throughput, audible playback time, or a comparison with other products.

## Hardware and environment

- CPU: AMD Ryzen 7 8845HS w/ Radeon 780M Graphics; 16 logical CPUs.
- Host RAM: 14.92 GiB.
- OS: `Linux-7.2.9-2-cachyos-x86_64-with-glibc2.44`.
- Python: 3.14.8; Python SQLite: 3.54.0.
- Isolated benchmark storage root: `/home/rasul/Projects/vela/target/catalog-bench`.
- Storage type and concurrent host activity are recorded below after the run; the script does not drop host filesystem caches.

## Method

The script applies the repository migrations to a fresh private database for each workload. One owner receives the indicated track count, with one physical-asset metadata row per track, deterministic UUIDv4 keys, 1,000 artist labels, albums of 12 entries, five genres and 5% favorites. No audio files are created. FTS5 triggers build the real search index during inserts. Seed time includes batched metadata writes and index maintenance, but is not media-ingestion throughput.

The baseline queries use the media crate's Track projection and its original owner/FTS/keyset/favorite ordering strategy. The adaptive sparse-search optimization below now changes rare/absent query execution; the baseline harness intentionally preserves the original strategy for comparisons. A page requests 50 entries plus one sentinel. Timing includes Python `execute` and full `fetchall` of all projected columns, but excludes connection creation, HTTP, authentication, JSON, Rust SQLx, network transfer and UI rendering. The Python SQLite version may differ from the server's SQLx-linked SQLite version.

SQLite uses WAL, synchronous FULL, a 2000 KiB page-cache target and disabled mmap. No ANALYZE is run. Warm measurements follow 10 untimed executions on the same connection. Cache-reset measurements use a fresh SQLite connection each time; the filesystem cache remains warm. They therefore do not establish disk-cold latency. Percentiles use nearest-rank selection; p99 from a small sample count is approximate.

## Results

| Tracks | Query | Warm p50 / p95 / p99 ms | Cache-reset p50 / p95 / p99 ms | Rows fetched |
| ---: | --- | ---: | ---: | ---: |
| 10,000 | first_page | 0.084 / 0.094 / 0.112 | 0.206 / 0.358 / 0.358 | 51 |
| 10,000 | keyset_midpoint | 0.087 / 0.094 / 0.102 | 0.208 / 0.262 / 0.262 | 51 |
| 10,000 | favorites | 0.766 / 0.814 / 0.816 | 1.250 / 1.687 / 1.687 | 51 |
| 10,000 | fts_rare | 0.458 / 0.480 / 0.490 | 0.685 / 0.822 / 0.822 | 1 |
| 10,000 | fts_broad | 0.222 / 0.232 / 0.240 | 0.523 / 0.586 / 0.586 | 51 |
| 10,000 | fts_no_match | 0.466 / 0.490 / 0.496 | 0.683 / 0.765 / 0.765 | 0 |
| 100,000 | first_page | 0.093 / 0.098 / 0.101 | 0.263 / 0.307 / 0.307 | 51 |
| 100,000 | keyset_midpoint | 0.096 / 0.103 / 0.123 | 0.255 / 0.827 / 0.827 | 51 |
| 100,000 | favorites | 1.401 / 1.590 / 1.636 | 1.570 / 2.277 / 2.277 | 51 |
| 100,000 | fts_rare | 7.123 / 8.356 / 8.789 | 8.177 / 9.162 / 9.162 | 1 |
| 100,000 | fts_broad | 1.184 / 1.211 / 1.246 | 1.432 / 1.551 / 1.551 | 51 |
| 100,000 | fts_no_match | 7.490 / 8.636 / 9.221 | 8.040 / 9.416 / 9.416 | 0 |
| 1,000,000 | first_page | 0.098 / 0.135 / 0.175 | 0.378 / 0.761 / 0.761 | 51 |
| 1,000,000 | keyset_midpoint | 0.101 / 0.156 / 0.194 | 0.433 / 0.673 / 0.673 | 51 |
| 1,000,000 | favorites | 1.469 / 1.740 / 2.016 | 1.781 / 1.968 / 1.968 | 51 |
| 1,000,000 | fts_rare | 75.354 / 81.607 / 82.074 | 73.683 / 81.622 / 81.622 | 1 |
| 1,000,000 | fts_broad | 10.766 / 11.089 / 11.612 | 11.146 / 11.518 / 11.518 | 51 |
| 1,000,000 | fts_no_match | 74.436 / 79.959 / 83.232 | 72.418 / 79.994 / 79.994 | 0 |

Each query has 100 warm samples and 10 cache-reset samples.

`fts_rare` matches one UniqueNeedle title; `fts_broad` matches approximately 10% of titles containing Signal; `fts_no_match` has zero matches. `keyset_midpoint` starts after the median UUID, and `favorites` selects the 5% favorite subset.

| Tracks | Seed seconds | Database MiB |
| ---: | ---: | ---: |
| 10,000 | 0.72 | 8.74 |
| 100,000 | 35.24 | 86.52 |
| 1,000,000 | 381.46 | 860.17 |

## Query plans

Plans below come from the 1,000,000-track database. Full SQL, parameters, all workload plans and raw timing samples are emitted in the JSON report.

### first_page

```text
SEARCH t USING INDEX tracks_owner_cursor (owner_id=?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### keyset_midpoint

```text
SEARCH t USING INDEX tracks_owner_cursor (owner_id=? AND id>?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### favorites

```text
SEARCH t USING INDEX tracks_owner_cursor (owner_id=?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### fts_rare

```text
SEARCH t USING INDEX tracks_owner_cursor (owner_id=?)
LIST SUBQUERY 1
SCAN tracks_fts VIRTUAL TABLE INDEX 0:M5
CREATE BLOOM FILTER
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### fts_broad

```text
SEARCH t USING INDEX tracks_owner_cursor (owner_id=?)
LIST SUBQUERY 1
SCAN tracks_fts VIRTUAL TABLE INDEX 0:M5
CREATE BLOOM FILTER
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### fts_no_match

```text
SEARCH t USING INDEX tracks_owner_cursor (owner_id=?)
LIST SUBQUERY 1
SCAN tracks_fts VIRTUAL TABLE INDEX 0:M5
CREATE BLOOM FILTER
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

## Reproduce

```bash
python3 scripts/benchmark_catalog.py --sizes 10000 100000 1000000 \
  --samples 100 --cold-samples 10 \
  --tmp-root target/catalog-bench \
  --output /tmp/vela-catalog-benchmark.json --write-doc docs/PERFORMANCE.md
```

The script creates and removes only its own temporary workload directories. It never opens a production database. At least roughly 1.8 GB of free storage is required for the million-entry workload, in addition to the rest of the host's workloads.

## Limits and next measurements

Single-process serial SQL results on this host do not establish VPS or ARM capacity. Search distribution, many owners, concurrent writes, a small OS cache and slower disks can change the results substantially. These are exploratory local measurements; no superiority or supported-listener-count claim follows from them.

Still unmeasured: HTTP p50/p95/p99 under concurrency, streaming time to first byte, time to audible playback, peak server RSS, import/scanner concurrency, transcode CPU, remote search, federation and native playback. Repeat representative load tests on an idle host and constrained VPS/ARM hardware before claiming production capacity.

Source fingerprints for this run:

```json
{
  "migrations/0001_initial.sql": "1f18d72a55b33f5b1188df7228960f75ae3d6c4523ca4c0d3feb81f713cb5360",
  "crates/media/src/lib.rs": "28b5db2859a7f6b460626b4dc4eba13030d2250593dc95ed63f3362ecdc8352a"
}
```

## Measured sparse-search optimization

After observing the owner-index scan for rare/absent matches, `list_tracks` now probes at most4097 FTS rowids. With zero matches it returns immediately. With at most4096 matches it drives a CROSS JOIN from FTS rowids, preserving the owner filter and UUID cursor/order. Broader searches retain the original owner-index plan. No search term changes permission checks.

`python3 scripts/benchmark_search.py --tracks 100000` seeded one isolated100k database on the same host/storage and compared baseline/adaptive plans over100 warm runs. Timing includes the cardinality probe and full row projection. Results were checked for exact equality. This is SQL/Python timing, not HTTP or native-SQLx latency.

| Query | Baseline p50/p95/p99 ms | Adaptive p50/p95/p99 ms |
| --- | ---: | ---: |
| Rare match | 7.381 / 8.466 / 8.657 | 0.098 / 0.117 / 0.121 |
| No matches | 7.544 / 8.178 / 8.476 | 0.014 / 0.015 / 0.020 |
| Common prefix (10% matches) | 1.277 / 1.382 / 1.419 | 1.735 / 2.041 / 2.145 |

The probe adds approximately0.46ms at the median for this common-prefix distribution. The tradeoff removes a catalog-wide scan for sparse/no-result searches while preserving a bounded probe and the existing broad plan. Million-entry results above describe the original baseline; post-optimization million-entry timings have not been measured. Raw comparison data: `docs/benchmarks/search-100k.json`. Recheck with multiple owners and concurrent writes before changing the threshold.

## Scanner resource boundary (0.2.0)

The implemented scanner has one serial worker and shares the media parser semaphore with imports/uploads. Traversal holds at most32 directory iterators by default, yields after256 skipped entries, and copies at most one source through a64KiB buffer before bounded media processing. Durable per-source fingerprint maps cap at100k admitted files by default; entry passes cap at1M. These are implementation limits verified by regression tests, not throughput/memory measurements. End-to-end scan rate, concurrent import/stream latency and peak RSS remain unmeasured; no capacity or performance superiority claim follows from the limits.

## Typed library filters and covering genre facets (0.3.0)

Reproducible Python/SQLite measurements now cover10k+10k and100k+100k owner/private synthetic metadata rows: typed predicates, both FTS branches, first/next/midpoint keyset pages, coherent facets, query plans and index build/storage cost. The host is Ryzen7 8845HS/16 logical CPUs,16GB RAM, Linux; Python3.14.8 SQLite3.54.0,2MiB connection cache target, WAL/FULL, mmap disabled. This uses a SQL mirror and generated metadata, not Rust SQLx/HTTP/decoded audio.100 warm samples plus10 new-connection samples follow warmup; filesystem cache is retained. Another project's build was present during the early baseline workload, so this is not an isolated-host capacity result. Complete methods, fingerprints, plans and raw samples are retained in [baseline report](benchmarks/catalog-filters-before-covering.md), [baseline JSON](benchmarks/catalog-filters-before-covering.json), [final report](benchmarks/catalog-filters-0.3.0.md) and [final JSON](benchmarks/catalog-filters-0.3.0.json).

Independent review found that putting explicit NOCASE on the title tuple's left expression prevented a full tuple seek. Equivalent RHS collation `(title,id)>(? COLLATE NOCASE,?)` preserves case/tie results and yields the expected `(owner_id,title,id)` seek. Real SQLite pagination tests cover multiple sizes/mixed-case ties; the final100k-owner midpoint p50 is0.121ms and next page0.132ms for this SQL fixture. This does not measure API latency or worst-case contention.

The first grouped genre facets measurement was156.088ms p50/174.301ms p95 for100k owner tracks plus100k private distractors. Its expression index still fetched full table rows for canonical MIN tags. Migration0006 appends raw genre after the unchanged owner/expression/UUID prefix. The same facet queries now use a covering index and measured22.614ms p50/26.863ms p95 in the final fixture. Exact canonical tag results and private boundaries remain unchanged. The five new B-trees occupy76.37MiB versus75.07MiB before covering across200k total tracks; migration0006 took0.662s and grew the checkpointed database by about1.30MiB in this run. These fixture measurements and SQL planner evidence support this local optimization; no native/API speedup is claimed.

Remaining measured costs: final year-range first page p50~98.9ms and an empty combined-predicate page~tens of ms; SQLite chooses a wide year range and temporary UUID sort before full Track projection. Forcing an index can regress rare/empty distributions, so no hint was added. Profile actual native SQLx/HTTP before considering bounded-ID selection followed by projection or planner-statistics maintenance. Index build can delay startup on real catalogs, and maintaining five additional indexes adds unmeasured ingestion/write cost.

Reproduce the final mode with the command in the final report; add `--baseline-indexes` to retain only0005's index definition. Databases are temporary and never connect to existing user storage. Full benchmark mode consumes time/disk for metadata/FTS and does not ship synthetic catalog content.

## Native task-history HTTP (0.4.0)

The job center reads at most101 projected rows per branch and merges at most202 before returning at most100 (50 in the UI). Migration0007 supplies owner/history and owner/status/history indexes; actual migrated SQLite query-plan tests check history seeks and active-source lookups. These are implementation bounds, not a general constant-latency guarantee.

The new stdlib `scripts/benchmark_jobs.py` starts the actual release server in fresh private fixtures, creates real accounts, stops it before seeding explicit synthetic terminal-only job metadata, then restarts and measures authenticated loopback HTTP. At10k and100k owned mixed import/scan histories plus equal foreign histories, nine cases cover first/middle/deep pages, kinds and terminal filters. All1,800 measured responses matched exactly50 owned reference rows, cursor and action flags. No audio, downloads, scans or existing user database were involved. Normal per-request session validation is inside the timer; initial setup/password login is excluded.

On this Ryzen7 8845HS shared host, the100k+100k fixture's nine cases measured p50=0.305–0.504ms, p95=0.533–0.866ms and p99=0.601–1.881ms. These are serial warm HTTP/1.1 measurements with100 samples/case, retained caches and possible browser/restart workload overlap. The native SQLite writer was3.46.0; the Python seeder used3.54.0. No audio throughput, concurrent listener capacity, browser rendering speed or superiority claim follows. Full hardware/methods, source/binary/script hashes and raw samples are retained in [report](benchmarks/jobs-0.4.0.md) and [JSON](benchmarks/jobs-0.4.0.json). The script's documented command reproduces the complete isolated run.

## Playback-order verification0.6

The new pure order model bounds queue/deck/priorities and traversal history at1000 occurrences. Unit permutation/10000-step checks establish correctness and bounds, not audio CPU or latency measurements. No additional performance claim follows from the shuffle refactor; full-library and audio-start/seek profiling remain outstanding.

## Browser bookmarks/recovery0.7

Snapshots cap serialized output at4MiB and queue size at1000. Media time updates serialize no more than once per5s; explicit edits/seek/pause/lifecycle checkpoints flush immediately. Recovery has three delays,10s HEAD deadlines and an8s watchdog. These are resource-policy bounds, not measured startup/seek/CPU/FPS claims. Browser tests exercise actual decode and position tolerances, not physical audible start latency or playback superiority. Large-queue serialization and cross-browser profiling remain outstanding.

## Now Playing0.8 measurement scope

The view projects existing root state; it issues no new catalog/stream request on open and materializes at most five preview entries from the bounded queue. NP01 checks actual audio-node/source/generation continuity and zero additional stream requests on open. This is a correctness/resource invariant, not a playback-latency or frame-rate benchmark. No new p50/p95/p99, gapless, CPU or memory performance claim is made for0.8; use the reproducible catalog benchmarks above for their stated SQL-only scope.

The Firefox0.8 follow-up is cross-engine functional validation, not a latency/throughput benchmark. Network fixtures explicitly release the paused test buffer before forcing a cold-source outage, preserve the real bookmark and require failed GETs, an authorized HEAD and a new application source generation. This is separate from production cache behavior and does not measure offline hit rate.


## Public catalog0.11 resource bounds

Public pages default20/max50 and public peer preview reads only20 records; retained
publication quotas1000/owner and10000/global bound ledger growth. Preview admits2
operations under5s and bounds bodies16KiB/128KiB. Four global DNS permits live through
actual blocking completion, not merely the canceled request lifetime. Public media
has an8-transfer sub-budget within16 total and uses the existing64KiB file buffer.
Cancellation/permit and bounded parser tests establish policy, not p50/p95/p99,
concurrent listener capacity or Internet latency. No new performance-superiority,
audio-quality, native/background or large public-network measurement is claimed.

## Phase 0 baseline: 100k metadata/FTS and isolated HTTP reads (2026-10-18)

New reproducible slices for the resilience/scale plan, executed on the same host
(AMD Ryzen7 8845HS, 16 logical CPUs, ~14.9GiB, Linux x86_64/CachyOS7.2.10-1,
glibc2.44, Rust1.99.0, SQLite3.54.0). All databases are private temporary
directories created and removed by the scripts; the live database is never opened.

Commands:

```bash
python3 scripts/benchmark_catalog.py --sizes 100000 --samples 5 \
  --output target/checks/phase0/catalog-100k.json
python3 scripts/benchmark_search.py --tracks 100000 \
  --output target/checks/phase0/search-100k.json
python3 scripts/benchmark_http.py --threads 8 --duration 5 --tracks 20 \
  --output target/checks/phase0/http-bench.json
```

### 100k metadata/FTS (warm, 5 samples; search 100 samples)

| Query | p50 | p95 |
|---|---|---|
| first page (keyset owner) | 0.100 ms | 0.101 ms |
| keyset midpoint | 0.102 ms | — |
| favorites page | 1.53 ms | 1.71 ms |
| FTS rare (baseline) | 7.75 ms | 8.50 ms |
| FTS rare (adaptive) | 0.100 ms | 0.108 ms |
| FTS broad (baseline) | 1.32 ms | 1.39 ms |
| FTS broad (adaptive) | 1.69 ms | 1.75 ms |
| FTS absent (baseline) | 7.97 ms | 8.45 ms |
| FTS absent (adaptive) | 0.015 ms | 0.016 ms |

Database: 180,101,120 bytes for 100k tracks, seed 34.1s. Raw reports:
`target/checks/phase0/catalog-100k.json`, `search-100k.json` (source hashes and
plans included).

### Isolated HTTP read path (`scripts/benchmark_http.py`)

Fresh server, temporary data dir, throwaway setup account, 20 synthetic 0.5s WAV
uploads, 8 Python client threads, 5s per endpoint. **Python is the bottleneck**, so
these are client-bound lower bounds, not server capacity claims; no audio is
streamed.

| Endpoint | RPS | p50 | p95 | p99 | Errors |
|---|---|---|---|---|---|
| /health | 5,978 | 1.26 ms | 2.18 ms | 2.87 ms | 0 |
| /api/me | 6,056 | 1.26 ms | 2.11 ms | 2.64 ms | 0 |
| /api/tracks?limit=50 | 4,691 | 1.61 ms | 2.66 ms | 3.42 ms | 0 |
| /api/search?q=Bench | 1,976 | 2.14 ms | 18.49 ms | 30.09 ms | 0 |
| /api/albums | 855 | 7.94 ms | 20.88 ms | 31.41 ms | 0 |

Findings to attack next: album/artist grouping is the slowest read group even at
20 tracks (p95 20.9ms), and search p95 is 8.6x its p50 under Python contention.
Both need a server-side profile before changing indexes/queries. The HTTP harness
must be replaced by a native load generator (wrk/k6 or a Rust client) before any
10k RPS claim; Python client CPU saturates first.

### 100k-track isolated HTTP reads with a real restart (2026-10-19)

`scripts/benchmark_http.py` now has `--seed-tracks N` (0 keeps the upload-only
behavior). One isolated server lifetime runs the real migrations and the real setup
API, is stopped gracefully, then N metadata-only `media_assets`+`tracks` rows owned
by that account are inserted with sqlite3 in 1,000-row transactions (no audio files,
no favorites, deterministic UUIDv4/content-hash/title shape from
`benchmark_catalog.seed_database`). The server restarts on the same port and data
directory and the saved pre-restart cookie must still authenticate. `/api/stats` and
`/api/artists?limit=50` are probed serially with a long timeout instead of inside the
concurrent loop, because their real cost exceeds the concurrent client timeout.

```bash
python3 scripts/benchmark_http.py --threads 8 --duration 5 --tracks 20 \
  --seed-tracks 100000 --output target/checks/phase0/http-bench-100k.json
```

20 real WAV uploads + 100,000 seeded rows (100,020 owner tracks, 8,334 albums,
1,001 artists). Seed 39.87s insert + 0.40s verification, database 199,798,784 bytes.
`_sqlx_migrations`=0012, native SQLite writer 3.46.0, FTS5 integrity-check ok,
`PRAGMA quick_check` ok, no foreign-key violations. The pre-restart cookie
authenticated `/api/me` after restart with the same account id (anonymous `/api/me`
= 401), `/api/stats` reported 100,020 owner tracks, and both server stops exited 0.
The temporary tree and database were removed; whole run 2m37s. Recorded-run hashes:
script `e2afea5b35321ae63e7bcb35811ba1eb25eec29cd53a3590df101534f25c9fbe`, measured
binary `b82f45156c7241a616714654b00342e414f339016ba2ab699ff6a47211bb2706`
(`target/release/vela-server`, unchanged by this work).

| Endpoint | RPS | p50 | p95 | p99 | Errors |
|---|---|---|---|---|---|
| /health | 6,236 | 1.23 ms | 2.01 ms | 2.45 ms | 0 |
| /api/me | 6,338 | 1.21 ms | 1.95 ms | 2.36 ms | 0 |
| /api/tracks?limit=50 | 2,699 | 2.90 ms | 3.85 ms | 4.46 ms | 0 |
| /api/search?q=Signal | 56 | 152.91 ms | 207.83 ms | 249.79 ms | 0 |
| /api/albums | 1,241 | 6.10 ms | 9.47 ms | 13.29 ms | 0 |
| /api/stats (1 thread, 2 samples) | — | 662.78 ms | — | — | 0 |
| /api/artists?limit=50 (1 thread, 2 samples) | — | 44,783 ms | — | — | 0 |

`q=Signal` matches the 10% seeded titles, so search takes the broad (non-sparse)
branch: the capped 4,097-row cardinality probe, then the `rowid IN (SELECT … MATCH)`
membership branch over 10,000 matching rows, plus the unindexed album/artist
substring scans. Seeded rows are metadata only; no audio is streamed, and
cheap-endpoint RPS remains Python-client-bound. The host was shared and busy during
this run (other agents' rustc builds, a `vela-media` test binary using ~14 CPUs and
a concurrent grouping benchmark; load average 5.9-11.1 on 16 logical CPUs) and
`/tmp` is tmpfs, so absolute latency here
is not an idle-host capacity figure. Two findings to profile next: broad search is
~25x slower than the album page at 100k, and `/api/artists` (album/track/artwork
subqueries over the `ARTIST_TRACKS` union) took 44.8s for this page (51.7s and
58.4s in two earlier probe measurements on the same fixture); `/api/stats`
performs catalog-wide `COUNT`/`SUM` plus group visibility `EXISTS` subqueries.


### Artist grouping fix (2026-10-19)

`crates/media/src/catalog.rs` now pins the artist-track union arms: the direct arm
uses `tracks_catalog_artist`, the album-credit arm `albums b CROSS JOIN tracks t ...`
so SQLite starts from `albums_catalog_artist` instead of scanning the owner's 100k
tracks per artist. On the same 100k temp DB the page query went from ~53,000ms to
~22ms (SQL) and the release HTTP endpoint from 44-58s to **29.8ms**
(`target/checks/phase0/http-bench-100k-fixed3.json`). `cargo test -p vela-media`
passes (63+6+8). `/api/stats` dropped from **665ms to 490ms** after pinning the album/artist
visibility EXISTS to `tracks_catalog_album`/`tracks_catalog_artist` and the
album-credit arm to albums-first (`http-bench-100k-stats.json`). Its remaining
~300ms is the owner-wide `COUNT/SUM` join to `media_assets`; fixing that needs a
covering index migration or incremental counters, not a query hint.

## AUD-09 measured 100k read costs (2026-10-20)

Executed on an isolated 100k-owner fixture with the release binary and the real
migrations/setup API (no audio, private temporary data, same host as the older
entries above; single client unless stated). Full numbers, method and artifacts:
[PROGRESS.md](PROGRESS.md) and `target/checks/aud09/`.

| Endpoint | Before | After |
| --- | --- | --- |
| `/api/stats` p50, 8 clients | 531ms | 119ms |
| `/api/search?q=Signal` p50, 8 clients | 138ms | 121ms |
| `/api/search?q=Signal` p50, 1 client | 22ms | 20ms |
| `/api/artists?limit=50` p50 | 30.9ms | 27.0ms |

The fix is migration 0013 (covering `tracks(owner_id, media_asset_id, duration_ms)`,
`media_assets(id, size_bytes)`, `tracks(owner_id, album_id)`), an index-only stats
aggregate whose three groups run on separate pool connections, and a two-phase
search that matches bounded identities before building summaries. Search identity
is unchanged (returned ids/titles match the authoritative FTS selection exactly).
Write cost: 2,000 metadata inserts took 5.1ms with the new indexes and 4.0ms
without (+1.1ms), +4.5MiB per 100k rows. These are single-host, single-client
lower bounds; the 4-connection SQLite pool bounds concurrency, so no capacity or
RPS-superiority claim is made. Live runtime was not upgraded: the migration was
verified on an isolated copy of the live data directory (schema 12->13,
`quick_check ok`, 0 FK violations).
