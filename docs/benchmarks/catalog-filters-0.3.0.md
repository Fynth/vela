# Typed catalog filter SQL measurements

Measured 2026-10-15T05:59:47.355588+00:00 using `scripts/benchmark_filters.py`.

Index mode: **Covering genre facets (migrations0005+0006)**. Applied index migrations: `0005_catalog_filters.sql, 0006_cover_genre_facets.sql`.

Each workload uses the stated owner count plus an equal number of private tracks belonging to a second owner in a new temporary database. Deterministic synthetic metadata includes repeated mixed-case titles, timestamp ties, unknown properties, five genre labels with whitespace/case variants, eight actual container/codec labels and private-only search/facet distractors. No audio files or existing user data are used.

The real migrations0001–0004, FTS/group triggers and batched seed writes run first. The selected real index migrations are timed separately against populated metadata. Seed timing thus excludes maintenance of the new five indexes; it is not media ingestion throughput. Index database growth is after WAL checkpoint; optional dbstat bytes account for the five final B-trees directly. Covering mode includes the cost of creating the original genre index in0005 and replacing it in0006, rather than silently timing only the final definition. Timed Python executescript calls exclude the following WAL checkpoint and are not native SQLx migration timings. Database file growth may include free pages left by an index replacement. No ANALYZE or VACUUM is run.

Queries use the current Track projection,50 rows plus a sentinel, bound filter values and an owner predicate. SQLite page-cache target is2000KiB, WAL/FULL, mmap disabled. Each alternate-sort next-page timing includes its owner-scoped key lookup. FTS timings include the bounded 4097-row probe and sparse/broad choice. Both facet reads share a transaction. Ten untimed executions precede warm samples; cache-reset samples open a new SQLite connection but retain the host filesystem cache. Reported timings include Python execute/fetch and SQL mirror construction, and exclude connection setup, Rust SQLx/DTO mapping, authentication, HTTP/JSON, network, concurrent writes and UI. The script checks row ownership, criteria, ordering, tie-breaks, page sentinels, foreign cursor handling and nondisclosure of private-only facets/results.

This SQL mirror is reviewed against the source fingerprints below; it is not an end-to-end server benchmark. Median anchors are selected before timing; fixture-only OFFSET is never part of a measured page. First, second and median pages do not characterize every worst-case distribution of repeated titles. Case folding in production remains SQLite ASCII NOCASE; no Unicode collation claim follows. Numeric thresholds do not establish losslessness. There is no product superiority, hardware-capacity or listener-count claim.

```json
{
  "cpu": "AMD Ryzen 7 8845HS w/ Radeon 780M Graphics",
  "logical_cpus": 16,
  "memory_bytes": 16024821760,
  "platform": "Linux-7.2.9-2-cachyos-x86_64-with-glibc2.44",
  "python": "3.14.8",
  "sqlite": "3.54.0",
  "temp_root": "/home/rasul/Projects/vela/target/filter-bench"
}
```

Samples: 100 warm and 10 new-connection samples per case. Percentiles use nearest rank; small-sample p99 is approximate.

