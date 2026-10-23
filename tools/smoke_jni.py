"""Run isolated real-JVM foundation tests, with explicit local test limits."""
import argparse
import hashlib
import http.client
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error

ROOT = Path(__file__).resolve().parents[1]


def environment(java_home):
    java_home = java_home.resolve()
    env = os.environ.copy()
    for key in ("PACS_LIBJVM_PATH", "JAVA_TOOL_OPTIONS", "JDK_JAVA_OPTIONS", "_JAVA_OPTIONS"):
        env.pop(key, None)
    temp = ROOT / "target" / "jni-temp"
    native = ROOT / "target" / "jni-native"
    temp.mkdir(parents=True, exist_ok=True)
    native.mkdir(parents=True, exist_ok=True)
    env.update(VIEWER_JNI_ENABLED="true", PACS_JAVA_HOME=str(java_home),
               PACS_FACADE_DIR=str(ROOT / "java/dicom-facade/target/distribution"),
               PACS_JVM_TEMP_DIR=str(temp), PACS_JVM_NATIVE_DIR=str(native),
               PACS_JVM_XMS_MB="32", PACS_JVM_XMX_MB="128", PACS_JVM_DIRECT_MB="32",
               PACS_JVM_TIMEOUT_MS="5000", PACS_JVM_SHUTDOWN_MS="1500", PACS_JVM_CHECK_JNI="true")
    env["PATH"] = str(java_home / "bin") + os.pathsep + env.get("PATH", "")
    return env


