#!/usr/bin/env python3
"""Run real native-process SIGTERM/restart checks against an isolated data directory."""
import argparse
import os
from pathlib import Path
import signal
import sqlite3
import socket
import subprocess
import tempfile
import time
import urllib.request


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=Path('target/release/vela-server'))
    parser.add_argument('--runs', type=int, default=10)
    parser.add_argument('--worker-failure', action='store_true', help='Also verify a real isolated worker database failure stops the process')
    args = parser.parse_args()
    if not 1 <= args.runs <= 100:
        parser.error('--runs must be1..100')
    exits = []
    with tempfile.TemporaryDirectory(prefix='vela-shutdown-') as directory:
        for index in range(args.runs + int(args.worker_failure)):
            with socket.socket() as listener:
                listener.bind(('127.0.0.1', 0))
                port = listener.getsockname()[1]
            environment = {**os.environ, 'VELA_BIND': f'127.0.0.1:{port}', 'VELA_DATA_DIR': directory, 'VELA_WEB_DIR': str(Path('dist').resolve()), 'NO_COLOR': '1'}
            process = subprocess.Popen([str(args.binary.resolve())], env=environment, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            try:
                deadline = time.monotonic() + 10
                while True:
                    try:
                        with urllib.request.urlopen(f'http://127.0.0.1:{port}/health', timeout=1) as response:
                            if response.status == 200:
                                break
                    except OSError:
                        if process.poll() is not None or time.monotonic() > deadline:
                            raise RuntimeError('Native server failed readiness')
                        time.sleep(.05)
                fatal = args.worker_failure and index == args.runs
                if fatal:
                    db=sqlite3.connect(Path(directory)/'vela.db')
                    try:
                        db.execute('DROP TABLE import_jobs')
                        db.commit()
                    finally:
                        db.close()
                else:
                    process.send_signal(signal.SIGTERM)
                output, _ = process.communicate(timeout=20)
                if fatal:
                    if process.returncode == 0 or b'Import worker stopped unexpectedly' not in output:
                        raise RuntimeError('Worker infrastructure failure did not fail the process')
                    print('Worker infrastructure failure correctly stopped the process')
                else:
                    exits.append(process.returncode)
                    if process.returncode:
                        print(output.decode(errors='replace'))
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait()
    print('SIGTERM exit codes:', exits)
    if any(exits):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
