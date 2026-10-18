#!/usr/bin/env python3
"""Isolated HTTP read-path load benchmark for Vela (no external load tools).

Starts a fresh server in a private temporary data directory, creates a throwaway
account through the real setup API, uploads synthetic WAVs, then drives concurrent
authenticated GETs for a fixed duration and reports latency percentiles/RPS.

This never opens the live database, never uses the live unit and never writes to
the user's library. Python's client is the load generator, so absolute RPS is a
client-bound lower bound, not a server capacity claim.
"""
from __future__ import annotations

import argparse
import http.client
import json
import math
import os
from pathlib import Path
import secrets
import signal
import socket
import statistics
import struct
import subprocess
import tempfile
import threading
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[1]


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


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/vela-server")
    parser.add_argument("--tracks", type=int, default=25, help="synthetic uploads (1..200)")
    parser.add_argument("--threads", type=int, default=16)
    parser.add_argument("--duration", type=float, default=10.0, help="seconds per endpoint")
    parser.add_argument("--output", type=Path, default=Path("/tmp/vela-http-bench.json"))
    args = parser.parse_args()
    if not 1 <= args.tracks <= 200 or not 1 <= args.threads <= 256 or not 1 <= args.duration <= 120:
        parser.error("tracks1..200, threads1..256, duration1..120")
    port = free_port()
    origin = f"http://127.0.0.1:{port}"
    report = {
        "server": str(args.binary),
        "origin": origin,
        "tracks": args.tracks,
        "threads": args.threads,
        "duration_seconds": args.duration,
        "host": {"platform": os.uname().sysname, "release": os.uname().release, "machine": os.uname().machine},
        "scope": "Python-thread HTTP client; lower-bound RPS; isolated temporary data; no audio streamed",
        "endpoints": {},
    }
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    with tempfile.TemporaryDirectory(prefix="vela-http-bench-") as folder:
        data = Path(folder) / "data"
        data.mkdir()
        environment = {
            **os.environ,
            "VELA_BIND": f"127.0.0.1:{port}",
            "VELA_DATA_DIR": str(data),
            "VELA_WEB_DIR": str(ROOT / "dist"),
            "VELA_SCAN_ROOTS": "[]",
        }
        with tempfile.TemporaryFile() as log:
            server = subprocess.Popen(
                [str(args.binary.resolve())],
                env=environment,
                stdout=log,
                stderr=subprocess.STDOUT,
            )
            try:
                deadline = time.monotonic() + 20
                while True:
                    try:
                        with opener.open(origin + "/health", timeout=1) as response:
                            if response.status == 200:
                                report["health"] = json.load(response)
                                break
                    except OSError:
                        pass
                    if server.poll() is not None or time.monotonic() > deadline:
                        raise RuntimeError("isolated server failed readiness")
                    time.sleep(0.05)

                password = secrets.token_urlsafe(24)
                setup = urllib.request.Request(
                    origin + "/api/setup",
                    data=json.dumps({"username": "benchmark", "password": password}).encode(),
                    headers={"Content-Type": "application/json", "Origin": origin},
                )
                with opener.open(setup, timeout=20) as response:
                    auth = json.load(response)
                    cookie = response.headers.get("Set-Cookie", "").split(";", 1)[0]
                if "vela_session=" not in cookie:
                    raise RuntimeError("setup did not return a session cookie")
                report["csrf_present"] = bool(auth.get("csrf_token"))

                for index in range(args.tracks):
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
                            "X-CSRF-Token": auth["csrf_token"],
                            "Origin": origin,
                            "Cookie": cookie,
                        },
                    )
                    with opener.open(upload, timeout=60) as response:
                        if response.status != 200:
                            raise RuntimeError(f"upload {index} failed")

                endpoints = [
                    ("health", "/health"),
                    ("me", "/api/me"),
                    ("tracks", "/api/tracks?limit=50"),
                    ("search", "/api/search?q=Bench"),
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
            finally:
                server.send_signal(signal.SIGTERM)
                try:
                    server.wait(timeout=20)
                except subprocess.TimeoutExpired:
                    server.kill()
                    server.wait()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