| Owner / private tracks | Case | Warm p50 / p95 / p99 ms | Cache-reset p50 / p95 / p99 ms | Rows |
| ---: | --- | ---: | ---: | ---: |
| 10,000 / 10,000 | library_first | 0.102 / 0.139 / 0.183 | 0.266 / 0.378 / 0.378 | 51 |
| 10,000 / 10,000 | library_next | 0.101 / 0.112 / 0.116 | 0.243 / 0.416 / 0.416 | 51 |
| 10,000 / 10,000 | genre_first | 0.104 / 0.114 / 0.123 | 0.261 / 0.303 / 0.303 | 51 |
| 10,000 / 10,000 | genre_next | 0.105 / 0.115 / 0.122 | 0.260 / 0.283 / 0.283 | 51 |
| 10,000 / 10,000 | format_first | 0.104 / 0.110 / 0.137 | 0.251 / 0.283 / 0.283 | 51 |
| 10,000 / 10,000 | format_next | 0.106 / 0.113 / 0.121 | 0.273 / 0.390 / 0.390 | 51 |
| 10,000 / 10,000 | year_range_first | 3.003 / 3.126 / 3.209 | 3.424 / 4.082 / 4.082 | 51 |
| 10,000 / 10,000 | year_range_next | 0.114 / 0.122 / 0.128 | 0.308 / 0.527 / 0.527 | 51 |
| 10,000 / 10,000 | quality_minimums_first | 0.129 / 0.148 / 0.155 | 0.400 / 0.456 / 0.456 | 51 |
| 10,000 / 10,000 | quality_minimums_next | 0.136 / 0.149 / 0.157 | 0.423 / 0.471 / 0.471 | 51 |
| 10,000 / 10,000 | combined_first | 1.773 / 2.003 / 2.260 | 1.949 / 2.214 / 2.214 | 0 |
| 10,000 / 10,000 | favorite_genre_first | 0.198 / 0.226 / 0.265 | 0.574 / 0.822 / 0.822 | 51 |
| 10,000 / 10,000 | favorite_genre_next | 0.183 / 0.205 / 0.212 | 0.560 / 0.677 / 0.677 | 51 |
| 10,000 / 10,000 | title_first | 0.104 / 0.118 / 0.140 | 0.259 / 0.290 / 0.290 | 51 |
| 10,000 / 10,000 | title_next | 0.113 / 0.121 / 0.126 | 0.278 / 0.353 / 0.353 | 51 |
| 10,000 / 10,000 | newest_first | 0.102 / 0.113 / 0.129 | 0.217 / 0.254 / 0.254 | 51 |
| 10,000 / 10,000 | newest_next | 0.111 / 0.123 / 0.130 | 0.237 / 0.284 / 0.284 | 51 |
| 10,000 / 10,000 | genre_title_first | 0.132 / 0.142 / 0.149 | 0.429 / 0.520 / 0.520 | 51 |
| 10,000 / 10,000 | genre_title_next | 0.144 / 0.155 / 0.175 | 0.426 / 0.614 / 0.614 | 51 |
| 10,000 / 10,000 | format_newest_first | 0.142 / 0.158 / 0.170 | 0.308 / 0.454 / 0.454 | 51 |
| 10,000 / 10,000 | format_newest_next | 0.161 / 0.177 / 0.185 | 0.335 / 0.366 / 0.366 | 51 |
| 10,000 / 10,000 | fts_rare_first | 0.063 / 0.069 / 0.078 | 0.202 / 0.227 / 0.227 | 1 |
| 10,000 / 10,000 | fts_broad_title_first | 2.662 / 3.196 / 3.927 | 2.944 / 3.286 / 3.286 | 51 |
| 10,000 / 10,000 | fts_broad_title_next | 2.868 / 3.777 / 4.068 | 3.172 / 4.362 / 4.362 | 51 |
| 10,000 / 10,000 | fts_broad_filtered_first | 2.591 / 3.307 / 3.806 | 2.737 / 3.823 / 3.823 | 51 |
| 10,000 / 10,000 | fts_broad_filtered_next | 2.388 / 3.242 / 3.511 | 2.690 / 3.226 / 3.226 | 51 |
| 10,000 / 10,000 | fts_private_only_first | 0.058 / 0.065 / 0.070 | 0.199 / 0.208 / 0.208 | 0 |
| 10,000 / 10,000 | fts_absent_first | 0.015 / 0.018 / 0.020 | 0.090 / 0.130 / 0.130 | 0 |
| 10,000 / 10,000 | private_genre_first | 0.024 / 0.029 / 0.030 | 0.068 / 0.070 / 0.070 | 0 |
| 10,000 / 10,000 | library_midpoint | 0.104 / 0.114 / 0.145 | 0.269 / 0.290 / 0.290 | 51 |
| 10,000 / 10,000 | library_midpoint_next | 0.107 / 0.130 / 0.138 | 0.246 / 0.330 / 0.330 | 51 |
| 10,000 / 10,000 | title_midpoint | 0.119 / 0.160 / 0.183 | 0.249 / 0.374 / 0.374 | 51 |
| 10,000 / 10,000 | title_midpoint_next | 0.114 / 0.125 / 0.133 | 0.242 / 0.283 / 0.283 | 51 |
| 10,000 / 10,000 | newest_midpoint | 0.113 / 0.122 / 0.138 | 0.246 / 0.280 / 0.280 | 51 |
| 10,000 / 10,000 | newest_midpoint_next | 0.116 / 0.129 / 0.143 | 0.267 / 0.376 / 0.376 | 51 |
| 10,000 / 10,000 | facets snapshot | 1.728 / 1.902 / 2.030 | 2.094 / 2.605 / 2.605 | bounded values |
| 100,000 / 100,000 | library_first | 0.117 / 0.128 / 0.132 | 0.321 / 0.464 / 0.464 | 51 |
| 100,000 / 100,000 | library_next | 0.120 / 0.127 / 0.134 | 0.281 / 0.377 / 0.377 | 51 |
| 100,000 / 100,000 | genre_first | 0.121 / 0.128 / 0.144 | 0.295 / 0.577 / 0.577 | 51 |
| 100,000 / 100,000 | genre_next | 0.124 / 0.132 / 0.141 | 0.300 / 0.439 / 0.439 | 51 |
| 100,000 / 100,000 | format_first | 0.121 / 0.129 / 0.149 | 0.301 / 0.395 / 0.395 | 51 |
| 100,000 / 100,000 | format_next | 0.123 / 0.132 / 0.148 | 0.303 / 0.405 / 0.405 | 51 |
| 100,000 / 100,000 | year_range_first | 98.918 / 103.648 / 105.024 | 100.313 / 103.689 / 103.689 | 51 |
| 100,000 / 100,000 | year_range_next | 0.130 / 0.155 / 0.185 | 0.438 / 1.042 / 1.042 | 51 |
| 100,000 / 100,000 | quality_minimums_first | 0.160 / 0.203 / 0.382 | 0.606 / 1.531 / 1.531 | 51 |
| 100,000 / 100,000 | quality_minimums_next | 0.155 / 0.169 / 0.178 | 0.585 / 0.734 / 0.734 | 51 |
| 100,000 / 100,000 | combined_first | 30.410 / 37.086 / 44.214 | 35.879 / 36.942 / 36.942 | 0 |
| 100,000 / 100,000 | favorite_genre_first | 0.288 / 0.408 / 0.573 | 1.227 / 1.711 / 1.711 | 51 |
| 100,000 / 100,000 | favorite_genre_next | 0.402 / 0.545 / 0.571 | 1.583 / 1.867 / 1.867 | 51 |
| 100,000 / 100,000 | title_first | 0.188 / 0.264 / 0.360 | 0.863 / 1.311 / 1.311 | 51 |
| 100,000 / 100,000 | title_next | 0.194 / 0.297 / 0.352 | 0.853 / 1.047 / 1.047 | 51 |
| 100,000 / 100,000 | newest_first | 0.168 / 0.231 / 0.280 | 0.651 / 0.769 / 0.769 | 51 |
| 100,000 / 100,000 | newest_next | 0.186 / 0.232 / 0.299 | 0.637 / 0.656 / 0.656 | 51 |
| 100,000 / 100,000 | genre_title_first | 0.255 / 0.370 / 0.398 | 1.131 / 1.405 / 1.405 | 51 |
| 100,000 / 100,000 | genre_title_next | 0.286 / 0.356 / 0.452 | 1.214 / 1.435 / 1.435 | 51 |
| 100,000 / 100,000 | format_newest_first | 0.256 / 0.418 / 0.507 | 0.809 / 1.019 / 1.019 | 51 |
| 100,000 / 100,000 | format_newest_next | 0.576 / 0.707 / 0.781 | 1.401 / 1.524 / 1.524 | 51 |
| 100,000 / 100,000 | fts_rare_first | 0.205 / 0.258 / 0.265 | 0.578 / 0.703 / 0.703 | 1 |
| 100,000 / 100,000 | fts_broad_title_first | 6.815 / 7.379 / 8.346 | 7.881 / 70.969 / 70.969 | 51 |
| 100,000 / 100,000 | fts_broad_title_next | 6.630 / 7.126 / 7.342 | 4.595 / 7.485 / 7.485 | 51 |
| 100,000 / 100,000 | fts_broad_filtered_first | 3.637 / 4.295 / 4.537 | 5.193 / 7.858 / 7.858 | 51 |
| 100,000 / 100,000 | fts_broad_filtered_next | 4.056 / 9.510 / 11.555 | 8.832 / 36.274 / 36.274 | 51 |
| 100,000 / 100,000 | fts_private_only_first | 0.319 / 0.617 / 2.929 | 0.837 / 1.116 / 1.116 | 0 |
| 100,000 / 100,000 | fts_absent_first | 0.023 / 0.035 / 0.049 | 0.233 / 0.489 / 0.489 | 0 |
| 100,000 / 100,000 | private_genre_first | 0.045 / 0.101 / 0.114 | 0.273 / 8.693 / 8.693 | 0 |
| 100,000 / 100,000 | library_midpoint | 0.110 / 0.120 / 0.140 | 0.275 / 0.575 / 0.575 | 51 |
| 100,000 / 100,000 | library_midpoint_next | 0.109 / 0.121 / 0.140 | 0.399 / 1.026 / 1.026 | 51 |
| 100,000 / 100,000 | title_midpoint | 0.121 / 0.132 / 0.161 | 0.420 / 0.824 / 0.824 | 51 |
| 100,000 / 100,000 | title_midpoint_next | 0.132 / 0.171 / 0.182 | 0.389 / 0.584 / 0.584 | 51 |
| 100,000 / 100,000 | newest_midpoint | 0.128 / 0.200 / 0.211 | 0.314 / 0.525 / 0.525 | 51 |
| 100,000 / 100,000 | newest_midpoint_next | 0.119 / 0.154 / 0.175 | 0.333 / 0.673 / 0.673 | 51 |
| 100,000 / 100,000 | facets snapshot | 22.614 / 26.863 / 30.480 | 20.936 / 28.061 / 28.061 | bounded values |

