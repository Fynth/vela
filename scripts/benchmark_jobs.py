#!/usr/bin/env python3
"""Measure real authenticated native /api/jobs requests on synthetic history.

Only fresh private TemporaryDirectory fixtures are accepted. The native server
initializes the schema/accounts; it is stopped before terminal-only metadata is
seeded, then restarted for serial loopback HTTP timings. No music or remote I/O
is involved. Existing Vela data and configuration are never opened or modified.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from datetime import datetime, timezone
import hashlib
import http.client
import json
import math
import os
from pathlib import Path
import platform
import secrets
import signal
import socket
import sqlite3
import subprocess
import tempfile
import time
from urllib.parse import urlencode
import uuid


REPOSITORY = Path(__file__).resolve().parents[1]
PAGE_SIZE = 50
STATUSES = ("completed", "failed", "cancelled")
PRIVATE_MARKERS = ("PRIVATE-BENCHMARK", "secret=", "example.invalid", "/private/")
HISTORY = """(
 SELECT id,owner_id,'import' AS kind,status,created_at,NULL AS source_id FROM import_jobs
 UNION ALL
 SELECT id,owner_id,'scan' AS kind,status,created_at,source_id FROM scan_jobs
)"""


def digest_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def source_fingerprint() -> dict:
    paths = [REPOSITORY / "Cargo.toml", REPOSITORY / "Cargo.lock"]
    for directory in ("apps", "crates"):
        root = REPOSITORY / directory
        paths.extend(root.rglob("*.rs"))
        paths.extend(root.rglob("Cargo.toml"))
    paths.extend((REPOSITORY / "migrations").glob("*.sql"))
    files = {str(path.relative_to(REPOSITORY)): digest_file(path)
             for path in sorted(set(paths))}
    encoded = json.dumps(files, sort_keys=True, separators=(",", ":")).encode()
    return {"sha256": hashlib.sha256(encoded).hexdigest(), "files": files,
            "scope": "All repository Rust source, workspace/crate manifests, Cargo.lock and SQL migrations."}


def hardware() -> dict:
    cpu = next((line.split(":", 1)[1].strip()
                for line in Path("/proc/cpuinfo").read_text().splitlines()
                if line.startswith("model name")), "unknown")
    memory = next((line.split(":", 1)[1].strip()
                   for line in Path("/proc/meminfo").read_text().splitlines()
                   if line.startswith("MemTotal:")), "unknown")
    return {"system": platform.platform(), "machine": platform.machine(),
            "cpu": cpu, "logical_cpus": os.cpu_count(), "memory_total": memory}


class Client:
    def __init__(self, port: int, cookie: str = ""):
        self.connection = http.client.HTTPConnection("127.0.0.1", port, timeout=15)
        self.cookie = cookie

    def request(self, method: str, path: str, body: dict | None = None,
                csrf: str = "") -> tuple[int, dict, bytes, float]:
        headers = {"Accept": "application/json"}
        if self.cookie:
            headers["Cookie"] = self.cookie
        if csrf:
            headers["X-CSRF-Token"] = csrf
        encoded = None
        if body is not None:
            encoded = json.dumps(body).encode()
            headers["Content-Type"] = "application/json"
        started = time.perf_counter_ns()
        self.connection.request(method, path, body=encoded, headers=headers)
        response = self.connection.getresponse()
        payload = response.read(1024 * 1024 + 1)
        elapsed = (time.perf_counter_ns() - started) / 1_000_000
        if len(payload) > 1024 * 1024:
            raise RuntimeError("Unexpected unbounded HTTP response")
        return response.status, {key.lower(): value for key, value in response.getheaders()}, payload, elapsed

    def json(self, method: str, path: str, body: dict | None = None,
             csrf: str = "", expected: int = 200) -> tuple[dict, dict]:
        status, headers, payload, _ = self.request(method, path, body, csrf)
        if status != expected:
            raise RuntimeError(f"Unexpected HTTP status {status} for {method} {path}")
        return json.loads(payload), headers

    def close(self):
        self.connection.close()


class Server:
    def __init__(self, binary: Path, fixture: Path):
        self.binary = binary
        self.fixture = fixture
        self.data = fixture / "data"
        (fixture / "web").mkdir()
        self.process: subprocess.Popen | None = None
        self.log = None

    def start(self) -> tuple[int, dict]:
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            port = listener.getsockname()[1]
        # Ignore inherited Vela settings, especially data roots and public bind.
        environment = {key: value for key, value in os.environ.items()
                       if not key.startswith("VELA_")}
        environment.update({"VELA_BIND": f"127.0.0.1:{port}",
                            "VELA_DATA_DIR": str(self.data),
                            "VELA_WEB_DIR": str(self.fixture / "web"),
                            "VELA_SCAN_ROOTS": "[]", "NO_COLOR": "1"})
        self.log = (self.fixture / "server.log").open("ab")
        self.process = subprocess.Popen([str(self.binary)], cwd=self.fixture,
                                        env=environment, stdout=self.log,
                                        stderr=subprocess.STDOUT)
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            if self.process.poll() is not None:
                raise RuntimeError("Isolated native server exited before readiness")
            client = Client(port)
            try:
                health, _ = client.json("GET", "/health")
                if health.get("status") == "ok":
                    return port, health
            except (OSError, http.client.HTTPException):
                time.sleep(.025)
            finally:
                client.close()
        raise RuntimeError("Isolated native server readiness timed out")

    def stop(self):
        if self.process is not None:
            if self.process.poll() is None:
                self.process.send_signal(signal.SIGTERM)
                try:
                    self.process.wait(timeout=30)
                except subprocess.TimeoutExpired:
                    self.process.kill()
                    self.process.wait()
                    raise RuntimeError("Isolated server did not shut down gracefully")
            code = self.process.returncode
            self.process = None
            if self.log is not None:
                self.log.close()
                self.log = None
            if code != 0:
                raise RuntimeError(f"Isolated native server exited with code {code}")


@dataclass
class Actor:
    identity: str
    cookie: str
    csrf: str


def bootstrap(port: int) -> tuple[Actor, Actor]:
    client = Client(port)
    password = secrets.token_urlsafe(32)
    try:
        auth, headers = client.json("POST", "/api/setup", {
            "username": "jobs-benchmark", "password": password})
        owner = Actor(auth["user"]["id"], headers["set-cookie"].split(";", 1)[0], auth["csrf_token"])
        client.cookie = owner.cookie
        other, _ = client.json("POST", "/api/users", {
            "username": "jobs-other", "password": password}, owner.csrf)
        auth, headers = client.json("POST", "/api/login", {
            "username": "jobs-other", "password": password})
        if auth["user"]["id"] != other["id"]:
            raise RuntimeError("Fixture login returned the wrong account")
        foreign = Actor(other["id"], headers["set-cookie"].split(";", 1)[0], auth["csrf_token"])
        return owner, foreign
    finally:
        client.close()


def sqlite_writer_version(database: Path) -> str:
    with database.open("rb") as stream:
        header = stream.read(100)
    if not header.startswith(b"SQLite format 3\x00"):
        raise RuntimeError("Missing native-created SQLite header")
    number = int.from_bytes(header[96:100], "big")
    return f"{number // 1_000_000}.{number // 1000 % 1000}.{number % 1000}"


def seed(database: Path, count: int, owner: Actor, foreign: Actor) -> tuple[dict, list[dict]]:
    connection = sqlite3.connect(database)
    connection.execute("PRAGMA foreign_keys=ON")
    schema = connection.execute("SELECT max(version) FROM _sqlx_migrations WHERE success=1").fetchone()[0]
    expected_schema = max(int(path.name.split("_", 1)[0]) for path in (REPOSITORY / "migrations").glob("*.sql"))
    if schema != expected_schema:
        raise RuntimeError(f"Expected native-embedded schema{expected_schema}, found {schema}")
    sources = [str(uuid.UUID(int=(9 << 120) | index)) for index in (1, 2)]
    started = time.perf_counter()
    with connection:
        for owner_index, (actor, source) in enumerate(zip((owner, foreign), sources)):
            connection.execute("INSERT INTO scan_sources(id,owner_id,name,root_id) VALUES(?,?,?,?)",
                               (source, actor.identity, "Synthetic owned history" if owner_index == 0 else "PRIVATE-BENCHMARK-SOURCE", f"fixture-{owner_index}"))
            for start in range(0, count, 1000):
                imports, scans = [], []
                for index in range(start, min(start + 1000, count)):
                    pair = index // 2
                    identity = str(uuid.UUID(int=((owner_index + 1) << 96) | pair))
                    timestamp = 1_700_000_000 + pair // 8
                    status = STATUSES[pair % len(STATUSES)]
                    if index % 2 == 0:
                        imports.append((identity, actor.identity, status, timestamp, timestamp + 5))
                    else:
                        scans.append((identity, actor.identity, source, status, int(status == "cancelled"), timestamp, timestamp + 5, timestamp + 5))
                connection.executemany("INSERT INTO import_jobs(id,owner_id,url,status,downloaded_bytes,total_bytes,error,created_at,updated_at) VALUES(?,?,'https://example.invalid/a.flac?secret=PRIVATE-BENCHMARK',?,1000,1000,'/private/PRIVATE-BENCHMARK-DIAGNOSTIC',?,?)", imports)
                connection.executemany("INSERT INTO scan_jobs(id,owner_id,source_id,status,cancel_requested,visited,imported,duplicates,unchanged,skipped,failed_files,error,created_at,updated_at,finished_at) VALUES(?,?,?,?,?,30,2,3,4,5,1,'/private/PRIVATE-BENCHMARK-DIAGNOSTIC',?,?,?)", scans)
    elapsed = time.perf_counter() - started
    if connection.execute("PRAGMA quick_check").fetchone()[0] != "ok":
        raise RuntimeError("Seeded fixture failed SQLite quick_check")
    if connection.execute("PRAGMA foreign_key_check").fetchone() is not None:
        raise RuntimeError("Seeded fixture has invalid foreign keys")
    rows = dict(connection.execute(f"SELECT owner_id,count(*) FROM {HISTORY} GROUP BY owner_id"))
    if rows != {owner.identity: count, foreign.identity: count}:
        raise RuntimeError("Synthetic history count mismatch")

    def cursor(offset: int) -> str:
        row = connection.execute(f"SELECT created_at,id,kind FROM {HISTORY} WHERE owner_id=? ORDER BY created_at DESC,id DESC,kind DESC LIMIT 1 OFFSET ?", (owner.identity, offset)).fetchone()
        return f"v1:{row[0]}:{row[1]}:{row[2]}"

    cases = [
        ("first_page", {}),
        ("middle_cursor", {"cursor": cursor(count // 2 - 1)}),
        ("deep_99_percent_cursor", {"cursor": cursor(count * 99 // 100 - 1)}),
        ("completed_filter", {"state": "completed"}),
        ("failed_filter", {"state": "failed"}),
        ("cancelled_filter", {"state": "cancelled"}),
        ("imports_filter", {"kind": "import"}),
        ("scans_filter", {"kind": "scan"}),
        ("deep_95_percent_failed_filter", {"cursor": cursor(count * 95 // 100 - 1), "state": "failed"}),
    ]
    prepared = []
    for label, parameters in cases:
        sql = f"SELECT id,kind,created_at,status,source_id FROM {HISTORY} WHERE owner_id=?"
        bound: list[str | int] = [owner.identity]
        if "cursor" in parameters:
            _, timestamp, identity, kind = parameters["cursor"].split(":")
            sql += " AND (created_at,id,kind)<(?,?,?)"
            bound.extend((int(timestamp), identity, kind))
        for key, column in (("kind", "kind"), ("state", "status")):
            if key in parameters:
                sql += f" AND {column}=?"
                bound.append(parameters[key])
        sql += " ORDER BY created_at DESC,id DESC,kind DESC LIMIT ?"
        bound.append(PAGE_SIZE + 1)
        expected = connection.execute(sql, bound).fetchall()
        if len(expected) < PAGE_SIZE:
            raise RuntimeError("Benchmark case must return exactly50 owned tasks")
        prepared.append({"name": label, "query": parameters, "expected": expected[:PAGE_SIZE],
                         "has_more": len(expected) > PAGE_SIZE})
    connection.execute("PRAGMA wal_checkpoint(TRUNCATE)")
    connection.close()
    return {"owned_history_rows": count, "foreign_history_rows": count,
            "owned_import_rows": count // 2, "owned_scan_rows": count // 2,
            "foreign_import_rows": count // 2, "foreign_scan_rows": count // 2,
            "terminal_statuses": list(STATUSES), "active_rows": 0,
            "schema_version": schema, "seed_seconds": elapsed,
            "database_bytes": database.stat().st_size, "owned_source_id": sources[0]}, prepared


def validate_page(payload: bytes, headers: dict, case: dict, source: str) -> dict:
    if headers.get("cache-control") != "no-store":
        raise RuntimeError("Task HTTP response lost no-store policy")
    text = payload.decode()
    if any(marker in text for marker in PRIVATE_MARKERS):
        raise RuntimeError("Task read model disclosed private seed metadata")
    page = json.loads(payload)
    if len(page["items"]) != PAGE_SIZE or page["has_active"]:
        raise RuntimeError("Task page size/activity did not match terminal-only fixture")
    observed = [(row["id"], row["kind"], row["created_at"], row["status"], row["source_id"])
                for row in page["items"]]
    if observed != case["expected"]:
        raise RuntimeError("Native HTTP result differs from exact owned SQL reference")
    expected_cursor = None
    if case["has_more"]:
        last = page["items"][-1]
        expected_cursor = f"v1:{last['created_at']}:{last['id']}:{last['kind']}"
    if page["next_cursor"] != expected_cursor:
        raise RuntimeError("Native HTTP cursor differs from expected bounded page")
    for row in page["items"]:
        if any(key in row for key in ("url", "owner_id", "path", "token")):
            raise RuntimeError("Unexpected private task field")
        if row["track_id"] is not None or row["can_cancel"]:
            raise RuntimeError("Invalid terminal fixture action/media access")
        expected_retry = row["kind"] == "import" and row["status"] in ("failed", "cancelled")
        if row["can_retry"] != expected_retry or row["can_rescan"] != (row["kind"] == "scan"):
            raise RuntimeError("Action flags did not match terminal fixture")
        if row["kind"] == "scan" and row["source_id"] != source:
            raise RuntimeError("Foreign source escaped owner boundary")
    return page


def summarize(samples: list[float]) -> dict:
    ordered = sorted(samples)
    def percentile(value: float) -> float:
        return ordered[max(0, math.ceil(len(ordered) * value) - 1)]
    return {"p50_ms": percentile(.50), "p95_ms": percentile(.95),
            "p99_ms": percentile(.99), "min_ms": ordered[0], "max_ms": ordered[-1],
            "mean_ms": sum(samples) / len(samples)}


def run_workload(binary: Path, count: int, samples: int, warmups: int) -> dict:
    with tempfile.TemporaryDirectory(prefix="vela-jobs-benchmark-") as temporary:
        fixture = Path(temporary)
        server = Server(binary, fixture)
        try:
            port, health = server.start()
            if health.get("version") != "0.4.0":
                raise RuntimeError("Benchmark requires native Vela0.4.0")
            owner, foreign = bootstrap(port)
            server.stop()
            native_sqlite = sqlite_writer_version(server.data / "vela.db")
            metadata, cases = seed(server.data / "vela.db", count, owner, foreign)
            port, restarted = server.start()
            if restarted != health:
                raise RuntimeError("Native version/readiness changed on fixture restart")
            client = Client(port, owner.cookie)
            other = Client(port, foreign.cookie)
            anonymous = Client(port)
            try:
                status, _, _, _ = anonymous.request("GET", "/api/jobs")
                if status != 401:
                    raise RuntimeError("Unauthenticated task history was not rejected")
                status, _, _, _ = other.request("GET", "/api/jobs?kind=scan")
                if status != 403:
                    raise RuntimeError("Nonadministrator scan query was not rejected")
                foreign_page, _ = other.json("GET", "/api/jobs?limit=50")
                if len(foreign_page["items"]) != PAGE_SIZE or foreign_page["has_active"]:
                    raise RuntimeError("Foreign authenticated fixture page mismatch")
                if any(row["kind"] != "import" or (uuid.UUID(row["id"]).int >> 96) != 2
                       for row in foreign_page["items"]):
                    raise RuntimeError("Foreign actor saw another owner or hidden scans")
                measurements = []
                for case in cases:
                    path = "/api/jobs?" + urlencode({"limit": PAGE_SIZE, **case["query"]})
                    timings, sizes = [], []
                    for iteration in range(warmups + samples):
                        status, headers, payload, elapsed = client.request("GET", path)
                        if status != 200:
                            raise RuntimeError(f"Task benchmark request failed with HTTP{status}")
                        validate_page(payload, headers, case, metadata["owned_source_id"])
                        if iteration >= warmups:
                            timings.append(elapsed)
                            sizes.append(len(payload))
                    measurements.append({"name": case["name"], "query": case["query"],
                                         "rows_per_response": PAGE_SIZE,
                                         "response_bytes_min": min(sizes), "response_bytes_max": max(sizes),
                                         **summarize(timings), "samples_ms": timings})
                    print(f"{count:,} owned +{count:,} foreign | {case['name']}: p50={measurements[-1]['p50_ms']:.3f}ms p95={measurements[-1]['p95_ms']:.3f}ms", flush=True)
            finally:
                client.close()
                other.close()
                anonymous.close()
            server.stop()
            metadata.pop("owned_source_id")
            return {**metadata, "native_version": health["version"],
                    "native_sqlite_header_last_writer_version_before_python_seed": native_sqlite,
                    "verified": ["HTTP401 without session", "HTTP403 nonadmin scan query",
                                 "Foreign user sees only own imports", "Every timed response has exactly50 reference-matching owned rows",
                                 "URLs/raw errors/foreign sources omitted", "No active jobs", "Exact kind-aware next cursor and action flags"],
                    "cases": measurements}
        finally:
            server.stop()


def write_markdown(path: Path, report: dict):
    lines = ["# Synthetic job-history HTTP measurements", "",
             f"Executed {report['created_at_utc']} using native Vela0.4.0 on loopback. "
             "These are serial, warm authenticated task-history requests against synthetic metadata. "
             "No audio, catalog, streaming, concurrent-client capacity, browser rendering, federation or product-superiority claim is made.", "",
             f"CPU: {report['hardware']['cpu']}; {report['hardware']['logical_cpus']} logical CPUs; memory {report['hardware']['memory_total']}. "
             f"OS: {report['hardware']['system']}. Python {report['python_version']}; Python SQLite {report['python_sqlite_version']}.", "",
             f"Native SHA256: `{report['native_binary']['sha256']}`. Runtime source fingerprint: `{report['source']['sha256']}`. "
             f"Script SHA256: `{report['script_sha256']}`. Full source hashes and raw samples are in the adjacent JSON.", "",
             "The server creates/migrates a fresh private temporary database and real accounts through setup/admin APIs. "
             "It stops cleanly before deterministic synthetic import/scan histories are inserted, then restarts with saved fixture sessions. "
             "Each owner has equal import/scan counts; completed/failed/cancelled states cycle evenly. "
             "Pairs deliberately share UUIDs and timestamps across tables. An equally large foreign history tests isolation. "
             "All jobs are terminal: workers perform no downloads or scans. No existing database or remote resource is accessed.", "",
             "Timing uses one reused HTTP/1.1 connection and perf_counter_ns from request send through complete response-body read. "
             "Normal per-request session authentication is included; client JSON parsing/validation, fixture seeding, initial account setup/password login and startup are excluded. "
             f"Each case has {report['warmup_requests_per_case']} discarded warmups and {report['measured_requests_per_case']} retained requests. "
             "Percentiles use empirical nearest rank. Every response is checked against an exact owned SQL reference prepared before startup; "
             "reference OFFSET only selects benchmark anchors offline, while timed HTTP uses real keyset cursors. "
             "Warm OS/database caches and the default four-connection SQLx pool apply; no CPU pinning or cold-cache reset is used.", "",
             f"Shared-host workload during run: {report['shared_host_workload'].rstrip('.')}. "
             "Results describe this run; tail values at this sample size do not establish a service-level guarantee.", "",
             "| Owned + foreign history | Case | p50 ms | p95 ms | p99 ms | Rows |", "| --- | --- | ---: | ---: | ---: | ---: |"]
    for workload in report["workloads"]:
        for case in workload["cases"]:
            lines.append(f"| {workload['owned_history_rows']:,} + {workload['foreign_history_rows']:,} | {case['name']} | {case['p50_ms']:.3f} | {case['p95_ms']:.3f} | {case['p99_ms']:.3f} | {case['rows_per_response']} |")
    lines.extend(["", "Native SQLite last-writer versions read from the native-created database header before any Python seed: " +
                  ", ".join(sorted({workload['native_sqlite_header_last_writer_version_before_python_seed'] for workload in report['workloads']})) + ". "
                  "This identifies the native SQLite writer independently of the Python SQLite seeder.", "",
                  "Reproduce from this source checkpoint with the matching already-built release binary:", "", "```bash",
                  "python3 scripts/benchmark_jobs.py --binary target/release/vela-server --counts 10000 100000 --samples 100 --warmups 5 \\",
                  "  --json docs/benchmarks/jobs-0.4.0.json --markdown docs/benchmarks/jobs-0.4.0.md \\",
                  "  --shared-host-workload 'Describe concurrent host activity for the new run'", "```", ""])
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=REPOSITORY / "target/release/vela-server")
    parser.add_argument("--counts", type=int, nargs="+", default=[10000, 100000])
    parser.add_argument("--samples", type=int, default=100)
    parser.add_argument("--warmups", type=int, default=5)
    parser.add_argument("--json", type=Path, required=True)
    parser.add_argument("--markdown", type=Path, required=True)
    parser.add_argument("--shared-host-workload", default="Shared development host; external activity uncontrolled and unmeasured.")
    args = parser.parse_args()
    if any(count < 10000 or count > 1_000_000 or count % 2 for count in args.counts):
        parser.error("counts must be even integers between10000 and1000000")
    if not 20 <= args.samples <= 10000 or not 1 <= args.warmups <= 100:
        parser.error("samples must be20..10000 and warmups1..100")
    binary = args.binary.resolve(strict=True)
    source = source_fingerprint()
    native_hash = digest_file(binary)
    report = {"created_at_utc": datetime.now(timezone.utc).isoformat(),
              "scope": "Synthetic terminal job metadata; actual authenticated serial warm native loopback HTTP; no remote I/O/media/concurrency/capacity claims.",
              "hardware": hardware(), "python_version": platform.python_version(),
              "python_sqlite_version": sqlite3.sqlite_version,
              "native_binary": {"path": str(binary.relative_to(REPOSITORY)) if binary.is_relative_to(REPOSITORY) else binary.name,
                                "sha256": native_hash},
              "source": source, "script_sha256": digest_file(Path(__file__)),
              "warmup_requests_per_case": args.warmups,
              "measured_requests_per_case": args.samples,
              "page_size": PAGE_SIZE, "percentile_method": "Empirical nearest rank",
              "shared_host_workload": args.shared_host_workload,
              "workloads": []}
    for count in args.counts:
        report["workloads"].append(run_workload(binary, count, args.samples, args.warmups))
    if source_fingerprint() != source or digest_file(binary) != native_hash:
        raise RuntimeError("Runtime sources or native binary changed during measurement; rerun after build settles")
    args.json.parent.mkdir(parents=True, exist_ok=True)
    args.json.write_text(json.dumps(report, indent=2) + "\n")
    write_markdown(args.markdown, report)


if __name__ == "__main__":
    main()
