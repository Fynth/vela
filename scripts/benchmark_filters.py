#!/usr/bin/env python3
"""Measure typed filter/sort SQL on isolated synthetic metadata, never media.

All databases are new private temporary files. No existing database path can be
provided. Timings are Python/SQLite, not native SQLx, API or playback timings.
The CLI writes evidence only to explicitly requested report/document paths.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass, replace
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import random
import shutil
import sqlite3
import sys
import tempfile
import time
import uuid

from benchmark_catalog import (
    CACHE_KIB, OWNER, PAGE_SIZE, REPOSITORY, SEED, connect, hardware,
    sql_projection, summarize,
)


FOREIGN_OWNER = "filters-benchmark-other-owner"
BASELINE_INDEX_MIGRATION = REPOSITORY / "migrations/0005_catalog_filters.sql"
COVERING_INDEX_MIGRATION = REPOSITORY / "migrations/0006_cover_genre_facets.sql"
FILTER_INDEXES = (
    "tracks_owner_genre_filter", "tracks_owner_codec_filter",
    "tracks_owner_year_filter", "tracks_owner_title_order",
    "tracks_owner_newest_order",
)
PROJECTION = sql_projection()
PROBE_SQL = (
    "SELECT COUNT(*) FROM (SELECT rowid FROM tracks_fts "
    "WHERE tracks_fts MATCH ? LIMIT 4097)"
)


@dataclass(frozen=True)
class Case:
    sort: str = "library"
    genre: str | None = None
    codec: str | None = None
    year_from: int | None = None
    year_to: int | None = None
    min_bitrate: int | None = None
    min_sample_rate: int | None = None
    min_bit_depth: int | None = None
    favorites: bool = False
    expression: str | None = None
    cursor: str | None = None


def seed(database: Path, count: int, baseline_indexes: bool) -> dict:
    """Seed the old schema, then time the selected real index migrations."""
    connection = connect(database)
    for migration in sorted((REPOSITORY / "migrations").glob("*.sql")):
        if migration.name < BASELINE_INDEX_MIGRATION.name:
            connection.executescript(migration.read_text())
    for owner, username in [(OWNER, "filter-benchmark"),
                            (FOREIGN_OWNER, "other-filter-benchmark")]:
        connection.execute(
            "INSERT INTO users(id,username,password_hash) VALUES(?,?,?)",
            (owner, username, "synthetic benchmark; unusable password"),
        )
    connection.commit()
    rng = random.Random(SEED)
    genres = ["Ambient", "Classical", "Electronic", "Jazz", "Rock", "  Jazz  ", "jazz", ""]
    codecs = ["FLAC", "MP3", "Opus", "AAC", "Vorbis", "MP4", "WAV", "WavPack"]
    bitrates = [128, 192, 256, 320, 640, 960, 1411, None]
    sample_rates = [22050, 44100, 48000, 96000, None]
    depths = [16, 24, None]
    started = time.perf_counter()
    for owner_index, owner in enumerate((OWNER, FOREIGN_OWNER)):
        for start in range(0, count, 1000):
            assets, tracks, favorites = [], [], []
            for index in range(start, min(start + 1000, count)):
                identity = str(uuid.UUID(int=rng.getrandbits(128), version=4))
                # Synthetic identity only, never represented as an audio digest.
                content_hash = f"{owner_index * count + index:064x}"
                # Repeated mixed-case titles and second timestamps exercise ties.
                prefix = "Signal" if index % 10 == 0 else ("Track" if index % 2 else "track")
                title = f"{prefix} {(index // 10) % 512:04d}"
                if index == count // 2:
                    title += " UniqueNeedle" if owner == OWNER else " PrivateNeedle"
                genre = genres[index % len(genres)]
                codec = codecs[(index // 3) % len(codecs)]
                if owner == FOREIGN_OWNER and index == 0:
                    genre, codec = "Private only", "PRIVATE"
                assets.append((identity, content_hash, f"media/{content_hash}.audio", "audio/example", 5000000))
                tracks.append((
                    identity, owner, identity, title, f"Artist{index % 1000:04d}",
                    f"Album{index // 12:07d}", f"Artist{(index // 12) % 1000:04d}",
                    genre, None if index % 10 == 0 else 1980 + (index // 5) % 45,
                    180000, codec, bitrates[(index // 7) % len(bitrates)],
                    sample_rates[(index // 11) % len(sample_rates)],
                    depths[(index // 13) % len(depths)], 1700000000 + index // 16,
                ))
                if index % 20 == 0:
                    favorites.append((owner, identity))
            with connection:
                connection.executemany(
                    "INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) VALUES(?,?,?,?,?)", assets,
                )
                connection.executemany(
                    "INSERT INTO tracks(id,owner_id,media_asset_id,title,artist,album,album_artist,genre,year,duration_ms,codec,bitrate,sample_rate,bit_depth,added_at) "
                    "VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)", tracks,
                )
                connection.executemany("INSERT INTO favorites(user_id,track_id) VALUES(?,?)", favorites)
    seed_seconds = time.perf_counter() - started
    if connection.execute("SELECT count(*) FROM tracks").fetchone()[0] != 2 * count:
        raise RuntimeError("Synthetic seed count mismatch")
    connection.execute("PRAGMA wal_checkpoint(TRUNCATE)")
    before = database.stat().st_size
    index_migrations = [BASELINE_INDEX_MIGRATION]
    if not baseline_indexes:
        index_migrations.append(COVERING_INDEX_MIGRATION)
    migration_measurements = []
    for migration in index_migrations:
        migration_before = database.stat().st_size
        started = time.perf_counter()
        connection.executescript(migration.read_text())
        migration_seconds = time.perf_counter() - started
        connection.execute("PRAGMA wal_checkpoint(TRUNCATE)")
        migration_measurements.append({
            "migration": migration.name, "seconds": migration_seconds,
            "database_before_bytes": migration_before,
            "database_after_bytes": database.stat().st_size,
        })
    after = database.stat().st_size
    try:
        sizes = dict(connection.execute(
            "SELECT name,sum(pgsize) FROM dbstat GROUP BY name HAVING name IN (?,?,?,?,?)",
            FILTER_INDEXES,
        ))
    except sqlite3.OperationalError:
        sizes = None  # dbstat is not part of every Python SQLite distribution.
    foreign_cursor = connection.execute(
        "SELECT id FROM tracks WHERE owner_id=? LIMIT 1", (FOREIGN_OWNER,),
    ).fetchone()[0]
    connection.close()
    return {
        "owner_tracks": count, "private_distractor_tracks": count,
        "seed_before_filter_indexes_seconds": seed_seconds,
        "filter_index_migrations": migration_measurements,
        "filter_index_migration_seconds": sum(entry["seconds"] for entry in migration_measurements),
        "database_before_filter_indexes_bytes": before,
        "database_bytes": after, "filter_index_database_growth_bytes": after - before,
        "filter_index_dbstat_bytes": sizes, "foreign_cursor": foreign_cursor,
    }


def build_query(connection: sqlite3.Connection, case: Case) -> tuple[str | None, tuple, dict]:
    """Mirror list_tracks_filtered, including its bounded probe and key lookup.

    Keep this SQL mirror synchronized with media/src/lib.rs; source fingerprints
    make each report auditable. Values are bound, not interpolated into SQL.
    """
    boundary = None
    boundary_query = None
    if case.cursor is not None and case.sort != "library":
        boundary_query = "SELECT title,added_at FROM tracks WHERE owner_id=? AND id=?"
        row = connection.execute(boundary_query, (OWNER, case.cursor)).fetchone()
        if row is None:
            raise ValueError("Invalid track cursor")
        boundary = row[0 if case.sort == "title" else 1]
    probe_count = None
    if case.expression is not None:
        probe_count = connection.execute(PROBE_SQL, (case.expression,)).fetchone()[0]
        if probe_count == 0:
            return None, (), {"probe_matches_up_to_4097": 0, "boundary_sql": boundary_query}
    sparse = probe_count is not None and probe_count <= 4096
    projection = PROJECTION
    if sparse:
        projection = projection.replace(
            "FROM tracks t", "FROM tracks_fts CROSS JOIN tracks t ON t.rowid=tracks_fts.rowid",
        )
    query = projection + "? WHERE t.owner_id=?"
    parameters: list[str | int] = [OWNER, OWNER]
    if case.expression is not None:
        if sparse:
            query += " AND tracks_fts MATCH ?"
        else:
            query += " AND t.rowid IN (SELECT rowid FROM tracks_fts WHERE tracks_fts MATCH ?)"
        parameters.append(case.expression)
    if case.favorites:
        query += " AND f.track_id IS NOT NULL"
    if case.genre is not None:
        query += " AND trim(t.genre) COLLATE NOCASE=?"
        parameters.append(case.genre)
    if case.codec is not None:
        query += " AND t.codec COLLATE NOCASE=?"
        parameters.append(case.codec)
    for column, operator, value in [
        ("year", ">=", case.year_from),
        ("bitrate", ">=", case.min_bitrate), ("sample_rate", ">=", case.min_sample_rate),
        ("bit_depth", ">=", case.min_bit_depth), ("year", "<=", case.year_to),
    ]:
        if value is not None:
            query += f" AND t.{column} {operator} ?"
            parameters.append(value)
    if case.cursor is not None:
        if case.sort == "library":
            query += " AND t.id > ?"
            parameters.append(case.cursor)
        elif case.sort == "title":
            # RHS collation preserves ASCII NOCASE and lets SQLite seek the
            # complete indexed tuple; lhs COLLATE caused owner-prefix scans.
            query += " AND (t.title,t.id)>(? COLLATE NOCASE,?)"
            parameters.extend((boundary, case.cursor))
        else:
            query += " AND (t.added_at,t.id)<(?,?)"
            parameters.extend((boundary, case.cursor))
    query += {
        "library": " ORDER BY t.id ASC LIMIT ?",
        "title": " ORDER BY t.title COLLATE NOCASE ASC,t.id ASC LIMIT ?",
        "newest": " ORDER BY t.added_at DESC,t.id DESC LIMIT ?",
    }[case.sort]
    parameters.append(PAGE_SIZE + 1)
    return query, tuple(parameters), {
        "probe_matches_up_to_4097": probe_count, "sparse_fts": sparse,
        "boundary_sql": boundary_query,
    }


def execute_page(connection: sqlite3.Connection, case: Case) -> list[tuple]:
    connection.execute("BEGIN")
    try:
        query, parameters, _ = build_query(connection, case)
        rows = [] if query is None else connection.execute(query, parameters).fetchall()
        connection.commit()
        return rows
    except BaseException:
        connection.rollback()
        raise


def validate_rows(connection: sqlite3.Connection, rows: list[tuple], case: Case) -> None:
    if len(rows) > PAGE_SIZE + 1:
        raise RuntimeError("Unbounded result page")
    for row in rows:
        owner = connection.execute("SELECT owner_id FROM tracks WHERE id=?", (row[0],)).fetchone()[0]
        if owner != OWNER:
            raise RuntimeError("Foreign-owner row returned")
        if case.genre is not None and row[5].strip(" ").lower() != case.genre.lower():
            raise RuntimeError("Wrong genre result")
        if case.codec is not None and row[10].lower() != case.codec.lower():
            raise RuntimeError("Wrong codec result")
        for actual, operator, required in [
            (row[6], "min", case.year_from), (row[6], "max", case.year_to),
            (row[11], "min", case.min_bitrate), (row[12], "min", case.min_sample_rate),
            (row[13], "min", case.min_bit_depth),
        ]:
            if required is not None and (actual is None or
                    (operator == "min" and actual < required) or
                    (operator == "max" and actual > required)):
                raise RuntimeError("Unknown/out-of-range result returned")
        if case.favorites and not row[18]:
            raise RuntimeError("Nonfavorite returned")
    key = {
        "library": lambda row: row[0],
        "title": lambda row: (row[1].lower(), row[0]),
        "newest": lambda row: (row[17], row[0]),
    }[case.sort]
    if rows != sorted(rows, key=key, reverse=case.sort == "newest"):
        raise RuntimeError("Incorrect sort or tie ordering")


def benchmark_case(database: Path, case: Case, samples: int, cold_samples: int) -> dict:
    connection = connect(database)
    rows = execute_page(connection, case)
    validate_rows(connection, rows, case)
    query, parameters, metadata = build_query(connection, case)
    plan = [] if query is None else [row[3] for row in connection.execute("EXPLAIN QUERY PLAN " + query, parameters)]
    boundary_plan = []
    if metadata["boundary_sql"] is not None:
        boundary_plan = [row[3] for row in connection.execute(
            "EXPLAIN QUERY PLAN " + metadata["boundary_sql"], (OWNER, case.cursor),
        )]
    for _ in range(10):
        execute_page(connection, case)
    warm = []
    for _ in range(samples):
        started = time.perf_counter_ns()
        measured = execute_page(connection, case)
        warm.append((time.perf_counter_ns() - started) / 1e6)
        if measured != rows:
            raise RuntimeError("Nondeterministic query result")
    connection.close()
    cold = []
    for _ in range(cold_samples):
        connection = connect(database)
        started = time.perf_counter_ns()
        measured = execute_page(connection, case)
        cold.append((time.perf_counter_ns() - started) / 1e6)
        connection.close()
        if measured != rows:
            raise RuntimeError("Result changed across connections")
    return {
        "conditions": case.__dict__, "returned_rows": len(rows), "sql": query,
        "parameters": parameters, "query_plan": plan, "cursor_query_plan": boundary_plan,
        **metadata, "warm": summarize(warm), "sqlite_cache_reset": summarize(cold),
        "warm_samples_ms": warm, "sqlite_cache_reset_samples_ms": cold,
    }


def facets(connection: sqlite3.Connection) -> dict:
    # Explicit transaction keeps genre/codec reads in the same SQLite snapshot.
    connection.execute("BEGIN")
    try:
        genres = [row[0] for row in connection.execute(
            "SELECT MIN(trim(genre)) FROM tracks WHERE owner_id=? AND trim(genre)<>'' "
            "GROUP BY trim(genre) COLLATE NOCASE ORDER BY trim(genre) COLLATE NOCASE LIMIT 101", (OWNER,),
        )]
        codecs = [row[0] for row in connection.execute(
            "SELECT MIN(codec) FROM tracks WHERE owner_id=? AND trim(codec)<>'' "
            "GROUP BY codec COLLATE NOCASE ORDER BY codec COLLATE NOCASE LIMIT 33", (OWNER,),
        )]
    finally:
        connection.commit()
    if "Private only" in genres or "PRIVATE" in codecs:
        raise RuntimeError("Foreign facet disclosed")
    return {"genres": genres[:100], "codecs": codecs[:32],
            "genres_truncated": len(genres) > 100, "codecs_truncated": len(codecs) > 32}


def benchmark_facets(database: Path, samples: int, cold_samples: int) -> dict:
    connection = connect(database)
    expected = facets(connection)
    plans = {}
    for label, expression, limit in [("genres", "trim(genre)", 101), ("codecs", "codec", 33)]:
        sql = (f"SELECT MIN({expression}) FROM tracks WHERE owner_id=? AND trim({label[:-1]})<>'' "
               f"GROUP BY {expression} COLLATE NOCASE ORDER BY {expression} COLLATE NOCASE LIMIT {limit}")
        plans[label] = {"sql": sql, "query_plan": [row[3] for row in connection.execute("EXPLAIN QUERY PLAN " + sql, (OWNER,))]}
    for _ in range(10):
        facets(connection)
    warm, cold = [], []
    for _ in range(samples):
        started = time.perf_counter_ns()
        result = facets(connection)
        warm.append((time.perf_counter_ns() - started) / 1e6)
        if result != expected:
            raise RuntimeError("Facet snapshot differs")
    connection.close()
    for _ in range(cold_samples):
        connection = connect(database)
        started = time.perf_counter_ns()
        result = facets(connection)
        cold.append((time.perf_counter_ns() - started) / 1e6)
        connection.close()
        if result != expected:
            raise RuntimeError("Facet snapshot differs across connections")
    return {"values": expected, "plans": plans, "warm": summarize(warm),
            "sqlite_cache_reset": summarize(cold), "warm_samples_ms": warm,
            "sqlite_cache_reset_samples_ms": cold}


def workload(database: Path, count: int, samples: int, cold_samples: int,
             baseline_indexes: bool) -> dict:
    result = seed(database, count, baseline_indexes)
    foreign_cursor = result.pop("foreign_cursor")
    connection = connect(database)
    # A foreign cursor must be indistinguishable from a missing one for alternate
    # sort-key lookups. The production API has separate adversarial Rust tests.
    for sort in ("title", "newest"):
        for cursor in (foreign_cursor, "00000000-0000-4000-8000-000000000000"):
            try:
                execute_page(connection, Case(sort=sort, cursor=cursor))
            except ValueError as error:
                if str(error) != "Invalid track cursor":
                    raise
            else:
                raise RuntimeError("Foreign/missing cursor accepted")
    connection.close()
    cases = {
        "library": Case(), "genre": Case(genre="Jazz"), "format": Case(codec="FLAC"),
        "year_range": Case(year_from=2000, year_to=2020),
        "quality_minimums": Case(min_bitrate=320, min_sample_rate=44100, min_bit_depth=16),
        "combined": Case(genre="Jazz", codec="FLAC", year_from=1990, year_to=2024,
                         min_bitrate=192, min_sample_rate=44100, min_bit_depth=16),
        "favorite_genre": Case(genre="Rock", favorites=True),
        "title": Case(sort="title"), "newest": Case(sort="newest"),
        "genre_title": Case(genre="Jazz", sort="title"),
        "format_newest": Case(codec="FLAC", sort="newest"),
        "fts_rare": Case(expression='"UniqueNeedle"*'),
        "fts_broad_title": Case(expression='"Signal"*', sort="title"),
        "fts_broad_filtered": Case(expression='"Signal"*', codec="FLAC", sort="newest"),
        "fts_private_only": Case(expression='"PrivateNeedle"*'),
        "fts_absent": Case(expression='"AbsentWordThatIsNotPresent"*'),
        "private_genre": Case(genre="Private only"),
    }
    connection = connect(database)
    for sort, ordering in [("library", "id ASC"),
                           ("title", "title COLLATE NOCASE ASC,id ASC"),
                           ("newest", "added_at DESC,id DESC")]:
        # OFFSET selects an anchor during fixture preparation only. It is never
        # part of a measured page request or production pagination query.
        cursor = connection.execute(
            f"SELECT id FROM tracks WHERE owner_id=? ORDER BY {ordering} LIMIT 1 OFFSET ?",
            (OWNER, count // 2),
        ).fetchone()[0]
        cases[sort + "_midpoint"] = Case(sort=sort, cursor=cursor)
    connection.close()
    result["queries"] = {}
    for name, case in cases.items():
        connection = connect(database)
        first_rows = execute_page(connection, case)
        connection.close()
        variants = [(name if case.cursor else name + "_first", case)]
        if len(first_rows) > PAGE_SIZE:
            variants.append((name + "_next", replace(case, cursor=first_rows[PAGE_SIZE - 1][0])))
        for label, variant in variants:
            measurement = benchmark_case(database, variant, samples, cold_samples)
            if label.endswith("_next"):
                connection = connect(database)
                next_rows = execute_page(connection, variant)
                connection.close()
                if next_rows[:1] != first_rows[PAGE_SIZE:PAGE_SIZE + 1]:
                    raise RuntimeError(f"Sentinel missing or duplicated at page boundary: {label}")
                if set(row[0] for row in first_rows[:PAGE_SIZE]) & set(row[0] for row in next_rows):
                    raise RuntimeError(f"Repeated track at page boundary: {label}")
            if name in ("fts_private_only", "fts_absent", "private_genre") and measurement["returned_rows"]:
                raise RuntimeError("Expected no authorized results")
            result["queries"][label] = measurement
            print(f"{count:,} owner + {count:,} private: {label}: "
                  f"p50={measurement['warm']['p50_ms']:.3f}ms "
                  f"p95={measurement['warm']['p95_ms']:.3f}ms", file=sys.stderr, flush=True)
    result["facets"] = benchmark_facets(database, samples, cold_samples)
    return result


def document(report: dict) -> str:
    migration_names = ", ".join(report["index_migrations"])
    mode = "Baseline indexes (migration0005 only)" if report["baseline_indexes"] else "Covering genre facets (migrations0005+0006)"
    lines = ["# Typed catalog filter SQL measurements", "",
             f"Measured {report['created_at']} using `scripts/benchmark_filters.py`.", "",
             f"Index mode: **{mode}**. Applied index migrations: `{migration_names}`.", "",
             "Each workload uses the stated owner count plus an equal number of private tracks "
             "belonging to a second owner in a new temporary database. Deterministic synthetic "
             "metadata includes repeated mixed-case titles, timestamp ties, unknown properties, "
             "five genre labels with whitespace/case variants, eight actual container/codec labels "
             "and private-only search/facet distractors. No audio files or existing user data are used.", "",
             "The real migrations0001–0004, FTS/group triggers and batched seed writes run first. "
             "The selected real index migrations are timed separately against populated metadata. "
             "Seed timing thus excludes maintenance of the new five indexes; it is not media "
             "ingestion throughput. Index database growth is after WAL checkpoint; optional dbstat "
             "bytes account for the five final B-trees directly. Covering mode includes the cost "
             "of creating the original genre index in0005 and replacing it in0006, rather than "
             "silently timing only the final definition. Timed Python executescript calls exclude "
             "the following WAL checkpoint and are not native SQLx migration timings. Database "
             "file growth may include free pages left by an index replacement. "
             "No ANALYZE or VACUUM is run.", "",
             f"Queries use the current Track projection,50 rows plus a sentinel, bound filter values "
             f"and an owner predicate. SQLite page-cache target is{CACHE_KIB}KiB, WAL/FULL, mmap disabled. "
             "Each alternate-sort next-page timing includes its owner-scoped key lookup. FTS timings "
             "include the bounded 4097-row probe and sparse/broad choice. Both facet reads share a "
             "transaction. Ten untimed executions precede warm samples; cache-reset samples open a "
             "new SQLite connection but retain the host filesystem cache. Reported timings include "
             "Python execute/fetch and SQL mirror construction, and exclude connection setup, "
             "Rust SQLx/DTO mapping, authentication, HTTP/JSON, network, concurrent writes and UI. "
             "The script checks row ownership, criteria, ordering, tie-breaks, page sentinels, "
             "foreign cursor handling and nondisclosure of private-only facets/results.", "",
             "This SQL mirror is reviewed against the source fingerprints below; it is not an "
             "end-to-end server benchmark. Median anchors are selected before timing; fixture-only "
             "OFFSET is never part of a measured page. First, second and median pages do not "
             "characterize every worst-case distribution of repeated titles. "
             "Case folding in production remains SQLite ASCII NOCASE; no Unicode collation claim "
             "follows. Numeric thresholds do not establish losslessness. There is no product "
             "superiority, hardware-capacity or listener-count claim.", "",
             "```json", json.dumps(report["hardware"], indent=2), "```", "",
             f"Samples: {report['warm_samples']} warm and {report['cold_samples']} new-connection "
             "samples per case. Percentiles use nearest rank; small-sample p99 is approximate.", "",
             "| Owner / private tracks | Case | Warm p50 / p95 / p99 ms | Cache-reset p50 / p95 / p99 ms | Rows |",
             "| ---: | --- | ---: | ---: | ---: |"]
    def triple(value: dict) -> str:
        return " / ".join(f"{value[key]:.3f}" for key in ("p50_ms", "p95_ms", "p99_ms"))
    for entry in report["workloads"]:
        for name, result in entry["queries"].items():
            lines.append(f"| {entry['owner_tracks']:,} / {entry['private_distractor_tracks']:,} | {name} | "
                         f"{triple(result['warm'])} | {triple(result['sqlite_cache_reset'])} | {result['returned_rows']} |")
        result = entry["facets"]
        lines.append(f"| {entry['owner_tracks']:,} / {entry['private_distractor_tracks']:,} | facets snapshot | "
                     f"{triple(result['warm'])} | {triple(result['sqlite_cache_reset'])} | bounded values |")
    lines += ["", "| Owner / private tracks | Seed seconds (old indexes) | New index build seconds | DB MiB | New B-tree MiB |",
              "| ---: | ---: | ---: | ---: | ---: |"]
    for entry in report["workloads"]:
        sizes = entry["filter_index_dbstat_bytes"]
        size = f"{sum(sizes.values()) / 1024 ** 2:.2f}" if sizes else "unavailable"
        lines.append(f"| {entry['owner_tracks']:,} / {entry['private_distractor_tracks']:,} | "
                     f"{entry['seed_before_filter_indexes_seconds']:.3f} | {entry['filter_index_migration_seconds']:.3f} | "
                     f"{entry['database_bytes'] / 1024 ** 2:.2f} | {size} |")
    lines += ["", "| Owner / private tracks | Index migration | Seconds | DB before / after MiB |",
              "| ---: | --- | ---: | ---: |"]
    for entry in report["workloads"]:
        for migration in entry["filter_index_migrations"]:
            lines.append(f"| {entry['owner_tracks']:,} / {entry['private_distractor_tracks']:,} | "
                         f"{migration['migration']} | {migration['seconds']:.3f} | "
                         f"{migration['database_before_bytes'] / 1024 ** 2:.2f} / "
                         f"{migration['database_after_bytes'] / 1024 ** 2:.2f} |")
    lines += ["", "## Query plans for the largest workload", ""]
    for name, result in report["workloads"][-1]["queries"].items():
        lines += [f"### {name}", "", "```text", *result["query_plan"], "```", ""]
    for name, result in report["workloads"][-1]["facets"]["plans"].items():
        lines += [f"### Facet {name}", "", "```text", *result["query_plan"], "```", ""]
    lines += ["## Reproduce", "", "```bash",
              "python3 scripts/benchmark_filters.py --sizes 10000 100000 \\",
              *(["  --baseline-indexes \\"] if report["baseline_indexes"] else []),
              f"  --samples {report['warm_samples']} --cold-samples {report['cold_samples']} \\",
              "  --tmp-root target/filter-bench \\",
              "  --output target/checks/catalog-filters.json \\",
              "  --write-doc target/checks/catalog-filters.md", "```", "",
              "Full SQL, bindings, probe cardinalities, plans, raw samples and per-index sizes "
              "are retained in the JSON report. Temporary databases are removed automatically. "
              "Re-run on idle constrained hosts and measure real HTTP/concurrent workloads before "
              "making operational recommendations.", "", "Source fingerprints:", "", "```json",
              json.dumps(report["source_sha256"], indent=2), "```", ""]
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sizes", nargs="+", type=int, default=[10000, 100000])
    parser.add_argument("--samples", type=int, default=100)
    parser.add_argument("--cold-samples", type=int, default=10)
    parser.add_argument("--tmp-root", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--write-doc", type=Path)
    parser.add_argument("--baseline-indexes", action="store_true",
                        help="Apply only migration0005 indexes; default also applies covering migration0006")
    args = parser.parse_args()
    if any(size < 100 or size > 1000000 for size in args.sizes):
        parser.error("sizes must be in 100..1000000")
    if not 1 <= args.samples <= 10000 or not 1 <= args.cold_samples <= 1000:
        parser.error("sample counts must be in 1..10000 and 1..1000")
    if args.tmp_root:
        args.tmp_root.mkdir(parents=True, exist_ok=True)
    files = [*sorted((REPOSITORY / "migrations").glob("*.sql")),
             REPOSITORY / "crates/media/src/lib.rs", REPOSITORY / "crates/media/src/filters.rs", Path(__file__).resolve(),
             REPOSITORY / "scripts/benchmark_catalog.py"]
    report = {
        "created_at": datetime.now(timezone.utc).isoformat(), "hardware": hardware(args.tmp_root),
        "warm_samples": args.samples, "cold_samples": args.cold_samples,
        "baseline_indexes": args.baseline_indexes,
        "index_mode": "baseline" if args.baseline_indexes else "covering_genre",
        "index_migrations": [BASELINE_INDEX_MIGRATION.name] +
                            ([] if args.baseline_indexes else [COVERING_INDEX_MIGRATION.name]),
        "source_sha256": {str(path.relative_to(REPOSITORY)): hashlib.sha256(path.read_bytes()).hexdigest() for path in files},
        "workloads": [],
    }
    for count in sorted(set(args.sizes)):
        storage = args.tmp_root or Path(tempfile.gettempdir())
        if shutil.disk_usage(storage).free < count * 6000 + 64 * 1024 ** 2:
            raise RuntimeError(f"Insufficient free storage for isolated {count:,}+{count:,} workload")
        with tempfile.TemporaryDirectory(prefix="vela-filter-bench-", dir=args.tmp_root) as directory:
            print(f"Starting isolated {count:,}+{count:,} metadata workload", file=sys.stderr, flush=True)
            report["workloads"].append(workload(Path(directory) / "vela.db", count, args.samples,
                                               args.cold_samples, args.baseline_indexes))
    serialized = json.dumps(report, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(serialized)
    else:
        print(serialized, end="")
    if args.write_doc:
        args.write_doc.parent.mkdir(parents=True, exist_ok=True)
        args.write_doc.write_text(document(report))


if __name__ == "__main__":
    main()
