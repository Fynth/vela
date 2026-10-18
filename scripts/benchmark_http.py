#!/usr/bin/env python3
"""Isolated HTTP read-path load benchmark for Vela (no external load tools).

Starts a fresh server in a private temporary data directory, creates a throwaway
account through the real setup API, uploads synthetic WAVs and/or seeds a large
synthetic catalog, then drives concurrent authenticated GETs for a fixed duration
and reports latency percentiles/RPS.

Large-catalog mode (`--seed-tracks N`) starts the isolated server once so the real
migrations and the real setup API create the schema and account. The server is
then stopped gracefully and N deterministic metadata-only media_assets/tracks rows
owned by that account are inserted with sqlite3; no audio files exist for those
rows and they are never streamed. The server is restarted on the same port and
data directory, and the saved session cookie must still authenticate, which proves
the session survived the restart. `--seed-tracks 0` keeps the original
upload-only behavior.

This never opens the live database, never uses the live unit and never writes to
the user's library. Only a private TemporaryDirectory is accepted. Python's client
is the load generator, so absolute RPS is a client-bound lower bound, not a server
capacity claim.

The concurrent loop keeps the original read endpoints (health, me, tracks, search,
albums). Aggregate/grouping reads `/api/stats` and `/api/artists?limit=50` are
probed serially with a long timeout instead, because their real cost on a large
synthetic catalog exceeds the concurrent client timeout and would otherwise be
reported as client timeouts. Both probes and their sample counts stay in the
report.
"""
from __future__ import annotations

import argparse
import hashlib
import http.client
import json
import math
import os
from pathlib import Path
import random
import secrets
import shutil
import signal
import socket
import sqlite3
import statistics
import struct
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parents[1]
SEED = 0x56454C41
SEED_BATCH = 1000
GENRES = ("Electronic", "Jazz", "Classical", "Rock", "Ambient")


def free_port() -> int:
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        return probe.getsockname()[1]


