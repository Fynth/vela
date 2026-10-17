#!/usr/bin/env python3
"""Run real native-process SIGTERM/restart checks against an isolated data directory."""
import argparse
import os
from pathlib import Path
import signal
import shutil
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
    faults = [('Import', 'import_jobs'), ('Scanner', 'scan_jobs'),
              ('Audio analysis', 'audio_analysis'), ('Listening retention', 'listening_sessions')] if args.worker_failure else []
    with tempfile.TemporaryDirectory(prefix='vela-shutdown-') as directory:
        for index in range(args.runs + len(faults)):
            # Destructive fault injection must never contaminate the next fixture.
            instance_dir = Path(directory) if index < args.runs else Path(directory) / f'fault-{index}'
            fatal = index >= args.runs
            failed_worker, failed_table = faults[index - args.runs] if fatal else ('', '')
            # Retention wakes hourly after its immediate startup pass. Inject its
            # fault into a disposable copy before that pass, never into user data.
            before_start = failed_worker == 'Listening retention'
            if before_start:
                instance_dir.mkdir()
                shutil.copy2(Path(directory) / 'vela.db', instance_dir / 'vela.db')
                with sqlite3.connect(instance_dir / 'vela.db') as db:
                    db.execute('DROP TABLE listening_sessions')
            with socket.socket() as listener:
                listener.bind(('127.0.0.1', 0))
                port = listener.getsockname()[1]
            environment = {**os.environ, 'VELA_BIND': f'127.0.0.1:{port}', 'VELA_DATA_DIR': str(instance_dir), 'VELA_WEB_DIR': str(Path('dist').resolve()), 'VELA_SCAN_ROOTS': '[]', 'NO_COLOR': '1'}
            process = subprocess.Popen([str(args.binary.resolve())], env=environment, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            try:
                deadline = time.monotonic() + 10
                while not before_start:
                    try:
                        with urllib.request.urlopen(f'http://127.0.0.1:{port}/health', timeout=1) as response:
                            if response.status == 200:
                                break
                    except OSError:
                        if process.poll() is not None or time.monotonic() > deadline:
                            raise RuntimeError('Native server failed readiness')
                        time.sleep(.05)
                if fatal and not before_start:
                    db=sqlite3.connect(instance_dir/'vela.db')
                    try:
                        # Static names only, selected above; all DBs are fresh fixtures.
                        db.execute(f'DROP TABLE {failed_table}')
                        db.commit()
                    finally:
                        db.close()
                elif not fatal:
                    process.send_signal(signal.SIGTERM)
                output, _ = process.communicate(timeout=20)
                if fatal:
                    if process.returncode == 0 or f'{failed_worker} worker stopped unexpectedly'.encode() not in output:
                        raise RuntimeError('Worker infrastructure failure did not fail the process')
                    print(f'{failed_worker} worker infrastructure failure correctly stopped the process')
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
