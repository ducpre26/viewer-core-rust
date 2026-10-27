"""Real viewer process against a local Postman WADO store and configured PostgreSQL.

Uses the existing Summer TOML/profile contract; temporary test TOML changes only
the listening port/logging/JNI flag, and disables client Basic auth for a cache
session fixture. No production configuration files or business data are changed.
"""
import json
import os
from pathlib import Path
import re
import shutil
import socket
import subprocess
import tempfile
import time
import tomllib
import urllib.error
import urllib.request

import config_fixture
from smoke_app import stop

ROOT = Path(__file__).resolve().parents[1]
LOGS = ROOT / 'target/step-11-logs'
HTTP = urllib.request.build_opener(urllib.request.ProxyHandler({}))


def free_port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]


def configured():
    # Test-only materialization of the currently selected TOML profile. The
    # application itself continues to load its normal Summer registry.
    def read(path):
        source = path.read_text(encoding='utf-8')
        source = re.sub(r'\$\{([^}:]+)(?::([^}]*))?\}',
                        lambda m: os.environ.get(m[1], m[2] or ''), source)
        return tomllib.loads(source)
    result = read(ROOT / 'config/app.toml')
    overlay = ROOT / f"config/app-{os.environ.get('SUMMER_ENV', 'dev')}.toml"
    return config_fixture.merge(result, read(overlay)) if overlay.exists() else result


def request(base, path, method='GET', body=None):
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(base + path, data=data, method=method,
                                 headers={'Content-Type': 'application/json'})
    try:
        response = HTTP.open(req, timeout=5)
    except urllib.error.HTTPError as error:
        response = error
    with response:
        return response.status, response.headers, response.read()


def run():
    LOGS.mkdir(parents=True, exist_ok=True)
    port, sim_port = free_port(), free_port()
    binary = ROOT / 'target/debug' / ('viewer-app.exe' if os.name == 'nt' else 'viewer-app')
    cli = shutil.which('postman')
    assert cli and binary.exists(), 'Requires Postman CLI >=1.70 and cargo build --workspace --locked'
    options = {}
    if os.name == 'nt':
        import ctypes
        ctypes.windll.kernel32.SetConsoleCtrlHandler(None, False)
        startup = subprocess.STARTUPINFO()
        startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        startup.wShowWindow = subprocess.SW_HIDE
        options.update(creationflags=subprocess.CREATE_NEW_CONSOLE, startupinfo=startup)
    simulation = app = None
    with tempfile.TemporaryDirectory(prefix='wado-smoke-', dir=ROOT/'target') as temporary, \
            (LOGS/'simulation.ndjson').open('w+b') as sim_log, \
            (LOGS/'simulation.err').open('w+b') as sim_err, \
            (LOGS/'app.log').open('w+b') as app_log:
        try:
            simulation = subprocess.Popen([cli, 'simulation', 'run', 'postman/simulations/wado.sim.yaml',
                                           '--port', str(sim_port), '--output', 'ndjson', '--no-history'],
                                          cwd=ROOT, stdout=sim_log, stderr=sim_err, **options)
            deadline = time.monotonic() + 30
            while True:
                assert simulation.poll() is None, 'Simulation exited; see simulation.err'
                events = (LOGS/'simulation.ndjson').read_text(errors='replace')
                if '"listening"' in events:
                    break
                assert time.monotonic() < deadline, 'Simulation startup timed out'
                time.sleep(0.1)
            config = config_fixture.merge(configured(), {
                'web': {'binding': '127.0.0.1', 'port': port},
                'auth': {'client_security_disabled': True},
                'jni': {'enabled': False}, 'logger': {'level': 'warn'},
            })
            working = Path(temporary)
            config_fixture.write(working/'config/app.toml', config)
            app = subprocess.Popen([str(binary)], cwd=working, stdout=app_log, stderr=app_log, **options)
            base = f'http://127.0.0.1:{port}/ws/rest'
            deadline = time.monotonic() + 30
            while True:
                assert app.poll() is None, 'Viewer startup failed; see app.log'
                try:
                    if request(base, '/ready')[0] == 200:
                        break
                except (urllib.error.URLError, TimeoutError):
                    pass
                assert time.monotonic() < deadline, 'Viewer startup timed out'
                time.sleep(0.1)
            store = f'http://127.0.0.1:{sim_port}/store/dicom/wado-rs'
            status, _, body = request(base, '/client/link', 'POST', {
                'storeUrl': store, 'storeAuth': 'Basic fixture-secret', 'studies': ['1.2.3'],
            })
            assert status == 200, f'Session creation status {status}'
            session = json.loads(body)['session']['id']
            try:
                prefix = f'/v1/session/{session}'
                for route in [
                    '/wado-rs/studies/1.2.3',
                    '/wado-rs/studies/1.2.3/series/4.5/instances/6.7/frames/1,2',
                    '/wado-uri?studyUID=1.2.3&seriesUID=4.5&objectUID=6.7',
                ]:
                    status, headers, payload = request(base, prefix + route)
                    assert status == 200 and payload == bytes([0, 1, 2, 13, 10, 255, 42])
                    assert 'boundary=fixture' in headers['Content-Type']
                    assert 'Set-Cookie' not in headers and 'X-Upstream-Secret' not in headers
                for code in [401, 403, 404, 429, 500, 503]:
                    status, _, payload = request(base, prefix + f'/wado-rs/studies/1.2.3/thumbnail?fixtureStatus={code}')
                    assert status == code and payload == bytes([0, 1, 2, 13, 10, 255, 42])
                status, _, body = request(base, prefix + '/wado-rs/studies/outside')
                assert status == 500 and body == b''
                assert request(base, '/v1/session/step11-missing/wado-rs/studies/1.2.3')[0] == 401
            finally:
                assert request(base, prefix, 'DELETE')[0] == 200
            stop(app)
            app = None
            events = [json.loads(line) for line in (LOGS/'simulation.ndjson').read_text().splitlines() if line.startswith('{')]
            assert any(event.get('type') == 'request' or event.get('event') == 'request' for event in events), 'No member request events'
            print('WADO runtime smoke passed: real viewer, configured PostgreSQL, local Postman store; session cleaned, graceful shutdown.')
        finally:
            if app and app.poll() is None:
                stop(app)
            if simulation and simulation.poll() is None:
                if os.name == 'nt':
                    subprocess.run(['taskkill', '/PID', str(simulation.pid), '/T', '/F'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
                else:
                    simulation.terminate()
                simulation.wait(timeout=10)


if __name__ == '__main__':
    run()
