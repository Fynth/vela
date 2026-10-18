#!/usr/bin/env python3
"""Opt-in actual Linux window/audio test with disposable Vela data and an owned null sink.

Build first: cargo build -p vela-desktop --example native-ui-qa --features native-qa
The user's default output, existing audio streams and persistent Vela data are untouched.
"""
import argparse
import http.cookiejar
import io
import json
import math
import os
from pathlib import Path
import secrets
import signal
import socket
import struct
import subprocess
import tempfile
import time
import urllib.request
import uuid
import wave

ROOT = Path(__file__).resolve().parent.parent
SINK = 'vela_native_test_null'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--server', type=Path, default=ROOT / 'target/runtime/vela-0.11.0/vela-server')
    parser.add_argument('--driver', type=Path, default=ROOT / 'target/debug/examples/native-ui-qa')
    parser.add_argument('--evidence', type=Path, default=ROOT / 'target/checks/native-window')
    parser.add_argument('--test-audio-engine', action='store_true', help='Also execute the opt-in actual output/seek/EOF test under the owned sink')
    parser.add_argument('--close-while-playing', action='store_true', help='Close the real window during active native playback and check original cleanup')
    args = parser.parse_args()
    evidence = args.evidence.resolve()
    if evidence.exists() and any(evidence.iterdir()):
        raise RuntimeError('Use a new/empty evidence directory; earlier evidence is preserved')
    evidence.mkdir(parents=True, exist_ok=True)
    report = evidence / 'report.json'
    sinks = subprocess.check_output(['pactl', 'list', 'short', 'sinks'], text=True)
    if any(line.split()[1] == SINK for line in sinks.splitlines()):
        raise RuntimeError('Fixture sink already exists; refusing to reuse or alter it')
    module = subprocess.check_output(['pactl', 'load-module', 'module-null-sink', f'sink_name={SINK}', 'sink_properties=device.description=Vela-isolated-QA'], text=True).strip()
    if not module.isdecimal():
        raise RuntimeError('Invalid owned Pulse module identity')
    server = driver = None
    try:
        with tempfile.TemporaryDirectory(prefix='vela-native-fixture-') as folder:
            private = Path(folder)
            (private / 'home').mkdir()
            (private / 'temporary').mkdir(mode=0o700)
            alsa = private / 'alsa.conf'
            alsa.write_text(f'pcm.!default {{ type pulse device "{SINK}" }}\nctl.!default {{ type pulse }}\n')
            with socket.socket() as listener:
                listener.bind(('127.0.0.1', 0))
                port = listener.getsockname()[1]
            origin = f'http://127.0.0.1:{port}'
            environment = {**os.environ, 'VELA_BIND': f'127.0.0.1:{port}', 'VELA_DATA_DIR': str(private / 'data'), 'VELA_WEB_DIR': str(ROOT / 'dist'), 'VELA_SCAN_ROOTS': '[]'}
            with (evidence / 'server.log').open('wb') as server_log, (evidence / 'window.log').open('wb') as window_log:
                server = subprocess.Popen([str(args.server.resolve())], env=environment, stdout=server_log, stderr=subprocess.STDOUT)
                deadline = time.monotonic() + 15
                health = urllib.request.build_opener(urllib.request.ProxyHandler({}))
                while True:
                    try:
                        with health.open(origin + '/health', timeout=1) as response:
                            if response.status == 200:
                                break
                    except OSError:
                        if server.poll() is not None or time.monotonic() > deadline:
                            raise RuntimeError('Isolated fixture server failed readiness')
                        time.sleep(.05)
                password = secrets.token_urlsafe(32)
                opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
                request = urllib.request.Request(origin + '/api/setup', data=json.dumps({'username': 'nativefixture', 'password': password}).encode(), headers={'Content-Type': 'application/json', 'Origin': origin})
                with opener.open(request, timeout=15) as response:
                    auth = json.load(response)
                buffer = io.BytesIO()
                with wave.open(buffer, 'wb') as output:
                    output.setparams((1, 2, 44100, 0, 'NONE', 'not compressed'))
                    output.writeframes(b''.join(struct.pack('<h', round(1500 * math.sin(2 * math.pi * 330 * index / 44100))) for index in range(44100 * 30)))
                fixtures = [('Native Continuity.wav', buffer.getvalue()), ('signal.flac', (ROOT / 'crates/media/tests/fixtures/signal.flac').read_bytes()), ('beacon.mp3', (ROOT / 'crates/media/tests/fixtures/beacon.mp3').read_bytes())]
                for name, data in fixtures:
                    boundary = 'VelaNative' + secrets.token_hex(16)
                    body = f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="{name}"\r\nContent-Type: application/octet-stream\r\n\r\n'.encode() + data + f'\r\n--{boundary}--\r\n'.encode()
                    request = urllib.request.Request(origin + '/api/upload', data=body, headers={'Content-Type': f'multipart/form-data; boundary={boundary}', 'X-CSRF-Token': auth['csrf_token'], 'Origin': origin})
                    with opener.open(request, timeout=20) as response:
                        assert response.status == 200
                with opener.open(urllib.request.Request(origin + '/api/tracks?limit=50', headers={'Origin': origin}), timeout=15) as response:
                    track_page = json.load(response)
                playlist_request = urllib.request.Request(origin + '/api/playlists', data=json.dumps({'name': 'Native evening'}).encode(), headers={'Content-Type': 'application/json', 'X-CSRF-Token': auth['csrf_token'], 'Origin': origin})
                with opener.open(playlist_request, timeout=15) as response:
                    playlist = json.load(response)
                revision = playlist['revision']
                for track in track_page['items'][:2]:
                    body = json.dumps({'revision': revision, 'mutation_id': str(uuid.uuid4()), 'track_id': track['id']}).encode()
                    request = urllib.request.Request(origin + f"/api/playlists/{playlist['id']}/items", data=body, headers={'Content-Type': 'application/json', 'X-CSRF-Token': auth['csrf_token'], 'Origin': origin})
                    with opener.open(request, timeout=15) as response:
                        revision = json.load(response)['playlist']['revision']
                # Per-process ALSA and Pulse routing, never a global default change.
                environment.update({'TMPDIR': str(private / 'temporary'), 'ALSA_CONFIG_PATH': str(alsa), 'PULSE_SINK': SINK, 'GDK_BACKEND': 'x11', 'VELA_NATIVE_TEST_SINK': SINK, 'VELA_DESKTOP_SERVER_URL': origin, 'VELA_NATIVE_QA_USERNAME': 'nativefixture', 'VELA_NATIVE_QA_PASSWORD': password, 'VELA_NATIVE_QA_REPORT': str(report), 'XDG_DATA_HOME': str(private / 'home'), 'XDG_CACHE_HOME': str(private / 'home/cache'), 'CARGO_BUILD_JOBS': '2'})
                if args.test_audio_engine:
                    with (evidence / 'audio-unit.log').open('wb') as audio_log:
                        subprocess.run(['cargo', 'test', '--locked', '-p', 'vela-desktop', '--lib', 'audio::tests::native_owned_null_sink_play_pause_seek_eof_replace_stop_and_cleanup', '--', '--ignored', '--exact', '--nocapture'], cwd=ROOT, env=environment, stdout=audio_log, stderr=subprocess.STDOUT, timeout=180, check=True)
                environment['VELA_NATIVE_QA_CLOSE_PLAYING'] = '1' if args.close_while_playing else '0'
                driver = subprocess.Popen([str(args.driver.resolve())], env=environment, stdout=window_log, stderr=subprocess.STDOUT)
                driver.wait(timeout=180)
                result = json.loads(report.read_text())
                if driver.returncode or not result['result']['ok'] or not all(image['passed'] for image in result['screenshots']):
                    raise RuntimeError(f'Native window verification failed; inspect {report}')
                originals = list((private / 'temporary').glob('vela-native-*'))
                result['temporary_originals_after_exit'] = len(originals)
                report.write_text(json.dumps(result, indent=2) + '\n')
                if originals:
                    raise RuntimeError('Native process left a completed temporary original after exit')
                print(json.dumps(result, indent=2))
    finally:
        for process in (driver, server):
            if process is not None and process.poll() is None:
                process.send_signal(signal.SIGTERM)
                try:
                    process.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
        subprocess.run(['pactl', 'unload-module', module], check=True)


if __name__ == '__main__':
    main()
