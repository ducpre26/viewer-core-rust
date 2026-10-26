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
import uuid
import config_fixture

ROOT = Path(__file__).resolve().parents[1]
HTTP = urllib.request.build_opener(urllib.request.ProxyHandler({}))


def send_stop_signal(process):
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


def stop(process):
    send_stop_signal(process)
    assert process.wait(timeout=15) == 0, "Application exited unsuccessfully"


def check(profile, binary, jni_settings=None, process_env=None, framework_overrides=False):
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    env = os.environ.copy()
    if process_env:
        env.update(process_env)
    env.pop("RUST_LOG", None)  # Test deterministic config logging; app keeps override support.
    env["SUMMER_ENV"] = profile
    # Literal TOML must win over variables the old modules read directly.
    env.update(VIEWER_CONFIG="absent-config", VIEWER_JNI_ENABLED="invalid",
               VIEWER_CLIENT_SECURITY_DISABLED="invalid", VIEWER_DEFAULT_EXPIRE_MINUTES="invalid",
               VIEWER_DATABASE_URL="postgresql://127.0.0.1:1/absent")
    configured = config_fixture.merge(config_fixture.settings(), {
        "web": {"binding": "127.0.0.1", "port": 0},
        "auth": {"client_security_disabled": True},
        "session": {"default_expire_minutes": 7, "path_prefix": "/smoke?session="},
        "jni": jni_settings or {"enabled": False},
    })
    if framework_overrides:
        env["TOML_TEST_PREFIX"] = "/smoke?session="
        configured["session"]["path_prefix"] = "${TOML_TEST_PREFIX}"
        env["TOML_TEST_DATABASE_URL"] = configured["database"]["url"]
        configured["database"]["url"] = "${TOML_TEST_DATABASE_URL}"
        env["RUST_LOG"] = "error"
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
    log_dir = ROOT / "target/step-03-logs"
    log_dir.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="toml-smoke-", dir=ROOT / "target") as temporary, (log_dir / f"app-{profile}-{'jni' if jni_settings else 'no-jni'}.log").open("w+b") as logs:
        working = Path(temporary)
        config_fixture.write(working / "config/app.toml", configured)
        config_fixture.write(working / f"config/app-{profile}.toml", {
            "web": {"port": port}, "logger": {"format": "json" if profile == "prod" else "compact"}
        })
        process = subprocess.Popen([str(binary)], cwd=working, env=env,
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
            dependencies = ["postgresql:up"] + (["jvm:up"] if jni_settings else [])
            assert get("/ready") == {"status": "ready", "dependencies": dependencies}
            spec = get("/docs/openapi.json")
            assert spec["info"]["title"] == "Viewer Core Infrastructure"
            for path in ("/health", "/ready"):
                operation = spec["paths"]["/ws/rest" + path]["get"]
                assert "200" in operation["responses"]
            # Step 8: use the actual process/TCP stack and its shared session cache.
            def post(path, body):
                request = urllib.request.Request(base + path,
                    data=json.dumps(body).encode("utf-8"),
                    headers={"Content-Type": "application/json"}, method="POST")
                with HTTP.open(request, timeout=5) as response:
                    assert response.status == 200
                    return json.load(response)

            created = post("/client/link", {"storeUrl": "http://store.invalid", "studies": ["1.2.3"]})
            session = created["session"]
            assert uuid.UUID(session["id"]).version == 4
            assert created["link"] == "/smoke?session=" + session["id"]
            assert session["permissions"] is None
            assert session["studies"] == ["1.2.3"]
            assert get("/v1/session/" + session["id"] + "/config")["header"]["code"] == 302
            legacy = post("/client/getlink", {"studyUIDs": {"1.2.3": {"url": "http://store.invalid"}}})
            assert legacy["header"]["code"] == 200
            assert legacy["body"].startswith("/smoke?session=")
            for path in ("/client/link", "/client/getlink"):
                assert "post" in spec["paths"]["/ws/rest" + path]
            try:
                get("/not-implemented")
                raise AssertionError("Unknown route must return 404")
            except urllib.error.HTTPError as error:
                assert error.code == 404
            stop(process)
            logs.seek(0)
            output = logs.read().decode("utf-8", errors="replace")
            if not framework_overrides:
                assert "shutdown" in output.lower(), "Missing graceful shutdown evidence"
            assert "PostgreSQL pool closed" in output, "Pool did not close"
            if jni_settings:
                for evidence in ("JVM ready", "JNI worker detached", "JVM destroyed"):
                    assert evidence in output, f"Missing JNI lifecycle evidence: {evidence}"
                assert "WARNING in native method" not in output
            assert "503" in spec["paths"]["/ws/rest/ready"]["get"]["responses"]
            for secret in (configured["database"]["password"], configured["database"]["url"], configured["auth"]["admin_password"]):
                if secret:
                    assert secret not in output, "Database secret leaked in runtime logs"
            if framework_overrides:
                assert "summer_job" not in output, "RUST_LOG must override logger.level"
            else:
                assert "summer_job" in output, "Scheduler plugin did not start"
            if profile == "prod":
                events = [json.loads(line) for line in output.splitlines() if line.startswith("{")]
                assert events, "Production logging must include JSON events"
            print(f"PASS {profile} JNI={'on' if jni_settings else 'off'}: health, readiness, OpenAPI, session/link HTTP, 404, logging, scheduler, graceful stop")
        except Exception:
            logs.seek(0)
            print(logs.read().decode("utf-8", errors="replace"), file=sys.stderr)
            raise
        finally:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)