def app_faults(binary, env, java_home):
    import smoke_app
    with tempfile.TemporaryDirectory(prefix="jni-app-fixtures-", dir=ROOT / "target") as temporary:
        distribution = Path(temporary).resolve()
        assert distribution.is_relative_to(ROOT / "target")
        (distribution / "dependencies").mkdir()
        classes = distribution / "classes"
        classes.mkdir()
        source = ROOT / "crates/dicom-jvm/tests/fixtures/DicomFacade.java"
        suffix = ".exe" if os.name == "nt" else ""
        jar = distribution / "pacs-dicom-facade.jar"
        for command in (
            [java_home / "bin" / f"javac{suffix}", "--release", "25", "-d", classes, source],
            [java_home / "bin" / f"jar{suffix}", "--create", "--file", jar, "-C", classes, "."],
        ):
            subprocess.run([str(arg) for arg in command], env=env, check=True, timeout=60, capture_output=True)
        manifest = json.loads((Path(env["PACS_FACADE_DIR"]) / "manifest.json").read_text(encoding="utf-8"))
        manifest["artifacts"] = {jar.name: hashlib.sha256(jar.read_bytes()).hexdigest()}
        (distribution / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
        for case in ("hang-startup", "throw-probe", "hang-probe", "java-thread"):
            with socket.socket() as sock:
                sock.bind(("127.0.0.1", 0))
                port = sock.getsockname()[1]
            child_env = env.copy()
            child_env.update(PACS_FACADE_DIR=str(distribution), JNI_TEST_CASE=case,
                             PACS_JVM_TIMEOUT_MS="1500", PACS_JVM_SHUTDOWN_MS="1500",
                             SUMMER_ENV="dev", VIEWER_BIND="127.0.0.1", VIEWER_PORT=str(port))
            options = {}
            if os.name == "nt":
                import ctypes
                ctypes.windll.kernel32.SetConsoleCtrlHandler(None, False)
                startup = subprocess.STARTUPINFO()
                startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
                startup.wShowWindow = subprocess.SW_HIDE
                options.update(creationflags=subprocess.CREATE_NEW_CONSOLE, startupinfo=startup)
            log = ROOT / "target/step-03-logs" / f"app-fault-{case}.log"
            with log.open("w") as output:
                process = subprocess.Popen([str(binary)], cwd=ROOT, env=child_env,
                                           stdout=output, stderr=subprocess.STDOUT, **options)
                try:
                    base = f"http://127.0.0.1:{port}/ws/rest"
                    if case != "hang-startup":
                        deadline = time.monotonic() + 20
                        while True:
                            assert process.poll() is None, "Premature app exit"
                            try:
                                with smoke_app.HTTP.open(base + "/health", timeout=1) as response:
                                    assert response.status == 200
                                    break
                            except (urllib.error.URLError, TimeoutError):
                                assert time.monotonic() < deadline, "App startup timeout"
                                time.sleep(0.05)
                        if case == "java-thread":
                            with smoke_app.HTTP.open(base + "/ready", timeout=5) as response:
                                assert response.status == 200
                            smoke_app.send_stop_signal(process)
                        else:
                            try:
                                with smoke_app.HTTP.open(base + "/ready", timeout=5) as response:
                                    raise AssertionError(f"Invalid readiness success: {response.status}")
                            except urllib.error.HTTPError as error:
                                assert error.code == 503
                            except (OSError, http.client.RemoteDisconnected):
                                pass  # Fatal exit may close HTTP before the 503 is flushed.
                    assert process.wait(timeout=10) != 0, "Fatal JVM must terminate the app"
                finally:
                    if process.poll() is None:
                        process.kill()
                        process.wait(timeout=5)
            text = log.read_text(encoding="utf-8", errors="replace")
            expected = "JVM startup error" if case == "hang-startup" else "JVM shutdown error" if case == "java-thread" else "JVM fatal error"
            assert expected in text, f"Missing fatal exit evidence: {case}"
            for secret in ("PRIVATE_PATIENT_TOKEN", "password=secret", env.get("VIEWER_DATABASE_PASSWORD")):
                if secret:
                    assert secret not in text, "Sensitive diagnostic leaked"
            assert "WARNING in native method" not in text
            print(f"PASS app fatal recovery: {case}; bounded nonzero exit")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java-home", type=Path, required=True)
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--app", action="store_true", help="Also test HTTP/readiness/shutdown; requires PostgreSQL environment and built viewer-app")
    args = parser.parse_args()
    log = ROOT / "target/step-03-logs/jni-suite.log"
    log.parent.mkdir(parents=True, exist_ok=True)
    command = ["cargo", "test", "-p", "dicom-jvm", "--locked", "--lib"]
    if args.release:
        command.append("--release")
    command += ["jvm_subprocess_suite", "--", "--ignored", "--nocapture"]
    print(f"JNI subprocess suite; log: {log}", flush=True)
    with log.open("w", encoding="utf-8") as output:
        result = subprocess.run(command, cwd=ROOT, env=environment(args.java_home),
                                stdout=output, stderr=subprocess.STDOUT, timeout=600)
    print(log.read_text(encoding="utf-8", errors="replace"))
    if result.returncode:
        sys.exit(result.returncode)
    if args.app:
        import smoke_app
        env = environment(args.java_home)
        command = ["cargo", "test", "-p", "viewer-app", "--locked"]
        if args.release:
            command.append("--release")
        command += ["postgres_and_jvm_readiness", "--", "--ignored", "--nocapture"]
        log = ROOT / "target/step-03-logs/jni-readiness.log"
        with log.open("w", encoding="utf-8") as output:
            result = subprocess.run(command, cwd=ROOT, env=env, stdout=output,
                                    stderr=subprocess.STDOUT, timeout=600)
        print(log.read_text(encoding="utf-8", errors="replace"))
        if result.returncode:
            sys.exit(result.returncode)
        binary = ROOT / "target" / ("release" if args.release else "debug") / (
            "viewer-app.exe" if os.name == "nt" else "viewer-app")
        for profile in ("dev", "prod"):
            smoke_app.check(profile, binary, env)
        # Startup fails before opening HTTP if the configured JVM cannot be loaded.
        bad = env.copy()
        bad["PACS_LIBJVM_PATH"] = str(ROOT / "target/absent-libjvm")
        result = subprocess.run([str(binary)], cwd=ROOT, env=bad,
                                capture_output=True, text=True, timeout=30)
        assert result.returncode != 0
        assert "JVM_CONFIG" in result.stderr
        assert "absent-libjvm" not in result.stderr, "Runtime path leaked"
        print("PASS app JNI startup failure: sanitized error, nonzero exit")
        app_faults(binary, env, args.java_home.resolve())