| Owner / private tracks | Seed seconds (old indexes) | New index build seconds | DB MiB | New B-tree MiB |
| ---: | ---: | ---: | ---: | ---: |
| 10,000 / 10,000 | 3.770 | 0.124 | 35.49 | 7.60 |
| 100,000 / 100,000 | 100.577 | 2.432 | 349.69 | 76.37 |

| Owner / private tracks | Index migration | Seconds | DB before / after MiB |
| ---: | --- | ---: | ---: |
| 10,000 / 10,000 | 0005_catalog_filters.sql | 0.095 | 27.89 / 35.36 |
| 10,000 / 10,000 | 0006_cover_genre_facets.sql | 0.029 | 35.36 / 35.49 |
| 100,000 / 100,000 | 0005_catalog_filters.sql | 1.771 | 273.32 / 348.39 |
| 100,000 / 100,000 | 0006_cover_genre_facets.sql | 0.662 | 348.39 / 349.69 |

## Query plans for the largest workload

### library_first

```text
SEARCH t USING INDEX tracks_owner_cursor (owner_id=?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### library_next

```text
SEARCH t USING INDEX tracks_owner_cursor (owner_id=? AND id>?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### genre_first

```text
SEARCH t USING INDEX tracks_owner_genre_filter (owner_id=? AND <expr>=?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### genre_next

```text
SEARCH t USING INDEX tracks_owner_genre_filter (owner_id=? AND <expr>=? AND id>?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### format_first

```text
SEARCH t USING INDEX tracks_owner_codec_filter (owner_id=? AND codec=?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### format_next

```text
SEARCH t USING INDEX tracks_owner_codec_filter (owner_id=? AND codec=? AND id>?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### year_range_first

```text
SEARCH t USING INDEX tracks_owner_year_filter (owner_id=? AND year>? AND year<?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
USE TEMP B-TREE FOR ORDER BY
```

### year_range_next

```text
SEARCH t USING INDEX tracks_owner_cursor (owner_id=? AND id>?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### quality_minimums_first

```text
SEARCH t USING INDEX tracks_owner_cursor (owner_id=?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### quality_minimums_next

```text
SEARCH t USING INDEX tracks_owner_cursor (owner_id=? AND id>?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### combined_first

```text
SEARCH t USING INDEX tracks_owner_year_filter (owner_id=? AND year>? AND year<?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
USE TEMP B-TREE FOR ORDER BY
```

### favorite_genre_first

```text
SEARCH t USING INDEX tracks_owner_genre_filter (owner_id=? AND <expr>=?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### favorite_genre_next

```text
SEARCH t USING INDEX tracks_owner_genre_filter (owner_id=? AND <expr>=? AND id>?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### title_first

```text
SEARCH t USING INDEX tracks_owner_title_order (owner_id=?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### title_next

```text
SEARCH t USING INDEX tracks_owner_title_order (owner_id=? AND (title,id)>(?,?))
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### newest_first

```text
SEARCH t USING INDEX tracks_owner_newest_order (owner_id=?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### newest_next

```text
SEARCH t USING INDEX tracks_owner_newest_order (owner_id=? AND (added_at,id)<(?,?))
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### genre_title_first

```text
SEARCH t USING INDEX tracks_owner_title_order (owner_id=?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### genre_title_next

```text
SEARCH t USING INDEX tracks_owner_title_order (owner_id=? AND (title,id)>(?,?))
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### format_newest_first

```text
SEARCH t USING INDEX tracks_owner_newest_order (owner_id=?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### format_newest_next

```text
SEARCH t USING INDEX tracks_owner_newest_order (owner_id=? AND (added_at,id)<(?,?))
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### fts_rare_first

```text
SCAN tracks_fts VIRTUAL TABLE INDEX 0:M5
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
USE TEMP B-TREE FOR ORDER BY
```

### fts_broad_title_first

```text
SEARCH t USING INDEX tracks_owner_title_order (owner_id=?)
LIST SUBQUERY 1
SCAN tracks_fts VIRTUAL TABLE INDEX 0:M5
CREATE BLOOM FILTER
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### fts_broad_title_next

```text
SEARCH t USING INDEX tracks_owner_title_order (owner_id=? AND (title,id)>(?,?))
LIST SUBQUERY 1
SCAN tracks_fts VIRTUAL TABLE INDEX 0:M5
CREATE BLOOM FILTER
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### fts_broad_filtered_first

```text
SEARCH t USING INDEX tracks_owner_newest_order (owner_id=?)
LIST SUBQUERY 1
SCAN tracks_fts VIRTUAL TABLE INDEX 0:M5
CREATE BLOOM FILTER
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### fts_broad_filtered_next

```text
SEARCH t USING INDEX tracks_owner_newest_order (owner_id=? AND (added_at,id)<(?,?))
LIST SUBQUERY 1
SCAN tracks_fts VIRTUAL TABLE INDEX 0:M5
CREATE BLOOM FILTER
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### fts_private_only_first

```text
SCAN tracks_fts VIRTUAL TABLE INDEX 0:M5
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
USE TEMP B-TREE FOR ORDER BY
```

### fts_absent_first

```text
```

### private_genre_first

```text
SEARCH t USING INDEX tracks_owner_genre_filter (owner_id=? AND <expr>=?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### library_midpoint

```text
SEARCH t USING INDEX tracks_owner_cursor (owner_id=? AND id>?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### library_midpoint_next

```text
SEARCH t USING INDEX tracks_owner_cursor (owner_id=? AND id>?)
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### title_midpoint

```text
SEARCH t USING INDEX tracks_owner_title_order (owner_id=? AND (title,id)>(?,?))
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### title_midpoint_next

```text
SEARCH t USING INDEX tracks_owner_title_order (owner_id=? AND (title,id)>(?,?))
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### newest_midpoint

```text
SEARCH t USING INDEX tracks_owner_newest_order (owner_id=? AND (added_at,id)<(?,?))
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### newest_midpoint_next

```text
SEARCH t USING INDEX tracks_owner_newest_order (owner_id=? AND (added_at,id)<(?,?))
SEARCH a USING INDEX sqlite_autoindex_media_assets_1 (id=?)
SEARCH f USING COVERING INDEX sqlite_autoindex_favorites_1 (user_id=? AND track_id=?) LEFT-JOIN
```

### Facet genres

```text
SEARCH tracks USING COVERING INDEX tracks_owner_genre_filter (owner_id=?)
```

### Facet codecs

```text
SEARCH tracks USING COVERING INDEX tracks_owner_codec_filter (owner_id=?)
```

## Reproduce

```bash
python3 scripts/benchmark_filters.py --sizes 10000 100000 \
  --samples 100 --cold-samples 10 \
  --tmp-root target/filter-bench \
  --output target/checks/catalog-filters.json \
  --write-doc target/checks/catalog-filters.md
```

Full SQL, bindings, probe cardinalities, plans, raw samples and per-index sizes are retained in the JSON report. Temporary databases are removed automatically. Re-run on idle constrained hosts and measure real HTTP/concurrent workloads before making operational recommendations.

Source fingerprints:

```json
{
  "migrations/0001_initial.sql": "1f18d72a55b33f5b1188df7228960f75ae3d6c4523ca4c0d3feb81f713cb5360",
  "migrations/0002_catalog_groups.sql": "9dff95fade851f8de07b048931e933e85b225c638104b0c9eda470be70f6d34c",
  "migrations/0003_catalog_metadata_revision.sql": "d6358caac6f016688794e25ef248a9728e77764304651156fecbc741ca61687d",
  "migrations/0004_readonly_scan_sources.sql": "3a3d8ba780f5623a1320b2e7b73803e797ad698bca11f91fd3eb0cb0d20ad711",
  "migrations/0005_catalog_filters.sql": "96fb58f5547f19cb0b9b775dd3004e5d5b5521d18dc5ace6a51b2791ca08ee3f",
  "migrations/0006_cover_genre_facets.sql": "c3e5c589f1da4850d39eb1e2290f2e656d19740f64bcf0b36a33e828581b807a",
  "crates/media/src/lib.rs": "b7719bee7332bcdd7ff3528bcb540ab3372230f55917259ae4924007499fc643",
  "crates/media/src/filters.rs": "0e8eda7673724357cb329804a2eca117aa0dc376964abbaa42fb56ac30d07390",
  "scripts/benchmark_filters.py": "2efc8430338cc864c95fcbe0472fd823bce8e334c10e5cad0a558a2725e9be85",
  "scripts/benchmark_catalog.py": "84c425443aef5ed679397d92d49dad28bc41e3dccf37f1d5be6b6c474e81a67b"
}
```