def percentile(values: list[float], fraction: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    index = min(len(ordered) - 1, max(0, math.ceil(len(ordered) * fraction) - 1))
    return ordered[index]


def digest_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def wav(seconds: float, frequency: float) -> bytes:
    rate = 44_100
    samples = int(rate * seconds)
    payload = bytearray()
    for sample in range(samples):
        value = int(2_000 * math.sin(2 * math.pi * frequency * sample / rate))
        payload += struct.pack("<h", value)
    header = bytearray(b"RIFF\x00\x00\x00\x00WAVEfmt ")
    header += struct.pack("<IHHIIHH", 16, 1, 1, rate, rate * 2, 2, 16)
    header += b"data" + struct.pack("<I", len(payload))
    result = header + payload
    result[4:8] = struct.pack("<I", len(result) - 8)
    return bytes(result)


def migration_version() -> int:
    versions = [int(path.name.split("_", 1)[0]) for path in (ROOT / "migrations").glob("*.sql")]
    if not versions:
        raise RuntimeError("No repository migrations found")
    return max(versions)


def ensure_isolated(directory: Path) -> Path:
    """Refuse any benchmark data directory that is not a private temporary path."""
    resolved = directory.resolve()
    temporary = Path(tempfile.gettempdir()).resolve()
    if not resolved.is_relative_to(temporary):
        raise RuntimeError(f"Refusing non-temporary benchmark data directory: {resolved}")
    for protected in (ROOT / "data", ROOT / "target", ROOT / "dist"):
        guarded = protected.resolve()
        if resolved == guarded or resolved.is_relative_to(guarded):
            raise RuntimeError(f"Refusing benchmark data inside live path: {resolved}")
    return resolved


def open_json(opener: urllib.request.OpenerDirector, url: str, cookie: str = "",
              expected: int = 200) -> dict:
    headers = {"Accept": "application/json"}
    if cookie:
        headers["Cookie"] = cookie
    request = urllib.request.Request(url, headers=headers)
    try:
        with opener.open(request, timeout=30) as response:
            status = response.status
            payload = json.load(response)
    except urllib.error.HTTPError as error:
        status = error.code
        payload = None
    if status != expected:
        raise RuntimeError(f"Unexpected HTTP {status} for {url}")
    return payload if payload is not None else {}


class Server:
    """One isolated native server lifetime on a fixed port and temporary data dir."""

    def __init__(self, binary: Path, fixture: Path, port: int):
        self.binary = binary
        self.fixture = fixture
        self.data = fixture / "data"
        self.port = port
        self.process: subprocess.Popen | None = None
        self.log = None

    @property
    def origin(self) -> str:
        return f"http://127.0.0.1:{self.port}"

    def start(self, opener: urllib.request.OpenerDirector) -> dict:
        # Ignore inherited Vela settings, especially live data roots and public bind.
        environment = {key: value for key, value in os.environ.items()
                       if not key.startswith("VELA_")}
        environment.update({
            "VELA_BIND": f"127.0.0.1:{self.port}",
            "VELA_DATA_DIR": str(self.data),
            "VELA_WEB_DIR": str(ROOT / "dist"),
            "VELA_SCAN_ROOTS": "[]",
            "NO_COLOR": "1",
        })
        self.log = (self.fixture / "server.log").open("ab")
        self.process = subprocess.Popen(
            [str(self.binary)], cwd=self.fixture, env=environment,
            stdout=self.log, stderr=subprocess.STDOUT,
        )
        deadline = time.monotonic() + 30
        while True:
            try:
                with opener.open(self.origin + "/health", timeout=1) as response:
                    if response.status == 200:
                        return json.load(response)
            except OSError:
                pass
            if self.process.poll() is not None:
                raise RuntimeError(f"Isolated server exited before readiness:\n{self.log_tail()}")
            if time.monotonic() > deadline:
                raise RuntimeError(f"Isolated server readiness timed out:\n{self.log_tail()}")
            time.sleep(0.05)

    def stop(self) -> int | None:
        """Stop gracefully and return the exit code (None when nothing was running)."""
        if self.process is None:
            return None
        if self.process.poll() is None:
            self.process.send_signal(signal.SIGTERM)
            try:
                self.process.wait(timeout=30)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait()
        code = self.process.returncode
        self.process = None
        if self.log is not None:
            self.log.close()
            self.log = None
        return code

    def log_tail(self, limit: int = 8192) -> str:
        path = self.fixture / "server.log"
        if not path.exists():
            return "(no server log)"
        return path.read_bytes()[-limit:].decode(errors="replace")


def stop_checked(server: Server) -> int:
    code = server.stop()
    if code != 0:
        raise RuntimeError(f"Isolated server exited with code {code}:\n{server.log_tail()}")
    return code


def bootstrap(opener: urllib.request.OpenerDirector, origin: str) -> dict:
    password = secrets.token_urlsafe(24)
    request = urllib.request.Request(
        origin + "/api/setup",
        data=json.dumps({"username": "benchmark", "password": password}).encode(),
        headers={"Content-Type": "application/json", "Origin": origin},
    )
    with opener.open(request, timeout=20) as response:
        auth = json.load(response)
        cookie = response.headers.get("Set-Cookie", "").split(";", 1)[0]
    if "vela_session=" not in cookie:
        raise RuntimeError("setup did not return a session cookie")
    owner = auth.get("user", {}).get("id")
    if not isinstance(owner, str) or not owner:
        raise RuntimeError("setup did not return an account id")
    # The cookie must already authenticate before any restart is attempted.
    me = open_json(opener, origin + "/api/me", cookie)
    if me.get("user", {}).get("id") != owner:
        raise RuntimeError("setup session did not authenticate its own account")
    return {"owner_id": owner, "username": "benchmark", "password": password,
            "cookie": cookie, "csrf": auth.get("csrf_token", "")}


def upload_wavs(opener: urllib.request.OpenerDirector, origin: str, session: dict,
                count: int) -> int:
    for index in range(count):
        boundary = "VelaBench" + secrets.token_hex(12)
        audio = wav(0.5, 300 + index)
        body = (
            f"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; "
            f"filename=\"Bench {index:02}.wav\"\r\nContent-Type: audio/wav\r\n\r\n"
        ).encode() + audio + f"\r\n--{boundary}--\r\n".encode()
        upload = urllib.request.Request(
            origin + "/api/upload",
            data=body,
            headers={
                "Content-Type": f"multipart/form-data; boundary={boundary}",
                "X-CSRF-Token": session["csrf"],
                "Origin": origin,
                "Cookie": session["cookie"],
            },
        )
        with opener.open(upload, timeout=60) as response:
            if response.status != 200:
                raise RuntimeError(f"upload {index} failed")
    return count


def sqlite_writer_version(database: Path) -> str:
    with database.open("rb") as stream:
        header = stream.read(100)
    if not header.startswith(b"SQLite format 3\x00"):
        raise RuntimeError("Missing native-created SQLite header")
    number = int.from_bytes(header[96:100], "big")
    return f"{number // 1_000_000}.{number // 1000 % 1000}.{number % 1000}"


def seed_catalog(database: Path, owner: str, count: int, uploaded: int) -> dict:
    """Insert deterministic synthetic metadata only; the native server is stopped."""
    if not database.is_file():
        raise RuntimeError("Isolated database created by the native server is missing")
    native_sqlite = sqlite_writer_version(database)
    connection = sqlite3.connect(database, timeout=30)
    connection.execute("PRAGMA foreign_keys=ON")
    connection.execute("PRAGMA journal_mode=WAL")
    connection.execute("PRAGMA synchronous=FULL")
    connection.execute("PRAGMA busy_timeout=30000")
    schema = connection.execute(
        "SELECT max(version) FROM _sqlx_migrations WHERE success=1").fetchone()[0]
    expected_schema = migration_version()
    if schema != expected_schema:
        raise RuntimeError(f"Expected native-embedded schema{expected_schema}, found {schema}")
    if connection.execute("SELECT COUNT(*) FROM users WHERE id=?", (owner,)).fetchone()[0] != 1:
        raise RuntimeError("Benchmark owner id is missing from the isolated database")
    baseline = {
        "tracks": connection.execute(
            "SELECT COUNT(*) FROM tracks WHERE owner_id=?", (owner,)).fetchone()[0],
        "media_assets": connection.execute("SELECT COUNT(*) FROM media_assets").fetchone()[0],
        "artists": connection.execute("SELECT COUNT(*) FROM artists").fetchone()[0],
        "albums": connection.execute("SELECT COUNT(*) FROM albums").fetchone()[0],
    }
    if baseline["tracks"] != uploaded or baseline["media_assets"] != uploaded:
        raise RuntimeError("Uploaded rows do not match the isolated account baseline")
    # Same deterministic shape as scripts/benchmark_catalog.py: synthetic UUIDv4 ids,
    # unique content hashes, 10% 'Signal' titles, 1000 artist labels, albums of 12,
    # five genres, 180s FLAC at 5,000,000 bytes. No audio file is created or streamed.
    rng = random.Random(SEED)
    insert_assets = ("INSERT INTO media_assets"
                     "(id,hash,storage_path,content_type,size_bytes) VALUES(?,?,?,?,?)")
    insert_tracks = ("INSERT INTO tracks"
                     "(id,owner_id,media_asset_id,title,artist,album,album_artist,"
                     "genre,duration_ms,codec) VALUES(?,?,?,?,?,?,?,?,?,?)")
    started = time.perf_counter()
    for start in range(0, count, SEED_BATCH):
        assets, tracks = [], []
        for index in range(start, min(start + SEED_BATCH, count)):
            identity = str(uuid.UUID(int=rng.getrandbits(128), version=4))
            content_hash = f"{index:064x}"  # Synthetic identity; not an audio checksum.
            title = f"{'Signal' if index % 10 == 0 else 'Track'} {index:07d}"
            assets.append((identity, content_hash, f"media/{content_hash}.audio",
                           "audio/flac", 5000000))
            tracks.append((identity, owner, identity, title, f"Artist{index % 1000:04d}",
                           f"Album{index // 12:07d}", f"Artist{(index // 12) % 1000:04d}",
                           GENRES[index % len(GENRES)], 180000, "FLAC"))
        with connection:  # one bounded transaction per batch
            connection.executemany(insert_assets, assets)
            connection.executemany(insert_tracks, tracks)
        if (start + SEED_BATCH) % 10000 == 0:
            print(f"seeded {min(start + SEED_BATCH, count):,}/{count:,} synthetic tracks",
                  file=sys.stderr, flush=True)
    insert_seconds = time.perf_counter() - started
    verify_started = time.perf_counter()
    observed = {
        "tracks": connection.execute(
            "SELECT COUNT(*) FROM tracks WHERE owner_id=?", (owner,)).fetchone()[0],
        "media_assets": connection.execute("SELECT COUNT(*) FROM media_assets").fetchone()[0],
        "artists": connection.execute("SELECT COUNT(*) FROM artists").fetchone()[0],
        "albums": connection.execute("SELECT COUNT(*) FROM albums").fetchone()[0],
        "signal_titles": connection.execute(
            "SELECT COUNT(*) FROM tracks WHERE owner_id=? AND title LIKE 'Signal %'",
            (owner,)).fetchone()[0],
        "fts_matches": connection.execute(
            "SELECT COUNT(*) FROM tracks_fts WHERE tracks_fts MATCH ?",
            ('"Signal"*',)).fetchone()[0],
    }
    expected = {
        "tracks": uploaded + count,
        "media_assets": uploaded + count,
        "artists": baseline["artists"] + min(count, 1000),
        "albums": baseline["albums"] + (count + 11) // 12,
        "signal_titles": (count + 9) // 10,
        "fts_matches": (count + 9) // 10,
    }
    if observed["tracks"] != expected["tracks"] or observed["media_assets"] != expected["media_assets"]:
        raise RuntimeError(f"Synthetic seed count mismatch: {observed} != {expected}")
    if observed["artists"] != expected["artists"] or observed["albums"] != expected["albums"]:
        raise RuntimeError(f"Synthetic grouping mismatch: {observed} != {expected}")
    if observed["signal_titles"] != expected["signal_titles"]:
        raise RuntimeError(f"Synthetic Signal title mismatch: {observed['signal_titles']}")
    if observed["fts_matches"] != expected["fts_matches"]:
        raise RuntimeError(f"FTS index built by Python seed mismatch: {observed['fts_matches']}")
    connection.execute("INSERT INTO tracks_fts(tracks_fts) VALUES('integrity-check')")
    connection.commit()
    quick_check = connection.execute("PRAGMA quick_check").fetchone()[0]
    if quick_check != "ok":
        raise RuntimeError(f"Seeded fixture failed SQLite quick_check: {quick_check}")
    foreign_keys = connection.execute("PRAGMA foreign_key_check").fetchall()
    if foreign_keys:
        raise RuntimeError("Seeded fixture has invalid foreign keys")
    verify_seconds = time.perf_counter() - verify_started
    connection.commit()
    connection.execute("PRAGMA wal_checkpoint(TRUNCATE)")
    connection.close()
    return {"mode": "metadata-only synthetic rows; no audio files created",
            "schema_version": schema,
            "native_sqlite_header_last_writer_version": native_sqlite,
            "batch_size": SEED_BATCH,
            "inserted_tracks": count, "uploaded_tracks": uploaded,
            "insert_seconds": round(insert_seconds, 3),
            "verify_seconds": round(verify_seconds, 3),
            "total_seconds": round(insert_seconds + verify_seconds, 3),
            "verified": {name: observed[name] for name in observed},
            "expected": expected,
            "fts_integrity_check": "ok", "quick_check": quick_check,
            "foreign_key_violations": len(foreign_keys),
            "database_bytes": database.stat().st_size}


def verify_restart(opener: urllib.request.OpenerDirector, origin: str, session: dict,
                   uploaded: int, seeded: int) -> dict:
    """Reuse the pre-restart cookie and confirm the seeded catalog is readable."""
    cookie = session["cookie"]
    me = open_json(opener, origin + "/api/me", cookie)
    if me.get("user", {}).get("id") != session["owner_id"]:
        raise RuntimeError("/api/me did not return the pre-restart account after restart")
    try:
        open_json(opener, origin + "/api/me", expected=401)
    except RuntimeError as error:
        raise RuntimeError(f"Anonymous /api/me was not rejected: {error}") from error
    stats = open_json(opener, origin + "/api/stats", cookie)
    expected_tracks = uploaded + seeded
    if stats.get("tracks") != expected_tracks:
        raise RuntimeError(f"Owner stats tracks {stats.get('tracks')} != {expected_tracks}")
    tracks = open_json(opener, origin + "/api/tracks?limit=50", cookie)
    albums = open_json(opener, origin + "/api/albums?limit=50", cookie)
    for name, page in (("tracks", tracks), ("albums", albums)):
        total = stats.get(name)
        expected_items = min(50, total)
        if len(page.get("items", [])) != expected_items:
            raise RuntimeError(f"Seeded /api/{name} page returned "
                               f"{len(page.get('items', []))} of {expected_items} rows")
        if bool(page.get("next_cursor")) != (total > 50):
            raise RuntimeError(f"Seeded /api/{name} page cursor did not match {total} rows")
    search = open_json(opener, origin + "/api/search?q=Signal", cookie)
    if len(search.get("tracks", [])) != 8:
        raise RuntimeError("Seeded /api/search did not return the bounded 8 track matches")
    return {"session_cookie_reused": True, "session_survived_restart": True,
            "me_user_id": me["user"]["id"], "anonymous_me_status": 401,
            "stats": {key: stats.get(key) for key in ("tracks", "albums", "artists")},
            "tracks_page_items": len(tracks["items"]),
            "albums_page_items": len(albums["items"]),
            "search_signal": {"tracks": len(search["tracks"]),
                              "albums": len(search.get("albums", [])),
                              "artists": len(search.get("artists", []))}}


def measure(opener: urllib.request.OpenerDirector, server: Server, session: dict,
            args: argparse.Namespace, report: dict, search_query: str) -> None:
    port = server.port
    cookie = session["cookie"]
    endpoints = [
        ("health", "/health"),
        ("me", "/api/me"),
        ("tracks", "/api/tracks?limit=50"),
        ("search", f"/api/search?q={urllib.parse.quote(search_query)}"),
        ("albums", "/api/albums"),
    ]

    def one(path: str) -> tuple[bool, float]:
        started = time.perf_counter()
        connection = http.client.HTTPConnection("127.0.0.1", port, timeout=10)
        try:
            connection.request("GET", path, headers={"Cookie": cookie, "Accept": "application/json"})
            response = connection.getresponse()
            response.read()
            ok = response.status == 200
        except OSError:
            ok = False
        finally:
            connection.close()
        return ok, (time.perf_counter() - started) * 1000.0

    for name, path in endpoints:
        # Warmup so lazy pools/statement caches are not attributed to p99.
        for _ in range(3):
            one(path)
        latencies: list[float] = []
        errors = 0
        lock = threading.Lock()
        finish = time.monotonic() + args.duration

        def worker() -> None:
            nonlocal errors
            local: list[float] = []
            while time.monotonic() < finish:
                ok, elapsed = one(path)
                if ok:
                    local.append(elapsed)
                else:
                    errors += 1
            with lock:
                latencies.extend(local)

        workers = [threading.Thread(target=worker) for _ in range(args.threads)]
        started = time.perf_counter()
        for thread in workers:
            thread.start()
        for thread in workers:
            thread.join()
        elapsed = time.perf_counter() - started
        total = len(latencies)
        report["endpoints"][name] = {
            "path": path,
            "requests": total,
            "requests_per_second": round(total / elapsed, 2),
            "errors": errors,
            "p50_ms": round(percentile(latencies, 0.50), 3),
            "p95_ms": round(percentile(latencies, 0.95), 3),
            "p99_ms": round(percentile(latencies, 0.99), 3),
            "max_ms": round(max(latencies, default=0.0), 3),
            "mean_ms": round(statistics.fmean(latencies), 3) if latencies else 0.0,
        }
        print(json.dumps({name: report["endpoints"][name]}), flush=True)


def probe_grouped_reads(server: Server, session: dict, report: dict, samples: int,
                        timeout: float) -> None:
    """Aggregate/grouping reads are probed serially, not in the concurrent loop.

    `/api/stats` and `/api/artists` aggregate per group and can exceed the 10s
    concurrent client timeout on a large synthetic catalog; timing them inside the
    8-thread loop would only measure client timeouts and could keep the server from
    draining on SIGTERM. One fresh connection per request keeps this comparable to
    the timed loop while the serial request count stays explicit in the report.
    """
    port = server.port
    cookie = session["cookie"]
    stats_payload: dict = {}
    for name, path in (("stats", "/api/stats"), ("artists", "/api/artists?limit=50")):
        latencies, sizes, statuses = [], [], []
        body = b""
        for _ in range(samples):
            connection = http.client.HTTPConnection("127.0.0.1", port, timeout=timeout)
            started = time.perf_counter()
            try:
                connection.request("GET", path,
                                   headers={"Cookie": cookie, "Accept": "application/json"})
                response = connection.getresponse()
                body = response.read()
                statuses.append(response.status)
                sizes.append(len(body))
            finally:
                connection.close()
            latencies.append((time.perf_counter() - started) * 1000.0)
        if any(status != 200 for status in statuses):
            raise RuntimeError(f"Diagnostic read {path} failed: {statuses}")
        payload = json.loads(body)
        if name == "stats":
            stats_payload = payload
            if payload.get("tracks") != report["catalog_tracks"]:
                raise RuntimeError(f"Diagnostic /api/stats tracks {payload.get('tracks')} "
                                   f"!= {report['catalog_tracks']}")
        else:
            artists = stats_payload.get("artists")
            if artists is None or len(payload.get("items", [])) != min(50, artists):
                raise RuntimeError("Diagnostic /api/artists page did not match the artist count")
        report["diagnostic_requests"][name] = {
            "path": path, "samples": samples, "status": statuses[0],
            "response_bytes": sizes[-1], "client_timeout_seconds": timeout,
            "samples_ms": [round(value, 3) for value in latencies],
            "p50_ms": round(statistics.median(latencies), 3),
            "max_ms": round(max(latencies), 3),
        }
        print(json.dumps({name: report["diagnostic_requests"][name]}), flush=True)


def run(args: argparse.Namespace, binary: Path, report: dict,
        opener: urllib.request.OpenerDirector) -> None:
    search_query = "Signal" if args.seed_tracks else "Bench"
    report["search_query"] = search_query
    temporary_root = Path(tempfile.gettempdir()).resolve()
    if args.seed_tracks:
        free = shutil.disk_usage(temporary_root).free
        report["temporary_storage_free_bytes"] = free
        required = args.seed_tracks * 2_000 + 256 * 1024 ** 2
        if free < required:
            raise RuntimeError(f"Insufficient temporary storage for {args.seed_tracks:,} "
                               f"seeded tracks ({free} < {required} bytes)")
    fixture = tempfile.mkdtemp(prefix="vela-http-bench-")
    try:
        root = Path(fixture)
        data = ensure_isolated(root / "data")
        data.mkdir()
        server = Server(binary, root, free_port())
        report["origin"] = server.origin
        try:
            report["health"] = server.start(opener)
            session = bootstrap(opener, server.origin)
            report["owner_user_id"] = session["owner_id"]
            report["csrf_present"] = bool(session["csrf"])
            report["uploaded_tracks"] = upload_wavs(opener, server.origin, session, args.tracks)
            if args.seed_tracks:
                report["initial_stop_exit_code"] = stop_checked(server)
                report["seed"] = seed_catalog(data / "vela.db", session["owner_id"],
                                              args.seed_tracks, report["uploaded_tracks"])
                report["restart"] = {"port": server.port, "port_reused": True,
                                     "graceful_stop_exit_code": report["initial_stop_exit_code"]}
                try:
                    health = server.start(opener)
                except RuntimeError as error:
                    if server.process is None or server.process.poll() is None:
                        raise
                    server.stop()
                    replacement = free_port()
                    print(f"warning: same-port restart failed ({error}); "
                          f"retrying on {replacement}", file=sys.stderr, flush=True)
                    server.port = replacement
                    report["restart"].update({"port": replacement, "port_reused": False})
                    health = server.start(opener)
                report["restart"]["health_unchanged"] = health == report["health"]
                report["restart"]["verification"] = verify_restart(
                    opener, server.origin, session,
                    report["uploaded_tracks"], args.seed_tracks)
                report["catalog_tracks"] = report["uploaded_tracks"] + args.seed_tracks
            else:
                report["restart"] = {"mode": "upload-only; the server is never restarted"}
                report["catalog_tracks"] = report["uploaded_tracks"]
            measure(opener, server, session, args, report, search_query)
            print("probing serial aggregate/grouping reads; /api/artists can take tens of "
                  "seconds on a large synthetic catalog", file=sys.stderr, flush=True)
            probe_grouped_reads(server, session, report, args.probe_samples, args.probe_timeout)
        finally:
            active = sys.exception()
            code = server.stop()
            report["final_stop_exit_code"] = code
            if active is None and code != 0:
                raise RuntimeError(f"Isolated server exited with code {code}:\n{server.log_tail()}")
    finally:
        shutil.rmtree(fixture, ignore_errors=True)
        removed = not Path(fixture).exists()
        database_gone = not (Path(fixture) / "data" / "vela.db").exists()
        report["cleanup"] = {"temporary_directory": fixture, "removed": removed,
                             "temporary_database_absent": database_gone}
        # The live tree is never a candidate; assert the temporary tree is really gone.
        if sys.exception() is None and (not removed or not database_gone):
            raise RuntimeError(f"Temporary benchmark data survived cleanup: {fixture}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/vela-server")
    parser.add_argument("--tracks", type=int, default=25,
                        help="synthetic WAV uploads (0..200; 0 keeps only seeded metadata)")
    parser.add_argument("--seed-tracks", type=int, default=0,
                        help="metadata-only seeded tracks owned by the benchmark account "
                             "(0 disables; up to1000000)")
    parser.add_argument("--threads", type=int, default=16)
    parser.add_argument("--duration", type=float, default=10.0, help="seconds per endpoint")
    parser.add_argument("--probe-samples", type=int, default=2,
                        help="serial samples per aggregate/grouping read (1..3)")
    parser.add_argument("--probe-timeout", type=float, default=180.0,
                        help="client timeout seconds for aggregate/grouping reads (10..600)")
    parser.add_argument("--output", type=Path, default=Path("/tmp/vela-http-bench.json"))
    args = parser.parse_args()
    if (not 0 <= args.tracks <= 200 or not 0 <= args.seed_tracks <= 1_000_000
            or not 1 <= args.threads <= 256 or not 1 <= args.duration <= 120
            or not 1 <= args.probe_samples <= 3 or not 10 <= args.probe_timeout <= 600):
        parser.error("tracks0..200, seed-tracks0..1000000, threads1..256, duration1..120, "
                     "probe-samples1..3, probe-timeout10..600")
    if args.tracks == 0 and args.seed_tracks == 0:
        parser.error("needs --tracks>0, --seed-tracks>0, or both")
    binary = args.binary.resolve(strict=True)
    report = {
        "server": str(binary),
        "server_sha256": digest_file(binary),
        "script_sha256": digest_file(Path(__file__).resolve()),
        "tracks": args.tracks,
        "seed_tracks": args.seed_tracks,
        "threads": args.threads,
        "duration_seconds": args.duration,
        "python": sys.version.split()[0],
        "python_sqlite": sqlite3.sqlite_version,
        "host": {"platform": os.uname().sysname, "release": os.uname().release,
                 "machine": os.uname().machine},
        "scope": "Python-thread HTTP client; per-request connection; lower-bound RPS; "
                 "isolated temporary data; synthetic metadata has no audio files and "
                 "no audio is streamed",
        "endpoints": {},
        "diagnostic_requests": {},
    }
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    run(args, binary, report, opener)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
