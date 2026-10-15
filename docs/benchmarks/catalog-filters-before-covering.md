# Typed catalog filter SQL measurements

Measured 2026-10-15T05:50:37.540746+00:00 using `scripts/benchmark_filters.py`.

Each workload uses the stated owner count plus an equal number of private tracks belonging to a second owner in a new temporary database. Deterministic synthetic metadata includes repeated mixed-case titles, timestamp ties, unknown properties, five genre labels with whitespace/case variants, eight actual container/codec labels and private-only search/facet distractors. No audio files or existing user data are used.

The real migrations0001–0004, FTS/group triggers and batched seed writes run first. The real migration0005 index build is timed separately against populated metadata. Seed timing thus excludes maintenance of the new five indexes; it is not media ingestion throughput. Index database growth is after WAL checkpoint; optional dbstat bytes account for the five new B-trees directly. No ANALYZE or VACUUM is run.

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
| 10,000 / 10,000 | library_first | 0.105 / 0.115 / 0.130 | 0.267 / 0.329 / 0.329 | 51 |
| 10,000 / 10,000 | library_next | 0.111 / 0.119 / 0.121 | 0.266 / 0.338 / 0.338 | 51 |
| 10,000 / 10,000 | genre_first | 0.113 / 0.121 / 0.143 | 0.274 / 0.385 / 0.385 | 51 |
| 10,000 / 10,000 | genre_next | 0.115 / 0.136 / 0.144 | 0.273 / 0.369 / 0.369 | 51 |
| 10,000 / 10,000 | format_first | 0.114 / 0.123 / 0.133 | 0.271 / 0.411 / 0.411 | 51 |
| 10,000 / 10,000 | format_next | 0.116 / 0.128 / 0.144 | 0.288 / 0.362 / 0.362 | 51 |
| 10,000 / 10,000 | year_range_first | 3.432 / 4.391 / 5.330 | 3.604 / 3.776 / 3.776 | 51 |
| 10,000 / 10,000 | year_range_next | 0.123 / 0.134 / 0.138 | 0.321 / 0.409 / 0.409 | 51 |
| 10,000 / 10,000 | quality_minimums_first | 0.143 / 0.161 / 0.187 | 0.410 / 0.870 / 0.870 | 51 |
| 10,000 / 10,000 | quality_minimums_next | 0.150 / 0.157 / 0.162 | 0.451 / 0.577 / 0.577 | 51 |
| 10,000 / 10,000 | combined_first | 1.938 / 2.156 / 2.399 | 2.065 / 2.261 / 2.261 | 0 |
| 10,000 / 10,000 | favorite_genre_first | 0.219 / 0.242 / 0.256 | 0.600 / 0.875 / 0.875 | 51 |
| 10,000 / 10,000 | favorite_genre_next | 0.201 / 0.220 / 0.231 | 0.573 / 0.753 / 0.753 | 51 |
| 10,000 / 10,000 | title_first | 0.114 / 0.123 / 0.138 | 0.271 / 0.357 / 0.357 | 51 |
| 10,000 / 10,000 | title_next | 0.119 / 0.126 / 0.133 | 0.314 / 0.495 / 0.495 | 51 |
| 10,000 / 10,000 | newest_first | 0.111 / 0.116 / 0.137 | 0.236 / 0.264 / 0.264 | 51 |
| 10,000 / 10,000 | newest_next | 0.119 / 0.128 / 0.153 | 0.262 / 0.328 / 0.328 | 51 |
| 10,000 / 10,000 | genre_title_first | 0.144 / 0.153 / 0.157 | 0.436 / 0.651 / 0.651 | 51 |
| 10,000 / 10,000 | genre_title_next | 0.154 / 0.171 / 0.183 | 0.439 / 0.594 / 0.594 | 51 |
| 10,000 / 10,000 | format_newest_first | 0.154 / 0.166 / 0.197 | 0.315 / 0.391 / 0.391 | 51 |
| 10,000 / 10,000 | format_newest_next | 0.170 / 0.184 / 0.186 | 0.356 / 0.552 / 0.552 | 51 |
| 10,000 / 10,000 | fts_rare_first | 0.066 / 0.071 / 0.076 | 0.214 / 0.240 / 0.240 | 1 |
| 10,000 / 10,000 | fts_broad_title_first | 3.018 / 4.148 / 4.490 | 3.263 / 4.433 / 4.433 | 51 |
| 10,000 / 10,000 | fts_broad_title_next | 3.070 / 4.281 / 4.536 | 3.477 / 4.642 / 4.642 | 51 |
| 10,000 / 10,000 | fts_broad_filtered_first | 2.419 / 3.109 / 4.243 | 2.952 / 4.755 / 4.755 | 51 |
| 10,000 / 10,000 | fts_broad_filtered_next | 2.349 / 3.238 / 3.819 | 2.609 / 3.706 / 3.706 | 51 |
| 10,000 / 10,000 | fts_private_only_first | 0.061 / 0.067 / 0.068 | 0.194 / 0.458 / 0.458 | 0 |
| 10,000 / 10,000 | fts_absent_first | 0.016 / 0.016 / 0.020 | 0.092 / 0.104 / 0.104 | 0 |
| 10,000 / 10,000 | private_genre_first | 0.025 / 0.027 / 0.029 | 0.070 / 0.074 / 0.074 | 0 |
| 10,000 / 10,000 | library_midpoint | 0.113 / 0.120 / 0.131 | 0.269 / 0.312 / 0.312 | 51 |
| 10,000 / 10,000 | library_midpoint_next | 0.111 / 0.116 / 0.118 | 0.260 / 0.349 / 0.349 | 51 |
| 10,000 / 10,000 | title_midpoint | 0.121 / 0.129 / 0.142 | 0.253 / 0.296 / 0.296 | 51 |
| 10,000 / 10,000 | title_midpoint_next | 0.122 / 0.128 / 0.134 | 0.255 / 0.376 / 0.376 | 51 |
| 10,000 / 10,000 | newest_midpoint | 0.119 / 0.125 / 0.135 | 0.245 / 0.301 / 0.301 | 51 |
| 10,000 / 10,000 | newest_midpoint_next | 0.118 / 0.123 / 0.137 | 0.255 / 0.339 / 0.339 | 51 |
| 10,000 / 10,000 | facets snapshot | 7.492 / 8.164 / 8.452 | 7.872 / 9.139 / 9.139 | bounded values |
| 100,000 / 100,000 | library_first | 0.109 / 0.114 / 0.116 | 0.271 / 0.326 / 0.326 | 51 |
| 100,000 / 100,000 | library_next | 0.109 / 0.114 / 0.121 | 0.271 / 0.340 / 0.340 | 51 |
| 100,000 / 100,000 | genre_first | 0.114 / 0.122 / 0.133 | 0.285 / 0.334 / 0.334 | 51 |
| 100,000 / 100,000 | genre_next | 0.116 / 0.125 / 0.150 | 0.281 / 0.435 / 0.435 | 51 |
| 100,000 / 100,000 | format_first | 0.114 / 0.124 / 0.132 | 0.291 / 0.362 / 0.362 | 51 |
| 100,000 / 100,000 | format_next | 0.116 / 0.122 / 0.126 | 0.301 / 0.346 / 0.346 | 51 |
| 100,000 / 100,000 | year_range_first | 92.631 / 99.143 / 104.580 | 91.904 / 93.789 / 93.789 | 51 |
| 100,000 / 100,000 | year_range_next | 0.125 / 0.137 / 0.153 | 0.378 / 0.633 / 0.633 | 51 |
| 100,000 / 100,000 | quality_minimums_first | 0.150 / 0.171 / 0.182 | 0.488 / 0.724 / 0.724 | 51 |
| 100,000 / 100,000 | quality_minimums_next | 0.152 / 0.171 / 0.223 | 0.483 / 0.733 / 0.733 | 51 |
| 100,000 / 100,000 | combined_first | 28.960 / 31.031 / 31.677 | 28.661 / 33.264 / 33.264 | 0 |
| 100,000 / 100,000 | favorite_genre_first | 0.248 / 0.293 / 0.345 | 0.731 / 1.016 / 1.016 | 51 |
| 100,000 / 100,000 | favorite_genre_next | 0.244 / 0.279 / 0.285 | 0.819 / 0.991 / 0.991 | 51 |
| 100,000 / 100,000 | title_first | 0.125 / 0.141 / 0.167 | 0.412 / 0.573 / 0.573 | 51 |
| 100,000 / 100,000 | title_next | 0.135 / 0.165 / 0.326 | 0.421 / 0.601 / 0.601 | 51 |
| 100,000 / 100,000 | newest_first | 0.123 / 0.135 / 0.149 | 0.303 / 0.402 / 0.402 | 51 |
| 100,000 / 100,000 | newest_next | 0.129 / 0.149 / 0.166 | 0.336 / 0.445 / 0.445 | 51 |
| 100,000 / 100,000 | genre_title_first | 0.168 / 0.223 / 0.260 | 0.562 / 0.832 / 0.832 | 51 |
| 100,000 / 100,000 | genre_title_next | 0.192 / 0.221 / 0.238 | 0.588 / 0.821 / 0.821 | 51 |
| 100,000 / 100,000 | format_newest_first | 0.174 / 0.194 / 0.200 | 0.432 / 0.519 / 0.519 | 51 |
| 100,000 / 100,000 | format_newest_next | 0.188 / 0.232 / 0.245 | 0.428 / 0.668 / 0.668 | 51 |
| 100,000 / 100,000 | fts_rare_first | 0.084 / 0.090 / 0.104 | 0.277 / 0.379 / 0.379 | 1 |
| 100,000 / 100,000 | fts_broad_title_first | 3.054 / 3.271 / 3.552 | 3.607 / 4.000 / 4.000 | 51 |
| 100,000 / 100,000 | fts_broad_title_next | 3.207 / 3.437 / 3.597 | 3.742 / 4.021 / 4.021 | 51 |
| 100,000 / 100,000 | fts_broad_filtered_first | 3.517 / 3.936 / 4.308 | 4.029 / 5.571 / 5.571 | 51 |
| 100,000 / 100,000 | fts_broad_filtered_next | 3.546 / 4.237 / 4.692 | 4.187 / 4.834 / 4.834 | 51 |
| 100,000 / 100,000 | fts_private_only_first | 0.075 / 0.083 / 0.088 | 0.249 / 0.276 / 0.276 | 0 |
| 100,000 / 100,000 | fts_absent_first | 0.014 / 0.015 / 0.017 | 0.083 / 0.090 / 0.090 | 0 |
| 100,000 / 100,000 | private_genre_first | 0.025 / 0.026 / 0.053 | 0.072 / 0.125 / 0.125 | 0 |
| 100,000 / 100,000 | library_midpoint | 0.122 / 0.145 / 0.172 | 0.316 / 0.655 / 0.655 | 51 |
| 100,000 / 100,000 | library_midpoint_next | 0.122 / 0.134 / 0.158 | 0.306 / 0.603 / 0.603 | 51 |
| 100,000 / 100,000 | title_midpoint | 0.133 / 0.159 / 0.171 | 0.370 / 1.296 / 1.296 | 51 |
| 100,000 / 100,000 | title_midpoint_next | 0.128 / 0.198 / 0.237 | 0.381 / 0.504 / 0.504 | 51 |
| 100,000 / 100,000 | newest_midpoint | 0.126 / 0.140 / 0.178 | 0.311 / 0.422 / 0.422 | 51 |
| 100,000 / 100,000 | newest_midpoint_next | 0.129 / 0.155 / 0.189 | 0.401 / 0.925 / 0.925 | 51 |
| 100,000 / 100,000 | facets snapshot | 156.088 / 174.301 / 179.814 | 151.981 / 164.780 / 164.780 | bounded values |

| Owner / private tracks | Seed seconds (old indexes) | New index build seconds | DB MiB | New B-tree MiB |
| ---: | ---: | ---: | ---: | ---: |
| 10,000 / 10,000 | 3.858 | 0.083 | 35.32 | 7.47 |
| 100,000 / 100,000 | 83.936 | 1.919 | 348.44 | 75.07 |

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
SEARCH tracks USING INDEX tracks_owner_genre_filter (owner_id=?)
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
  "crates/media/src/lib.rs": "b7719bee7332bcdd7ff3528bcb540ab3372230f55917259ae4924007499fc643",
  "crates/media/src/filters.rs": "0e8eda7673724357cb329804a2eca117aa0dc376964abbaa42fb56ac30d07390",
  "scripts/benchmark_filters.py": "79ca69aca6ae10bf9a2cc90cbc263f17bd1f91eb983dea13336bddc3bc030fd6",
  "scripts/benchmark_catalog.py": "84c425443aef5ed679397d92d49dad28bc41e3dccf37f1d5be6b6c474e81a67b"
}
```
