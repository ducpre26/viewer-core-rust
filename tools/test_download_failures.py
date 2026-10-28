"""Real HTTP/FFmpeg checks for cancellation, dependency failures and response timing.

Uses synthetic local Postman store data and the production debug viewer/facade.
No real PACS requests, no disk-full/soak/platform certification in this runner.
"""
import argparse
import base64
import hashlib
import http.client
import json
import os
from pathlib import Path
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import time
import urllib.parse

import config_fixture
from smoke_wado import configured
from smoke_app import stop
import smoke_jni

ROOT = Path(__file__).resolve().parents[1]
LOGS = ROOT / 'target/step-12-error-logs'
FIXTURES = ROOT / 'target/step-12-error-fixtures'
sys.path.insert(0, str(ROOT / 'target/step-12-tools'))
RESULTS = []
CONNECTIONS = []

def wait(predicate, timeout=30, message='condition timed out'):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = predicate()
        if result:
            return result
        time.sleep(.02)
    raise AssertionError(message)

def port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]

def connect(url, method='GET', body=None, small_window=False):
    parsed = urllib.parse.urlsplit(url)
    connection = http.client.HTTPConnection(parsed.hostname, parsed.port, timeout=90)
    connection.connect()
    if small_window:
        connection.sock.setsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF, 16384)
    payload = json.dumps(body).encode() if body is not None else None
    headers = {'Content-Type': 'application/json'} if body is not None else {}
    if body is not None and method == 'GET':
        method = 'POST'
    connection.request(method, urllib.parse.urlunsplit(('', '', parsed.path, parsed.query, '')), payload, headers)
    CONNECTIONS.append(connection)
    return connection

def request(url, method='GET', body=None):
    connection = connect(url, method, body)
    try:
        response = connection.getresponse()
        return response.status, dict(response.getheaders()), response.read()
    finally:
        connection.close()

def has_headers(connection):
    connection.sock.settimeout(.02)
    try:
        return bool(connection.sock.recv(1, socket.MSG_PEEK))
    except (TimeoutError, BlockingIOError):
        return False
    finally:
        connection.sock.settimeout(90)

def abort(connection):
    if connection.sock:
        connection.sock.setsockopt(socket.SOL_SOCKET, socket.SO_LINGER,
            struct.pack('HH' if os.name == 'nt' else 'ii', 1, 0))
    connection.close()

def record(name, **details):
    RESULTS.append({'case': name, 'passed': True, **details})
    (LOGS / 'results.json').write_text(json.dumps(RESULTS, indent=2)+'\n', encoding='utf-8')
    print('PASS '+name, flush=True)

def process_options():
    if os.name != 'nt':
        return {}
    import ctypes
    ctypes.windll.kernel32.SetConsoleCtrlHandler(None, False)
    startup = subprocess.STARTUPINFO()
    startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
    startup.wShowWindow = subprocess.SW_HIDE
    return {'creationflags': subprocess.CREATE_NEW_CONSOLE, 'startupinfo': startup}

