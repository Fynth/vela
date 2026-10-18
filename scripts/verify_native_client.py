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
import select
import signal
import socket
import ssl
import struct
import threading
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
    parser.add_argument('--server', type=Path, default=ROOT / 'target/debug/vela-server', help='Current source server; build with cargo build -p vela-server')
    parser.add_argument('--driver', type=Path, default=ROOT / 'target/debug/examples/native-ui-qa')
    parser.add_argument('--evidence', type=Path, default=ROOT / 'target/checks/native-window')
    parser.add_argument('--test-audio-engine', action='store_true', help='Also execute the opt-in actual output/seek/EOF test under the owned sink')
    parser.add_argument('--close-while-playing', action='store_true', help='Close the real window during active native playback and check original cleanup')
    parser.add_argument('--test-peer', action='store_true', help='Start a second isolated Vela server and exercise its explicit public catalog')
    parser.add_argument('--test-session', action='store_true', help='Also verify opt-in Secret Service session save/resume/delete across two native processes')
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
    server = driver = peer_process = peer2_process = None
    session_service = None
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
                tls_connections = []
                if args.test_peer:
                    with socket.socket() as peer_listener:
                        peer_listener.bind(('127.0.0.1', 0))
                        peer_port = peer_listener.getsockname()[1]
                    peer_origin = 'https://peer.invalid'
                    peer_transport = f'http://127.0.0.1:{peer_port}'
                    peer_data = private / 'peer-data'
                    peer_environment = {**os.environ, 'VELA_BIND': f'127.0.0.1:{peer_port}', 'VELA_DATA_DIR': str(peer_data), 'VELA_WEB_DIR': str(ROOT / 'dist'), 'VELA_SCAN_ROOTS': '[]'}
                    with (evidence / 'peer-server.log').open('wb') as peer_log:
                        peer_process = subprocess.Popen([str(args.server.resolve())], env=peer_environment, stdout=peer_log, stderr=subprocess.STDOUT)
                    peer_deadline = time.monotonic() + 15
                    peer_health = urllib.request.build_opener(urllib.request.ProxyHandler({}))
                    while True:
                        try:
                            with peer_health.open(peer_transport + '/health', timeout=1) as response:
                                if response.status == 200:
                                    break
                        except OSError:
                            if peer_process.poll() is not None or time.monotonic() > peer_deadline:
                                raise RuntimeError('Isolated peer server failed readiness')
                            time.sleep(.05)
                    peer_password = secrets.token_urlsafe(32)
                    peer_opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
                    setup_request = urllib.request.Request(peer_transport + '/api/setup', data=json.dumps({'username': 'peerowner', 'password': peer_password}).encode(), headers={'Content-Type': 'application/json', 'Origin': peer_transport})
                    with peer_opener.open(setup_request, timeout=15) as response:
                        peer_auth = json.load(response)
                    peer_audio = (ROOT / 'crates/media/tests/fixtures/signal.flac').read_bytes()
                    boundary = 'VelaPeer' + secrets.token_hex(16)
                    peer_body = f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="signal.flac"\r\nContent-Type: audio/flac\r\n\r\n'.encode() + peer_audio + f'\r\n--{boundary}--\r\n'.encode()
                    upload_request = urllib.request.Request(peer_transport + '/api/upload', data=peer_body, headers={'Content-Type': f'multipart/form-data; boundary={boundary}', 'X-CSRF-Token': peer_auth['csrf_token'], 'Origin': peer_transport})
                    with peer_opener.open(upload_request, timeout=20) as response:
                        peer_track = json.load(response)['track']
                    settings_request = urllib.request.Request(peer_transport + '/api/network/settings', data=json.dumps({'revision': 0, 'enabled': True, 'origin': peer_origin, 'name': 'Peer QA'}).encode(), headers={'Content-Type': 'application/json', 'X-CSRF-Token': peer_auth['csrf_token'], 'Origin': peer_transport}, method='PUT')
                    with peer_opener.open(settings_request, timeout=15) as response:
                        json.load(response)
                    publication_request = urllib.request.Request(peer_transport + '/api/publications', data=json.dumps({'track_id': peer_track['id'], 'license': 'CC-BY-4.0', 'attribution': 'Peer QA signal fixture', 'rights_evidence': 'https://peer.invalid/rights', 'allow_stream': True, 'allow_download': False, 'rights_confirmed': True}).encode(), headers={'Content-Type': 'application/json', 'X-CSRF-Token': peer_auth['csrf_token'], 'Origin': peer_transport})
                    with peer_opener.open(publication_request, timeout=15) as response:
                        json.load(response)
                    for index in range(20):
                        samples = 44_100 * 2
                        tone = io.BytesIO()
                        with wave.open(tone, 'wb') as output:
                            output.setnchannels(1)
                            output.setsampwidth(2)
                            output.setframerate(44_100)
                            frequency = 300 + index * 15
                            output.writeframes(b''.join(
                                struct.pack('<h', round(1800 * math.sin(2 * math.pi * frequency * sample / 44_100)))
                                for sample in range(samples)
                            ))
                        data = tone.getvalue()
                        boundary_n = 'VelaPeerN' + secrets.token_hex(16)
                        body_n = f'--{boundary_n}\r\nContent-Disposition: form-data; name="file"; filename="Vela Window {index + 1:02}.wav"\r\nContent-Type: audio/wav\r\n\r\n'.encode() + data + f'\r\n--{boundary_n}--\r\n'.encode()
                        request_n = urllib.request.Request(peer_transport + '/api/upload', data=body_n, headers={'Content-Type': f'multipart/form-data; boundary={boundary_n}', 'X-CSRF-Token': peer_auth['csrf_token'], 'Origin': peer_transport})
                        with peer_opener.open(request_n, timeout=20) as response:
                            track_n = json.load(response)['track']
                        publication_n = urllib.request.Request(peer_transport + '/api/publications', data=json.dumps({'track_id': track_n['id'], 'license': 'CC0-1.0', 'attribution': f'Peer QA window {index + 1}', 'rights_evidence': f'https://peer.invalid/rights/{index + 1}', 'allow_stream': True, 'allow_download': False, 'rights_confirmed': True}).encode(), headers={'Content-Type': 'application/json', 'X-CSRF-Token': peer_auth['csrf_token'], 'Origin': peer_transport})
                        with peer_opener.open(publication_n, timeout=15) as response:
                            json.load(response)
                    with urllib.request.urlopen(peer_transport + '/api/public/catalog?limit=20', timeout=5) as response:
                        peer_catalog = json.load(response)
                    if not peer_catalog.get('items') or not peer_catalog['items'][0].get('can_stream') or not peer_catalog.get('next_cursor'):
                        raise RuntimeError('Isolated peer public catalog did not expose paginated streaming fixtures')
                    with urllib.request.urlopen(peer_transport + '/api/public/catalog?q=Vela&limit=20', timeout=5) as response:
                        if not json.load(response).get('items'):
                            raise RuntimeError('Isolated peer public search did not match its fixture')
                    environment.update({'VELA_NATIVE_PEER_CANONICAL': peer_origin, 'VELA_NATIVE_PEER_TRANSPORT': peer_transport, 'VELA_NATIVE_QA_PEER_ORIGIN': peer_origin})
                    with socket.socket() as peer2_listener:
                        peer2_listener.bind(('127.0.0.1', 0))
                        peer2_port = peer2_listener.getsockname()[1]
                    peer2_origin = 'https://peer2.invalid'
                    peer2_transport = f'http://127.0.0.1:{peer2_port}'
                    peer2_data = private / 'peer2-data'
                    peer2_environment = {**os.environ, 'VELA_BIND': f'127.0.0.1:{peer2_port}', 'VELA_DATA_DIR': str(peer2_data), 'VELA_WEB_DIR': str(ROOT / 'dist'), 'VELA_SCAN_ROOTS': '[]'}
                    with (evidence / 'peer2-server.log').open('wb') as peer2_log:
                        peer2_process = subprocess.Popen([str(args.server.resolve())], env=peer2_environment, stdout=peer2_log, stderr=subprocess.STDOUT)
                    peer2_deadline = time.monotonic() + 15
                    while True:
                        try:
                            with peer_health.open(peer2_transport + '/health', timeout=1) as response:
                                if response.status == 200:
                                    break
                        except OSError:
                            if peer2_process.poll() is not None or time.monotonic() > peer2_deadline:
                                raise RuntimeError('Isolated second peer server failed readiness')
                            time.sleep(.05)
                    peer2_opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
                    setup2 = urllib.request.Request(peer2_transport + '/api/setup', data=json.dumps({'username': 'peerowner2', 'password': secrets.token_urlsafe(32)}).encode(), headers={'Content-Type': 'application/json', 'Origin': peer2_transport})
                    with peer2_opener.open(setup2, timeout=15) as response:
                        peer2_auth = json.load(response)
                    peer2_audio = (ROOT / 'crates/media/tests/fixtures/beacon.mp3').read_bytes()
                    boundary2 = 'VelaPeer2' + secrets.token_hex(16)
                    peer2_body = f'--{boundary2}\r\nContent-Disposition: form-data; name="file"; filename="beacon.mp3"\r\nContent-Type: audio/mpeg\r\n\r\n'.encode() + peer2_audio + f'\r\n--{boundary2}--\r\n'.encode()
                    upload2 = urllib.request.Request(peer2_transport + '/api/upload', data=peer2_body, headers={'Content-Type': f'multipart/form-data; boundary={boundary2}', 'X-CSRF-Token': peer2_auth['csrf_token'], 'Origin': peer2_transport})
                    with peer2_opener.open(upload2, timeout=20) as response:
                        peer2_track = json.load(response)['track']
                    settings2 = urllib.request.Request(peer2_transport + '/api/network/settings', data=json.dumps({'revision': 0, 'enabled': True, 'origin': peer2_origin, 'name': 'Peer QA Two'}).encode(), headers={'Content-Type': 'application/json', 'X-CSRF-Token': peer2_auth['csrf_token'], 'Origin': peer2_transport}, method='PUT')
                    with peer2_opener.open(settings2, timeout=15) as response:
                        json.load(response)
                    publication2 = urllib.request.Request(peer2_transport + '/api/publications', data=json.dumps({'track_id': peer2_track['id'], 'license': 'CC0-1.0', 'attribution': 'Peer QA two beacon fixture', 'rights_evidence': 'https://peer2.invalid/rights', 'allow_stream': True, 'allow_download': False, 'rights_confirmed': True}).encode(), headers={'Content-Type': 'application/json', 'X-CSRF-Token': peer2_auth['csrf_token'], 'Origin': peer2_transport})
                    with peer2_opener.open(publication2, timeout=15) as response:
                        json.load(response)
                    with urllib.request.urlopen(peer2_transport + '/api/public/catalog?limit=20', timeout=5) as response:
                        peer2_catalog = json.load(response)
                    if not peer2_catalog.get('items') or not peer2_catalog['items'][0].get('can_stream'):
                        raise RuntimeError('Second peer public catalog did not expose the streaming fixture')
                    with urllib.request.urlopen(peer2_transport + '/api/public/catalog?q=Vela&limit=20', timeout=5) as response:
                        if not json.load(response).get('items'):
                            raise RuntimeError('Second peer public search did not match its fixture')
                    config_dir = private / 'home' / 'vela'
                    config_dir.mkdir(parents=True, mode=0o700)
                    cached_origin = 'https://offline.example.org'
                    config_path = config_dir / 'peers.json'
                    config_path.write_text(json.dumps({'version': 1, 'peers': [peer2_origin + '/', cached_origin + '/']}))
                    config_path.chmod(0o600)
                    def fnv1a(value):
                        hashed = 0xcbf29ce484222325
                        for byte in value.encode():
                            hashed ^= byte
                            hashed = (hashed * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
                        return f'{hashed:016x}'
                    cache_dir = private / 'home' / 'cache' / 'vela' / 'public-catalog'
                    cache_dir.mkdir(parents=True, mode=0o700)
                    cache_file = cache_dir / f'{fnv1a(cached_origin + "/")}.json'
                    cache_file.write_text(json.dumps({
                        'version': 1,
                        'origin': cached_origin + '/',
                        'fetched_at': int(time.time()) - 3700,
                        'items': [{
                            'id': str(uuid.uuid4()), 'revision': 1, 'title': 'Cached Beacon',
                            'artist': 'Vela Test', 'album': 'Cached Album', 'genre': '',
                            'year': None, 'duration_ms': 30000, 'codec': 'flac',
                            'license': 'CC0-1.0', 'attribution': 'Cached fixture',
                            'can_stream': True, 'can_download': False,
                        }],
                    }))
                    cache_file.chmod(0o600)
                    tls_dir = private / 'tls'
                    tls_dir.mkdir(mode=0o700)
                    ca_key = tls_dir / 'ca.key'
                    ca_pem = tls_dir / 'ca.pem'
                    server_key = tls_dir / 'server.key'
                    server_csr = tls_dir / 'server.csr'
                    server_pem = tls_dir / 'server.pem'
                    san = tls_dir / 'san.ext'
                    san.write_text('subjectAltName=IP:127.0.0.1\n')
                    subprocess.run(['openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-keyout', str(ca_key), '-out', str(ca_pem), '-days', '1', '-subj', '/CN=Vela QA CA'], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    subprocess.run(['openssl', 'req', '-newkey', 'rsa:2048', '-nodes', '-keyout', str(server_key), '-out', str(server_csr), '-subj', '/CN=127.0.0.1'], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    subprocess.run(['openssl', 'x509', '-req', '-in', str(server_csr), '-CA', str(ca_pem), '-CAkey', str(ca_key), '-CAcreateserial', '-out', str(server_pem), '-days', '1', '-extfile', str(san)], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    for key_file in (ca_key, server_key):
                        key_file.chmod(0o600)
                    tls_context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
                    tls_context.load_cert_chain(certfile=server_pem, keyfile=server_key)
                    with socket.socket() as tls_listener_probe:
                        tls_listener_probe.bind(('127.0.0.1', 0))
                        tls_port = tls_listener_probe.getsockname()[1]
                    def relay_tls(client_socket):
                        try:
                            with tls_context.wrap_socket(client_socket, server_side=True) as tls:
                                with socket.create_connection(('127.0.0.1', peer2_port), timeout=10) as upstream:
                                    sockets = [tls, upstream]
                                    while True:
                                        readable, _, _ = select.select(sockets, [], [], 20)
                                        if not readable:
                                            break
                                        for source in readable:
                                            data = source.recv(65536)
                                            if not data:
                                                return
                                            target = upstream if source is tls else tls
                                            target.sendall(data)
                        except (OSError, ssl.SSLError):
                            return
                    def tls_accept_loop(listener):
                        while True:
                            try:
                                client_socket, _ = listener.accept()
                            except OSError:
                                return
                            tls_connections.append(client_socket.getpeername())
                            threading.Thread(target=relay_tls, args=(client_socket,), daemon=True).start()
                    tls_listener = socket.socket()
                    tls_listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
                    tls_listener.bind(('127.0.0.1', tls_port))
                    tls_listener.listen(16)
                    threading.Thread(target=tls_accept_loop, args=(tls_listener,), daemon=True).start()
                    environment.update({
                        'VELA_NATIVE_PEER_CA': str(ca_pem),
                        'VELA_NATIVE_PEER_TRANSPORTS': json.dumps({peer_origin + '/': peer_transport, peer2_origin + '/': f'https://127.0.0.1:{tls_port}'}),
                        'VELA_NATIVE_QA_PEER_ORIGIN': peer_origin,
                        'VELA_NATIVE_QA_PEER2_ORIGIN': peer2_origin,
                        'VELA_NATIVE_QA_CACHED_ORIGIN': cached_origin,
                    })
                # Per-process ALSA and Pulse routing, never a global default change.
                environment.update({'TMPDIR': str(private / 'temporary'), 'ALSA_CONFIG_PATH': str(alsa), 'PULSE_SINK': SINK, 'GDK_BACKEND': 'x11', 'VELA_NATIVE_TEST_SINK': SINK, 'VELA_DESKTOP_SERVER_URL': origin, 'VELA_NATIVE_QA_USERNAME': 'nativefixture', 'VELA_NATIVE_QA_PASSWORD': password, 'VELA_NATIVE_QA_REPORT': str(report), 'XDG_DATA_HOME': str(private / 'home'), 'XDG_CONFIG_HOME': str(private / 'home'), 'XDG_CACHE_HOME': str(private / 'home/cache'), 'CARGO_BUILD_JOBS': '2'})
                if args.test_session:
                    session_service = 'org.vela.music.desktop.qa.' + secrets.token_hex(8)
                    environment.update({'VELA_NATIVE_KEYRING_SERVICE': session_service, 'VELA_NATIVE_QA_TEST_SESSION': '1'})
                if args.test_audio_engine:
                    with (evidence / 'audio-unit.log').open('wb') as audio_log:
                        subprocess.run(['cargo', 'test', '--locked', '-p', 'vela-desktop', '--lib', 'audio::tests::native_owned_null_sink_play_pause_seek_eof_replace_stop_and_cleanup', '--', '--ignored', '--exact', '--nocapture'], cwd=ROOT, env=environment, stdout=audio_log, stderr=subprocess.STDOUT, timeout=180, check=True)
                environment['VELA_NATIVE_QA_CLOSE_PLAYING'] = '1' if args.close_while_playing else '0'
                stop_request = private / 'server-stop.request'
                stopped_ack = private / 'server-stopped.ack'
                for marker in (stop_request, stopped_ack):
                    marker.unlink(missing_ok=True)
                environment.update({'VELA_NATIVE_QA_STOP_REQUEST': str(stop_request), 'VELA_NATIVE_QA_STOPPED_ACK': str(stopped_ack)})
                driver = subprocess.Popen([str(args.driver.resolve())], env=environment, stdout=window_log, stderr=subprocess.STDOUT)
                while driver.poll() is None:
                    if stop_request.exists():
                        if server.poll() is None:
                            server.send_signal(signal.SIGTERM)
                            server.wait(timeout=30)
                        stopped_ack.write_text('stopped')
                        stop_request.unlink(missing_ok=True)
                    time.sleep(0.1)
                driver.wait(timeout=30)
                result = json.loads(report.read_text())
                if driver.returncode or not result['result']['ok'] or not all(image['passed'] for image in result['screenshots']):
                    raise RuntimeError(f'Native window verification failed; inspect {report}')
                originals = list((private / 'temporary').glob('vela-native-*'))
                result['temporary_originals_after_exit'] = len(originals)
                report.write_text(json.dumps(result, indent=2) + '\n')
                if originals:
                    raise RuntimeError('Native process left a completed temporary original after exit')
                offline_audio = list((private / 'home' / 'vela' / 'offline').glob('*/*.audio'))
                offline_json = list((private / 'home' / 'vela' / 'offline').glob('*/*.json'))
                if len(offline_audio) != 1 or len(offline_json) != 1:
                    raise RuntimeError('Offline fixture did not persist exactly one audio+manifest pair')
                if any(path.stat().st_mode & 0o077 for path in offline_audio + offline_json):
                    raise RuntimeError('Offline fixture files are not private0600')
                offline_manifest = json.loads(offline_json[0].read_text())
                if offline_manifest.get('version') != 2 or len(offline_manifest.get('digest', '')) != 64:
                    raise RuntimeError('Offline manifest does not carry a v2 BLAKE3 digest')
                result['offline_copies'] = {
                    'audio': str(offline_audio[0]),
                    'manifest': str(offline_json[0]),
                    'bytes': offline_audio[0].stat().st_size,
                    'digest': offline_manifest['digest'],
                }
                report.write_text(json.dumps(result, indent=2) + '\n')
                if args.test_peer:
                    if not tls_connections:
                        raise RuntimeError('TLS-terminated peer received no native TLS connections')
                    result['tls_peer'] = {
                        'origin': peer2_origin,
                        'connections': len(tls_connections),
                        'ca': str(ca_pem),
                    }
                    report.write_text(json.dumps(result, indent=2) + '\n')
                    config_path = private / 'home' / 'vela' / 'peers.json'
                    config = json.loads(config_path.read_text())
                    expected = sorted([peer_origin + '/', peer2_origin + '/', cached_origin + '/'])
                    if config.get('version') != 1 or config.get('peers') != expected:
                        raise RuntimeError(f'Saved peer config mismatch: {config!r} expected {expected!r}')
                    if config_path.stat().st_mode & 0o077:
                        raise RuntimeError('Saved peer config is not private0600')
                    result['saved_peer_config'] = {'path': str(config_path), 'peers': config['peers']}
                    report.write_text(json.dumps(result, indent=2) + '\n')
                if args.test_session:
                    if not session_service:
                        raise RuntimeError('Session fixture service was not configured')
                    def secret_lookup():
                        return subprocess.run(['secret-tool', 'lookup', 'service', session_service], capture_output=True, text=True)
                    stored = secret_lookup()
                    if stored.returncode != 0 or not stored.stdout.strip():
                        raise RuntimeError('Opt-in session was not written to the Secret Service')
                    envelope = json.loads(stored.stdout)
                    if envelope.get('version') != 1 or envelope.get('origin') != origin + '/' or len(envelope.get('token', '')) != 64:
                        raise RuntimeError('Saved-session envelope is malformed')
                    if 'password' in envelope or 'csrf' in envelope or 'username' in envelope:
                        raise RuntimeError('Saved-session envelope contains forbidden credentials')
                    offline_path = Path(result['offline_copies']['audio'])
                    tampered = bytearray(offline_path.read_bytes())
                    tampered[0] ^= 0x7f
                    offline_path.write_bytes(tampered)
                    resume_env = environment.copy()
                    with (evidence / 'session-resume-server.log').open('wb') as resume_log:
                        server = subprocess.Popen([str(args.server.resolve())], env=resume_env, stdout=resume_log, stderr=subprocess.STDOUT)
                    resume_deadline = time.monotonic() + 15
                    while True:
                        try:
                            with health.open(origin + '/health', timeout=1) as response:
                                if response.status == 200:
                                    break
                        except OSError:
                            if server.poll() is not None or time.monotonic() > resume_deadline:
                                raise RuntimeError('Primary server did not restart for session resume')
                            time.sleep(.05)
                    resume_report = evidence / 'session-resume-report.json'
                    resume_env.update({'VELA_NATIVE_QA_SESSION_PHASE': 'resume', 'VELA_NATIVE_QA_REPORT': str(resume_report)})
                    with (evidence / 'session-resume-window.log').open('wb') as resume_window_log:
                        resume_driver = subprocess.Popen([str(args.driver.resolve())], env=resume_env, stdout=resume_window_log, stderr=subprocess.STDOUT)
                        resume_driver.wait(timeout=120)
                    if resume_driver.returncode or not resume_report.exists():
                        raise RuntimeError('Saved-session resume process failed')
                    resumed = json.loads(resume_report.read_text())
                    if not resumed['result']['ok']:
                        raise RuntimeError(f"Saved-session resume failed: {resumed['result'].get('failure')}")
                    cleared = secret_lookup()
                    if cleared.returncode == 0 and cleared.stdout.strip():
                        raise RuntimeError('Saved session survived explicit sign-out')
                    result['session_resume'] = {
                        'stored_origin': envelope['origin'],
                        'token_stored': True,
                        'contains_password': False,
                        'resumed_checks': resumed['result']['checks'],
                        'cleared_after_signout': True,
                    }
                    report.write_text(json.dumps(result, indent=2) + '\n')
                print(json.dumps(result, indent=2))
    finally:
        for process in (driver, server, peer_process, peer2_process):
            if process is not None and process.poll() is None:
                process.send_signal(signal.SIGTERM)
                try:
                    process.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
        if session_service:
            subprocess.run(['secret-tool', 'clear', 'service', session_service], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        subprocess.run(['pactl', 'unload-module', module], check=True)


if __name__ == '__main__':
    main()
