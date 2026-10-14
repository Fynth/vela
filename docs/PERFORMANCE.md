# Reproducible performance measurements

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