class App:
    def __init__(self, root, store, java, timeout_ms, frames):
        self.files = root / 'files'
        self.files.mkdir()
        self.base = f'http://127.0.0.1:{port()}/ws/rest'
        web_port = urllib.parse.urlsplit(self.base).port
        settings = smoke_jni.jni_settings(java)
        settings.update(xmx_mb=384, shutdown_ms=10000)
        config = config_fixture.merge(configured(), {
            'web': {'binding': '127.0.0.1', 'port': web_port, 'middlewares': {'logger': {'enable': False}}},
            'auth': {'client_security_disabled': True}, 'logger': {'level': 'warn'}, 'jni': settings,
            'download': {'enabled': True, 'temp_dir': str(self.files), 'max_jobs': 1,
                'timeout_ms': timeout_ms, 'max_input_bytes': 256*1024*1024,
                'max_output_bytes': 128*1024*1024, 'max_job_bytes': 512*1024*1024,
                'max_metadata_bytes': 8*1024*1024, 'max_entries': 100,
                'max_frames': frames+10, 'max_pixels': 512*512*(frames+10), 'video_fps': 10,
                'cleanup_ttl_seconds': 1, 'cleanup_interval_seconds': 1},
        })
        config_fixture.write(root / 'config/app.toml', config)
        env = smoke_jni.environment(java)
        env.update(SUMMER_ENV='dev', RUST_LOG='warn')
        self.log = (LOGS / f'app-{timeout_ms}.log').open('wb')
        binary = ROOT / 'target/debug' / ('viewer-app.exe' if os.name == 'nt' else 'viewer-app')
        self.process = subprocess.Popen([str(binary)], cwd=root, env=env,
            stdout=self.log, stderr=self.log, **process_options())
        self.session = None
        try:
            def ready():
                assert self.process.poll() is None, 'viewer exited during startup'
                try:
                    return request(self.base+'/ready')[0] == 200
                except OSError:
                    return False
            wait(ready, 45, 'viewer readiness timeout')
            self.set_store(store)
        except Exception:
            self.close()
            raise

    def set_store(self, store):
        if self.session:
            request(self.base+'/v1/session/'+self.session, 'DELETE')
        status, _, data = request(self.base+'/client/link', body={
            'storeUrl': store, 'storeAuth': 'Basic fixture-secret', 'studies': ['1.2.3'], 'expireIn': 10})
        assert status == 200
        self.session = json.loads(data)['session']['id']

    def url(self, kind='dicom', scope='instance'):
        suffix = '' if scope == 'study' else '/series/1.2.3.4'
        if scope == 'instance':
            suffix += '/instances/1.2.3.4.5'
        return f'{self.base}/v1/session/{self.session}/download/studies/1.2.3{suffix}?downloadType={kind}'

    def clean(self, timeout=45):
        wait(lambda: not any(self.files.iterdir()), timeout, 'job files/permits not released')

    def close(self):
        for connection in CONNECTIONS:
            connection.close()
        if self.process.poll() is None:
            try:
                if self.files.exists():
                    self.clean(90)
                if self.session:
                    request(self.base+'/v1/session/'+self.session, 'DELETE')
                stop(self.process)
            except Exception:
                if self.process.poll() is None:
                    self.process.kill()
                    self.process.wait(timeout=5)
                raise
            finally:
                self.log.close()

