#!/usr/bin/env python3
"""Reproducible metadata-only SQLite benchmark; creates no real media files.

Run from any directory. Databases live in private temporary directories and are
removed after each workload. This never connects to an existing Vela database.
The SQL projection comes from media/src/lib.rs; filtering preserves the original
rowid-IN baseline. benchmark_search.py compares the current adaptive FTS path.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import random
import re
import shutil
import sqlite3
import sys
import tempfile
import time
import uuid
from datetime import datetime, timezone


REPOSITORY = Path(__file__).resolve().parents[1]
OWNER = "catalog-benchmark-owner"
SEED = 0x56454C41
CACHE_KIB = 2000
PAGE_SIZE = 50


def sql_projection() -> str:
    source = (REPOSITORY / "crates/media/src/lib.rs").read_text()
    match = re.search(r'const TRACK_SELECT: &str = ("[^\n]+?");', source)
    if match is None:
        raise RuntimeError("Cannot find media Track SQL projection; update the benchmark contract")
    return json.loads(match.group(1))


def connect(database: Path) -> sqlite3.Connection:
    connection = sqlite3.connect(database, timeout=10)
    connection.execute("PRAGMA foreign_keys=ON")
    connection.execute("PRAGMA journal_mode=WAL")
    connection.execute("PRAGMA synchronous=FULL")
    connection.execute(f"PRAGMA cache_size=-{CACHE_KIB}")
    connection.execute("PRAGMA mmap_size=0")
    return connection


def catalog_query(projection: str, *, search: str | None = None,
                  cursor: str | None = None, favorites: bool = False) -> tuple[str, tuple]:
    query = projection + "? WHERE t.owner_id=?"
    parameters: list[str | int] = [OWNER, OWNER]
    if search is not None:
        query += " AND t.rowid IN (SELECT rowid FROM tracks_fts WHERE tracks_fts MATCH ?)"
        parameters.append(search)
    if cursor is not None:
        query += " AND t.id > ?"
        parameters.append(cursor)
    if favorites:
        query += " AND f.track_id IS NOT NULL"
    query += " ORDER BY t.id ASC LIMIT ?"
    # The service fetches one sentinel row to determine whether another page exists.
    parameters.append(PAGE_SIZE + 1)
    return query, tuple(parameters)


def seed_database(database: Path, count: int) -> dict:
    connection = connect(database)
    migrations = sorted((REPOSITORY / "migrations").glob("*.sql"))
    for migration in migrations:
        connection.executescript(migration.read_text())
    connection.execute("INSERT INTO users(id,username,password_hash) VALUES(?,?,?)",
                       (OWNER, "benchmark", "synthetic benchmark account; unusable password"))
    connection.commit()
    rng = random.Random(SEED)
    started = time.perf_counter()
    genres = ["Electronic", "Jazz", "Classical", "Rock", "Ambient"]
    for start in range(0, count, 1000):
        assets, tracks, favorites = [], [], []
        for index in range(start, min(start + 1000, count)):
            identity = str(uuid.UUID(int=rng.getrandbits(128), version=4))
            content_hash = f"{index:064x}"  # Synthetic identity; not an audio checksum.
            title = f"{'Signal' if index % 10 == 0 else 'Track'} {index:07d}"
            if index == count // 2:
                title += " UniqueNeedle"
            assets.append((identity, content_hash, f"media/{content_hash}.audio", "audio/flac", 5000000))
            tracks.append((identity, OWNER, identity, title, f"Artist{index % 1000:04d}",
                           f"Album{index // 12:07d}", f"Artist{(index // 12) % 1000:04d}",
                           genres[index % len(genres)], 180000, "FLAC"))
            if index % 20 == 0:
                favorites.append((OWNER, identity))
        with connection:
            connection.executemany("INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) VALUES(?,?,?,?,?)", assets)
            connection.executemany("INSERT INTO tracks(id,owner_id,media_asset_id,title,artist,album,album_artist,genre,duration_ms,codec) VALUES(?,?,?,?,?,?,?,?,?,?)", tracks)
            connection.executemany("INSERT INTO favorites(user_id,track_id) VALUES(?,?)", favorites)
        if (start + 1000) % 100000 == 0:
            print(f"seeded {min(start + 1000, count):,}/{count:,} rows", file=sys.stderr, flush=True)
    seed_seconds = time.perf_counter() - started
    actual = connection.execute("SELECT COUNT(*) FROM tracks WHERE owner_id=?", (OWNER,)).fetchone()[0]
    if actual != count:
        raise RuntimeError("Synthetic seed count mismatch")
    midpoint = connection.execute("SELECT id FROM tracks WHERE owner_id=? ORDER BY id LIMIT 1 OFFSET ?",
                                  (OWNER, count // 2)).fetchone()[0]
    connection.execute("PRAGMA wal_checkpoint(TRUNCATE)")
    connection.close()
    return {"seed_seconds": seed_seconds, "database_bytes": database.stat().st_size,
            "midpoint_cursor": midpoint}


def summarize(samples: list[float]) -> dict:
    ordered = sorted(samples)
    result = {"samples": len(samples), "min_ms": ordered[0], "max_ms": ordered[-1]}
    for name, percentile in [("p50_ms", .50), ("p95_ms", .95), ("p99_ms", .99)]:
        result[name] = ordered[max(0, math.ceil(len(ordered) * percentile) - 1)]
    return result


def timed_query(connection: sqlite3.Connection, query: str, parameters: tuple) -> tuple[float, int]:
    started = time.perf_counter_ns()
    rows = connection.execute(query, parameters).fetchall()
    elapsed_ms = (time.perf_counter_ns() - started) / 1_000_000
    return elapsed_ms, len(rows)


def benchmark_case(database: Path, query: str, parameters: tuple,
                   warm_samples: int, cold_samples: int) -> dict:
    connection = connect(database)
    plan = [row[3] for row in connection.execute("EXPLAIN QUERY PLAN " + query, parameters)]
    for _ in range(10):
        connection.execute(query, parameters).fetchall()
    warm = []
    for _ in range(warm_samples):
        elapsed, rows = timed_query(connection, query, parameters)
        warm.append(elapsed)
    connection.close()
    cold = []
    for _ in range(cold_samples):
        connection = connect(database)
        # Opening/configuring a connection is outside the timing interval. Its
        # page/statement caches are empty; the host filesystem cache is not reset.
        elapsed, rows = timed_query(connection, query, parameters)
        cold.append(elapsed)
        connection.close()
    return {"sql": query, "parameters": parameters, "returned_rows": rows,
            "query_plan": plan, "warm": summarize(warm), "sqlite_cache_reset": summarize(cold),
            "warm_samples_ms": warm, "sqlite_cache_reset_samples_ms": cold}


def hardware(temp_root: Path | None) -> dict:
    cpu = platform.processor()
    cpuinfo = Path("/proc/cpuinfo")
    if cpuinfo.exists():
        for line in cpuinfo.read_text().splitlines():
            if line.startswith("model name"):
                cpu = line.split(":", 1)[1].strip()
                break
    memory = None
    try:
        memory = os.sysconf("SC_PAGE_SIZE") * os.sysconf("SC_PHYS_PAGES")
    except (ValueError, OSError, AttributeError):
        pass
    return {"cpu": cpu, "logical_cpus": os.cpu_count(), "memory_bytes": memory,
            "platform": platform.platform(), "python": platform.python_version(),
            "sqlite": sqlite3.sqlite_version,
            "temp_root": str((temp_root or Path(tempfile.gettempdir())).resolve())}


def render_document(report: dict) -> str:
    host = report["hardware"]
    memory = f"{host['memory_bytes'] / 1024 ** 3:.2f} GiB" if host["memory_bytes"] else "unavailable"
    lines = ["# Reproducible performance measurements", "",
             f"Measured at {report['created_at']} using `scripts/benchmark_catalog.py`.", "",
             "These measurements exercise SQLite catalog queries over synthetic metadata. "
             "They are not HTTP latency, streaming throughput, audible playback time, or a comparison with other products.", "",
             "## Hardware and environment", "",
             f"- CPU: {host['cpu']}; {host['logical_cpus']} logical CPUs.",
             f"- Host RAM: {memory}.",
             f"- OS: `{host['platform']}`.",
             f"- Python: {host['python']}; Python SQLite: {host['sqlite']}.",
             f"- Isolated benchmark storage root: `{host['temp_root']}`.",
             "- Storage type and concurrent host activity are recorded below after the run; "
             "the script does not drop host filesystem caches.", "",
             "## Method", "",
             "The script applies the repository migrations to a fresh private database for each workload. "
             "One owner receives the indicated track count, with one physical-asset metadata row per track, "
             "deterministic UUIDv4 keys, 1,000 artist labels, albums of 12 entries, five genres and 5% favorites. "
             "No audio files are created. FTS5 triggers build the real search index during inserts. "
             "Seed time includes batched metadata writes and index maintenance, but is not media-ingestion throughput.", "",
             "Queries use the media crate's current Track projection and the same owner, FTS, keyset, "
             "favorite and ordering clauses as `list_tracks`. A page requests 50 entries plus one sentinel. "
             "Timing includes Python `execute` and full `fetchall` of all projected columns, but excludes "
             "connection creation, HTTP, authentication, JSON, Rust SQLx, network transfer and UI rendering. "
             "The Python SQLite version may differ from the server's SQLx-linked SQLite version.", "",
             f"SQLite uses WAL, synchronous FULL, a {CACHE_KIB} KiB page-cache target and disabled mmap. "
             "No ANALYZE is run. Warm measurements follow 10 untimed executions on the same connection. "
             "Cache-reset measurements use a fresh SQLite connection each time; the filesystem cache remains warm. "
             "They therefore do not establish disk-cold latency. Percentiles use nearest-rank selection; "
             "p99 from a small sample count is approximate.", "",
             "## Results", "",
             "| Tracks | Query | Warm p50 / p95 / p99 ms | Cache-reset p50 / p95 / p99 ms | Rows fetched |",
             "| ---: | --- | ---: | ---: | ---: |"]
    for workload in report["workloads"]:
        for name, case in workload["queries"].items():
            warm, cold = case["warm"], case["sqlite_cache_reset"]
            triple = lambda result: " / ".join(f"{result[key]:.3f}" for key in ("p50_ms", "p95_ms", "p99_ms"))
            lines.append(f"| {workload['tracks']:,} | {name} | {triple(warm)} | {triple(cold)} | {case['returned_rows']} |")
    lines += ["", f"Each query has {report['warm_samples']} warm samples and {report['cold_samples']} cache-reset samples.", "",
              "`fts_rare` matches one UniqueNeedle title; `fts_broad` matches approximately 10% of titles "
              "containing Signal; `fts_no_match` has zero matches. `keyset_midpoint` starts after the "
              "median UUID, and `favorites` selects the 5% favorite subset.", "",
              "| Tracks | Seed seconds | Database MiB |", "| ---: | ---: | ---: |"]
    for workload in report["workloads"]:
        lines.append(f"| {workload['tracks']:,} | {workload['seed_seconds']:.2f} | {workload['database_bytes'] / 1024 ** 2:.2f} |")
    lines += ["", "## Query plans", ""]
    largest = report["workloads"][-1]
    lines.append(f"Plans below come from the {largest['tracks']:,}-track database. Full SQL, parameters, "
                 "all workload plans and raw timing samples are emitted in the JSON report.")
    for name, case in largest["queries"].items():
        lines += ["", f"### {name}", "", "```text", *case["query_plan"], "```"]
    lines += ["", "## Reproduce", "", "```bash",
              "python3 scripts/benchmark_catalog.py --sizes 10000 100000 1000000 \\",
              f"  --samples {report['warm_samples']} --cold-samples {report['cold_samples']} \\",
              f"  --tmp-root {report['requested_temp_root'] or '/tmp'} \\",
              "  --output /tmp/vela-catalog-benchmark.json --write-doc docs/PERFORMANCE.md", "```", "",
              "The script creates and removes only its own temporary workload directories. "
              "It never opens a production database. At least roughly 1.8 GB of free storage is "
              "required for the million-entry workload, in addition to the rest of the host's workloads.", "",
              "## Limits and next measurements", "",
              "Single-process serial SQL results on this host do not establish VPS or ARM capacity. "
              "Search distribution, many owners, concurrent writes, a small OS cache and slower disks "
              "can change the results substantially. These are exploratory local measurements; "
              "no superiority or supported-listener-count claim follows from them.", "",
              "Still unmeasured: HTTP p50/p95/p99 under concurrency, streaming time to first byte, "
              "time to audible playback, peak server RSS, import/scanner concurrency, transcode CPU, "
              "remote search, federation and native playback. Repeat representative load tests on "
              "an idle host and constrained VPS/ARM hardware before claiming production capacity.", "",
              "Source fingerprints for this run:", "",
              "```json", json.dumps(report["source_sha256"], indent=2), "```", ""]
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sizes", nargs="+", type=int, default=[10000, 100000, 1000000])
    parser.add_argument("--samples", type=int, default=100)
    parser.add_argument("--cold-samples", type=int, default=10)
    parser.add_argument("--tmp-root", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--write-doc", type=Path)
    args = parser.parse_args()
    if not args.sizes or any(size < 100 or size > 1000000 for size in args.sizes):
        parser.error("workload sizes must be in 100..=1000000")
    if args.samples < 1 or args.cold_samples < 1:
        parser.error("sample counts must be positive")
    if args.tmp_root:
        args.tmp_root.mkdir(parents=True, exist_ok=True)
    projection = sql_projection()
    fingerprints = {str(path.relative_to(REPOSITORY)): hashlib.sha256(path.read_bytes()).hexdigest()
                    for path in [*sorted((REPOSITORY / "migrations").glob("*.sql")),
                                 REPOSITORY / "crates/media/src/lib.rs"]}
    report = {"created_at": datetime.now(timezone.utc).isoformat(), "hardware": hardware(args.tmp_root),
              "warm_samples": args.samples, "cold_samples": args.cold_samples,
              "requested_temp_root": str(args.tmp_root) if args.tmp_root else None,
              "source_sha256": fingerprints, "workloads": []}
    for count in sorted(set(args.sizes)):
        storage = args.tmp_root or Path(tempfile.gettempdir())
        if shutil.disk_usage(storage).free < count * 1800 + 64 * 1024 ** 2:
            raise RuntimeError(f"Insufficient free storage for {count:,}-entry isolated benchmark")
        with tempfile.TemporaryDirectory(prefix="vela-catalog-bench-", dir=args.tmp_root) as directory:
            database = Path(directory) / "vela.db"
            print(f"Starting {count:,}-track workload", file=sys.stderr, flush=True)
            workload = {"tracks": count, **seed_database(database, count), "queries": {}}
            cases = {
                "first_page": {},
                "keyset_midpoint": {"cursor": workload["midpoint_cursor"]},
                "favorites": {"favorites": True},
                "fts_rare": {"search": '"UniqueNeedle"*'},
                "fts_broad": {"search": '"Signal"*'},
                "fts_no_match": {"search": '"AbsentWordThatIsNotPresent"*'},
            }
            for name, options in cases.items():
                query, parameters = catalog_query(projection, **options)
                case = benchmark_case(database, query, parameters, args.samples, args.cold_samples)
                expected_rows = {
                    "first_page": min(PAGE_SIZE + 1, count),
                    "keyset_midpoint": min(PAGE_SIZE + 1, count - count // 2 - 1),
                    "favorites": min(PAGE_SIZE + 1, (count + 19) // 20),
                    "fts_rare": 1,
                    "fts_broad": min(PAGE_SIZE + 1, (count + 9) // 10),
                    "fts_no_match": 0,
                }[name]
                if case["returned_rows"] != expected_rows:
                    raise RuntimeError(f"Unexpected result count for {name}: "
                                       f"{case['returned_rows']} != {expected_rows}")
                workload["queries"][name] = case
                print(f"{count:,} {name}: warm p50={case['warm']['p50_ms']:.3f}ms "
                      f"p95={case['warm']['p95_ms']:.3f}ms", file=sys.stderr, flush=True)
            report["workloads"].append(workload)
    serialized = json.dumps(report, indent=2)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(serialized + "\n")
    else:
        print(serialized)
    if args.write_doc:
        args.write_doc.parent.mkdir(parents=True, exist_ok=True)
        args.write_doc.write_text(render_document(report))


if __name__ == "__main__":
    main()
