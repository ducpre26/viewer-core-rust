"""Run infrastructure smoke checks against a real process (Python standard library)."""
import argparse
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
HTTP = urllib.request.build_opener(urllib.request.ProxyHandler({}))


def stop(process):
    if os.name == "nt":
        # Attach a helper to the child's private, hidden console. Ctrl+C never
        # reaches the user's terminal or another application.
        subprocess.run(
            [sys.executable, "-c", """
import ctypes, sys, time
k = ctypes.WinDLL('kernel32', use_last_error=True)
k.FreeConsole()
if not k.AttachConsole(int(sys.argv[1])):
    raise ctypes.WinError(ctypes.get_last_error())
if not k.SetConsoleCtrlHandler(None, True):
    raise ctypes.WinError(ctypes.get_last_error())
if not k.GenerateConsoleCtrlEvent(0, 0):
    raise ctypes.WinError(ctypes.get_last_error())
time.sleep(0.2)
""", str(process.pid)], check=True, timeout=5,
        )
    else:
        process.send_signal(signal.SIGTERM)
    assert process.wait(timeout=15) == 0, "Application exited unsuccessfully"


def check(profile, binary):
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    env = os.environ.copy()
    env.pop("RUST_LOG", None)
    env.update(SUMMER_ENV=profile, VIEWER_BIND="127.0.0.1", VIEWER_PORT=str(port),
               VIEWER_LOG_LEVEL="info", VIEWER_CONFIG=str(ROOT / "config/app.toml"))
    options = {}
    if os.name == "nt":
        import ctypes
        # Child processes inherit the console Ctrl+C ignore flag from launchers.
        # Clear it in this test runner before creating the isolated console.
        ctypes.windll.kernel32.SetConsoleCtrlHandler(None, False)
        startup = subprocess.STARTUPINFO()
        startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        startup.wShowWindow = subprocess.SW_HIDE
        options.update(creationflags=subprocess.CREATE_NEW_CONSOLE, startupinfo=startup)
    with tempfile.TemporaryFile() as logs:
        process = subprocess.Popen([str(binary)], cwd=ROOT, env=env,
                                   stdout=logs, stderr=logs, **options)
        try:
            base = f"http://127.0.0.1:{port}/ws/rest"

            def get(path):
                with HTTP.open(base + path, timeout=2) as response:
                    assert response.status == 200
                    assert "application/json" in response.headers["Content-Type"]
                    return json.load(response)

            deadline = time.monotonic() + 30
            while True:
                assert process.poll() is None, "Application exited during startup"
                try:
                    assert get("/health") == {"status": "up"}
                    break
                except (urllib.error.URLError, TimeoutError):
                    if time.monotonic() >= deadline:
                        raise AssertionError("Startup timed out")
                    time.sleep(0.1)
            assert get("/ready") == {"status": "ready", "dependencies": []}
            spec = get("/docs/openapi.json")
            assert spec["info"]["title"] == "Viewer Core Infrastructure"
            for path in ("/health", "/ready"):
                operation = spec["paths"]["/ws/rest" + path]["get"]
                assert "200" in operation["responses"]
            try:
                get("/not-implemented")
                raise AssertionError("Unknown route must return 404")
            except urllib.error.HTTPError as error:
                assert error.code == 404
            stop(process)
            logs.seek(0)
            output = logs.read().decode("utf-8", errors="replace")
            assert "shutdown" in output.lower(), "Missing graceful shutdown evidence"
            assert "summer_job" in output, "Scheduler plugin did not start"
            if profile == "prod":
                events = [json.loads(line) for line in output.splitlines() if line.startswith("{")]
                assert events, "Production logging must include JSON events"
            print(f"PASS {profile}: health, readiness, OpenAPI, 404, logging, scheduler, graceful stop")
        except Exception:
            logs.seek(0)
            print(logs.read().decode("utf-8", errors="replace"), file=sys.stderr)
            raise
        finally:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--release", action="store_true")
    args = parser.parse_args()
    binary = ROOT / "target" / ("release" if args.release else "debug") / (
        "viewer-app.exe" if os.name == "nt" else "viewer-app")
    for profile in ("dev", "prod"):
        check(profile, binary)