def generate(java, frames):
    FIXTURES.mkdir(parents=True, exist_ok=True)
    suffix = '.exe' if os.name == 'nt' else ''
    dist = ROOT / 'java/dicom-facade/target/distribution'
    cp = os.pathsep.join(map(str, [FIXTURES, dist/'pacs-dicom-facade.jar', *sorted((dist/'dependencies').glob('*.jar'))]))
    env = smoke_jni.environment(java)
    with (LOGS/'generate.log').open('wb') as log:
        for command in [
            [java/'bin'/f'javac{suffix}', '-cp', cp, '-d', FIXTURES, ROOT/'tools/DownloadFailureFixture.java'],
            [java/'bin'/f'java{suffix}', '-Xmx64m', '-cp', cp, 'DownloadFailureFixture', FIXTURES/'video.dcm', frames],
        ]:
            subprocess.run(list(map(str, command)), env=env, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)
    media = json.loads((ROOT/'docs/baseline/step-12-media.json').read_text(encoding='utf-8'))
    jpeg = json.loads((ROOT/'docs/baseline/step-12-rendered-jpeg.json').read_text(encoding='utf-8'))
    manifest = {'frames': frames, 'videoPath': str((FIXTURES/'video.dcm').resolve()),
        'dicomBase64': media['inputDicomBase64'], 'jpegBase64': jpeg['framesBase64'][0]}
    (FIXTURES/'manifest.json').write_text(json.dumps(manifest), encoding='utf-8')
    return manifest

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--java-home', type=Path, required=True)
    parser.add_argument('--frames', type=int, default=384)
    parser.add_argument('--transport-only', action='store_true')
    args = parser.parse_args()
    assert 64 <= args.frames <= 768
    java = args.java_home.resolve()
    LOGS.mkdir(parents=True, exist_ok=True)
    manifest = generate(java, args.frames)
    cli = shutil.which('postman')
    assert cli, 'Postman CLI is required'
    sim_log = (LOGS/'simulation.ndjson').open('wb')
    sim_err = (LOGS/'simulation.err').open('wb')
    sim = subprocess.Popen([cli, 'simulation', 'run', 'postman/simulations/download-failures.sim.yaml',
        '--port', str(port()), '--output', 'ndjson', '--no-history'], cwd=ROOT,
        stdout=sim_log, stderr=sim_err, **process_options())
    app = None
    working = None
    try:
        def listening():
            assert sim.poll() is None, 'simulation exited; see simulation.err'
            for line in (LOGS/'simulation.ndjson').read_text(encoding='utf-8', errors='replace').splitlines():
                try:
                    event = json.loads(line)
                except ValueError:
                    continue
                if event.get('type') == 'listening' or event.get('event') == 'listening':
                    return event
            return None
        event = wait(listening, 30, 'simulation listening timeout')
        (LOGS/'listening.json').write_text(json.dumps(event, indent=2), encoding='utf-8')
        # Use the origin reported by the simulation's listening event.
        addresses = json.dumps(event)
        import re
        match = re.search(r'http://(?:127\.0\.0\.1|localhost):\d+(?:/store)?', addresses)
        assert match, 'No address in simulation listening event'
        origin = match.group(0).removesuffix('/store').replace('http://localhost:', 'http://127.0.0.1:')
        control = origin+'/store/__control'
        def mode(name):
            status, _, body = request(control+'?'+urllib.parse.urlencode({'mode': name}))
            assert status == 200
            return json.loads(body)
        def state():
            return json.loads(request(control)[2])
        store = origin+'/store/dicom/wado-rs'
        working = tempfile.mkdtemp(prefix='download-errors-', dir=ROOT/'target')
        if working:
            app = App(Path(working).resolve(), store, java, 800, args.frames)
            # Gate after one upstream chunk; make sure bytes really reached Rust.
            mode('gated')
            connection = connect(app.url())
            wait(lambda: state()['firstChunk'], 5)
            def partial():
                return any(path.stat().st_size > 0 for path in app.files.glob('download-*/*.bin'))
            wait(partial, .6, 'Rust did not receive the partial source body before deadline')
            assert not has_headers(connection), 'download headers escaped before upstream EOF'
            released = time.monotonic()
            request(control+'?release=1')
            response = connection.getresponse()
            received_headers = time.monotonic()
            body = response.read()
            assert response.status == 200 and body == base64.b64decode(manifest['dicomBase64'])
            connection.close();app.clean()
            record('headers-after-upstream-eof', headersAfterReleaseMs=round((received_headers-released)*1000, 2))

            # The simulation gateway does not preserve a broken upstream socket:
            # run wire-level failures against the same member's direct listener.
            direct = state()['directPort']
            store = f'http://127.0.0.1:{direct}/dicom/wado-rs'
            app.set_store(store)

            cases = [(f'status-{code}', 'dicom', 'instance', 500) for code in [401,403,404,429,500,503]]
            cases += [(f'status-{code}', 'jpeg', 'instance', 500) for code in [401,403,429,503]]
            cases += [('html-502','dicom','instance',500), ('metadata-503','dicom','series',500),
                ('body-truncated','dicom','instance',500), ('body-truncated','jpeg','instance',500),
                ('headers-hang','dicom','instance',504), ('body-hang','dicom','instance',504),
                ('second-instance-fails','dicom','series',500), ('second-frame-fails','jpeg','instance',500)]
            for name, kind, scope, expected in cases:
                mode(name)
                started = time.monotonic()
                status, headers, body = request(app.url(kind,scope))
                assert status == expected, f'{name}/{kind}: expected {expected}, got {status}, body={body[:120]!r}'
                assert not any(key.lower()=='content-disposition' for key in headers)
                assert b'STORE_PRIVATE_MARKER' not in body
                assert json.loads(body)['status'] == expected
                app.clean()
                record(f'{name}-{kind}-{scope}', status=status, elapsedMs=round((time.monotonic()-started)*1000,2), calls=len(state()['requests']))
            mode('ok')
            app.set_store('http://127.0.0.1:'+str(port())+'/dicom/wado-rs')
            unreachable_status = request(app.url())[0]
            assert unreachable_status in (500, 504), unreachable_status
            app.clean();app.set_store(store)
            record('store-unreachable', status=unreachable_status)
            app.close();app=None

        if not args.transport_only:
            run_native_and_committed_response_tests(store, mode, java, args.frames)
        print(f'PASS {len(RESULTS)} selected download checks', flush=True)
    finally:
        if app is not None:
            app.close()
        if sim.poll() is None:
            sim.terminate()
            try:
                sim.wait(timeout=10)
            except subprocess.TimeoutExpired:
                sim.kill();sim.wait(timeout=5)
        sim_log.close();sim_err.close()
        if working:
            target=Path(working).resolve()
            assert target.is_relative_to((ROOT/'target').resolve()) and target.name.startswith('download-errors-')
            shutil.rmtree(target)