def check_startup_failures(binary):
    base = config_fixture.merge(config_fixture.settings(), {"jni": {"enabled": False}, "database": {"timeout_seconds": 1}})
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        unavailable_port = sock.getsockname()[1]
    cases = [
        ({"session": {"default_expire_minutes": "invalid"}}, "Invalid [session]"),
        ({"session": {"default_expire_minutes": 2**63 - 1}}, "out of range"),
        ({"auth": {"client_security_disabled": "invalid"}}, "Invalid [auth]"),
        ({"database": {"url": None}}, "[database]"),
        ({"database": {"password": "smoke-invalid-password-7f38"}}, "authentication failed"),
        ({"database": {"url": f"postgresql://127.0.0.1:{unavailable_port}/viewer_rust"}}, "connection failed"),
        ({"database": {"max_connections": 0}}, "positive integers"),
        ({"database": {"timeout_seconds": 0}}, "positive integers"),
        ({"database": {"typo": "sensitive-marker"}}, "[database]"),
        ({"jni": {"enabled": True}}, "JVM_CONFIG"),
    ]
    for overrides, expected in cases:
        configured = config_fixture.merge(base, overrides)
        with tempfile.TemporaryDirectory(prefix="toml-invalid-", dir=ROOT / "target") as temporary:
            config_fixture.write(Path(temporary) / "config/app.toml", configured)
            env = os.environ.copy()
            env["SUMMER_ENV"] = "test"
            result = subprocess.run([str(binary)], cwd=temporary, env=env,
                                    capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=15)
        output = result.stdout + result.stderr
        assert result.returncode != 0, "Invalid configuration must fail startup"
        assert expected in output, "Missing sanitized startup error category"
        for secret in (configured["database"]["password"], configured["database"].get("url"),
                       configured["auth"]["admin_password"], "sensitive-marker"):
            if secret:
                assert secret not in output, "Config secret leaked in startup logs"
    print("PASS startup failures: TOML field/type/range, DB credentials/connectivity, JNI; no secrets logged")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--release", action="store_true")
    args = parser.parse_args()
    binary = ROOT / "target" / ("release" if args.release else "debug") / (
        "viewer-app.exe" if os.name == "nt" else "viewer-app")
    for profile in ("dev", "prod"):
        check(profile, binary)
    check("dev", binary, framework_overrides=True)
    print("PASS framework defaults: profile overlay, TOML interpolation and RUST_LOG override")
    check_startup_failures(binary)