def run_native_and_committed_response_tests(store, mode, java, frames):
    import imageio_ffmpeg
    root = Path(tempfile.mkdtemp(prefix='download-errors-native-', dir=ROOT/'target')).resolve()
    app = None
    def active_video():
        for path in app.files.glob('download-*/*.bin'):
            try:
                with path.open('rb') as stream:
                    prefix = stream.read(65536)
                if prefix[4:8] == b'ftyp' and b'moof' in prefix and path.stat().st_size > 2048:
                    return path
            except (FileNotFoundError, PermissionError):
                pass
        return None
    try:
        app = App(root, store, java, 60000, frames)
        mode('video')
        started = time.monotonic()
        status, _, body = request(app.url('mp4','series'))
        assert status == 200, body[:200]
        output = LOGS/'real-video.mp4'
        output.write_bytes(body)
        decoded = subprocess.run([imageio_ffmpeg.get_ffmpeg_exe(), '-v', 'error', '-i', str(output),
            '-map', '0:v:0', '-progress', 'pipe:1', '-nostats', '-f', 'null', '-'],
            capture_output=True, text=True, timeout=90, check=True)
        counts = [int(line[6:]) for line in decoded.stdout.splitlines() if line.startswith('frame=')]
        assert counts and counts[-1] == frames, decoded.stdout
        baseline_seconds = time.monotonic()-started
        app.clean()
        record('real-ffmpeg-complete', frames=frames, bytes=len(body), seconds=round(baseline_seconds,2))

        mode('video')
        connection = connect(app.url('mp4','series'))
        output = wait(active_video, 30, 'real encoder did not produce fragmented MP4')
        initial = output.stat().st_size
        wait(lambda: output.exists() and output.stat().st_size > initial, 10, 'encoder output did not grow')
        assert not has_headers(connection), 'MP4 headers committed before encode completed'
        job = output.parent
        abort(connection)
        assert job.exists(), 'native output deleted while encoding'
        assert request(app.url())[0] == 503, 'native cancellation released permit early'
        orphan = app.files/'download-orphan-control'
        orphan.mkdir(); (orphan/'.lease').touch()
        old = time.time()-60
        os.utime(orphan/'.lease', (old,old))
        os.utime(job/'.lease', (old,old))
        wait(lambda: not orphan.exists(), 5, 'TTL reaper did not run')
        assert job.exists(), 'TTL reaper deleted running native job'
        app.clean(90)
        assert request(app.base+'/ready')[0] == 200
        mode('ok'); assert request(app.url())[0] == 200; app.clean()
        record('disconnect-during-real-encode', retainedUntilNativeCompletion=True, ttlSkippedActive=True)

        mode('large-file')
        connection = connect(app.url(), small_window=True)
        response = connection.getresponse()
        assert response.status == 200
        mode('status-503')
        digest = hashlib.sha256()
        while chunk := response.read(1024*1024):
            digest.update(chunk)
        connection.close(); app.clean()
        with (FIXTURES/'video.dcm').open('rb') as source:
            expected = hashlib.file_digest(source, 'sha256').hexdigest()
        assert digest.hexdigest() == expected
        assert request(app.url())[0] == 500
        app.clean()
        record('store-fails-after-response-headers', currentDownload='complete-byte-identical', nextDownloadStatus=500)

        mode('large-file')
        connection = connect(app.url(), small_window=True)
        response = connection.getresponse()
        assert response.status == 200
        abort(connection)
        response.close()
        app.clean()
        mode('ok'); assert request(app.url())[0] == 200; app.clean()
        record('disconnect-during-response-stream', cleanup=True, permitRecovered=True)

        mode('large-file')
        connection = connect(app.url(), small_window=True)
        response = connection.getresponse()
        assert response.status == 200
        assert int(response.getheader('Content-Length')) == (FIXTURES/'video.dcm').stat().st_size
        candidates = list(app.files.glob('download-*/*.bin'))
        assert len(candidates) == 1, candidates
        damaged = candidates[0].resolve()
        assert damaged.is_relative_to(app.files.resolve())
        # Artificial fault in this test job only, after response commitment.
        with damaged.open('r+b') as staged:
            staged.truncate(0)
        try:
            response.read()
        except http.client.IncompleteRead as error:
            assert error.expected > 0
        else:
            raise AssertionError('truncated staged file looked like a complete successful download')
        connection.close(); app.clean()
        mode('ok'); assert request(app.url())[0] == 200; app.clean()
        record('staged-file-truncated-after-headers', committedStatus=200, clientDetectedIncomplete=True)
        app.close(); app = None
    finally:
        if app is not None:
            app.close()
        assert root.is_relative_to((ROOT/'target').resolve()) and root.name.startswith('download-errors-')
        shutil.rmtree(root)

    root = Path(tempfile.mkdtemp(prefix='download-errors-deadline-', dir=ROOT/'target')).resolve()
    app = None
    try:
        # Deadline leaves enough time for source retrieval and Java initialization,
        # while the noisy real video keeps FFmpeg busy beyond the deadline.
        timeout_ms = 3000
        app = App(root, store, java, timeout_ms, frames)
        mode('video')
        connection = connect(app.url('mp4','series'))
        output = wait(active_video, 10, 'deadline expired before real encode was observed')
        response = connection.getresponse()
        assert response.status == 504, response.read()[:200]
        assert json.loads(response.read())['status'] == 504
        connection.close()
        assert output.parent.exists(), 'deadline deleted active native resources'
        assert request(app.url())[0] == 503
        assert request(app.base+'/ready')[0] == 503
        assert request(app.base+'/health')[0] == 200
        app.clean(90)
        assert request(app.base+'/ready')[0] == 200
        mode('ok'); assert request(app.url())[0] == 200; app.clean()
        record('deadline-during-real-encode', status=504, busyStatus=503, readinessRecovered=True)
    finally:
        if app is not None:
            app.close()
        assert root.is_relative_to((ROOT/'target').resolve()) and root.name.startswith('download-errors-')
        shutil.rmtree(root)

if __name__ == '__main__':
    main()
