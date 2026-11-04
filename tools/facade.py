"""Build/verify the Java facade with an explicit JDK and packaged-only classpath."""

import argparse
import hashlib
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
import zipfile

ROOT = Path(__file__).resolve().parents[1]
MODULE = ROOT / "java" / "dicom-facade"
DIST = MODULE / "target" / "distribution"
LOGS = ROOT / "target" / "step-03-logs"
EXPECTED = {"apiVersion": 2, "operations": ["downloadMetadata", "downloadSeries", "encodeVideo"], "codecs": []}


def run(command, env, log, timeout=120):
    print(f"Running: {command[0]} (log: {log})", flush=True)
    with log.open("w", encoding="utf-8") as output:
        result = subprocess.run(
            [str(arg) for arg in command], cwd=ROOT, env=env,
            stdout=output, stderr=subprocess.STDOUT, timeout=timeout, check=False,
        )
    content = log.read_text(encoding="utf-8", errors="replace")
    if result.returncode:
        raise RuntimeError(f"Command failed ({result.returncode}); see {log}\n{content[-4000:]}")
    return content


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact_hashes():
    return {path.relative_to(DIST).as_posix(): sha256(path)
            for path in sorted(DIST.rglob("*.jar"))}


def smoke(java_home, env):
    jar = DIST / "pacs-dicom-facade.jar"
    if not jar.is_file() or not (DIST / "dependencies").is_dir():
        raise RuntimeError("Missing facade JAR/dependencies directory; run build first")
    for artifact in [jar, *sorted((DIST / "dependencies").glob("*.jar"))]:
        if "orthanc" in artifact.name.lower():
            raise RuntimeError("Orthanc must not be packaged in the Rust facade distribution")
        with zipfile.ZipFile(artifact) as archive:
            if any(name.startswith("org/orthanc/") or "orthanc_transcode" in name.lower() for name in archive.namelist()):
                raise RuntimeError("Orthanc classes/native library found in the facade distribution")
    with zipfile.ZipFile(jar) as archive:
        classes = {name for name in archive.namelist() if name.endswith(".class")}
        if classes != {"vn/saolasoft/pacs/dicom/DicomFacade.class"}:
            raise RuntimeError("Unexpected classes in facade JAR")
        manifest = archive.read("META-INF/MANIFEST.MF").decode("utf-8")
        if "Facade-Api-Version: 2" not in manifest.splitlines():
            raise RuntimeError("JAR manifest API version mismatch")
    classpath = os.pathsep.join(str(p) for p in [jar, *sorted((DIST / "dependencies").glob("*.jar"))])
    suffix = ".exe" if os.name == "nt" else ""
    with tempfile.TemporaryDirectory(prefix="facade-smoke-", dir=MODULE / "target") as temp:
        run([java_home / "bin" / f"javac{suffix}", "--release", "25", "-cp", classpath,
             "-d", temp, MODULE / "src" / "smoke" / "java" / "FacadeSmoke.java"],
            env, LOGS / "compile-facade-smoke.log")
        # Only the probe and packaged JARs, never Maven's target/classes.
        output = run([java_home / "bin" / f"java{suffix}", "-Xcheck:jni", "-cp",
                      os.pathsep.join([temp, classpath]), "FacadeSmoke", jar],
                     env, LOGS / "smoke-facade.log", timeout=30)
    lines = output.splitlines()
    if len(lines) != 5 or json.loads(lines[0]) != EXPECTED:
        raise RuntimeError("Unexpected capabilities or smoke output; see smoke-facade.log")
    return dict(zip(("vendor", "version", "os", "arch"), lines[1:]))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("build", "verify"))
    parser.add_argument("--java-home", type=Path, required=True, help="Explicit Java 25 JDK directory")
    parser.add_argument("--maven-repo", type=Path, help="Override Maven's local dependency cache")
    parser.add_argument("--source-revision", help="Full Git commit for builds without .git")
    parser.add_argument("--source-dirty", choices=("true", "false"), help="Working-tree state paired with --source-revision")
    args = parser.parse_args()
    if (args.source_revision is None) != (args.source_dirty is None):
        parser.error("--source-revision and --source-dirty must be supplied together")
    if args.source_revision is not None and not re.fullmatch(r"[0-9a-fA-F]{40}|[0-9a-fA-F]{64}", args.source_revision):
        parser.error("--source-revision must be a full Git commit hash")
    java_home = args.java_home.resolve()
    suffix = ".exe" if os.name == "nt" else ""
    for name in ("java", "javac"):
        if not (java_home / "bin" / f"{name}{suffix}").is_file():
            raise RuntimeError(f"Missing {name} in the supplied JDK")
    env = os.environ.copy()
    env["JAVA_HOME"] = str(java_home)
    env["PATH"] = str(java_home / "bin") + os.pathsep + env.get("PATH", "")
    # Keep ambient Java launcher/classpath settings out of this controlled smoke.
    for name in ("CLASSPATH", "JAVA_TOOL_OPTIONS", "JDK_JAVA_OPTIONS", "_JAVA_OPTIONS"):
        env.pop(name, None)
    LOGS.mkdir(parents=True, exist_ok=True)
    manifest_path = DIST / "manifest.json"
    if args.action == "build":
        maven = shutil.which("mvn.cmd" if os.name == "nt" else "mvn")
        if not maven:
            raise RuntimeError("Maven is required on PATH")
        run([maven, "-version"], env, LOGS / "facade-toolchain.log")
        repository = [f"-Dmaven.repo.local={args.maven_repo.resolve()}"] if args.maven_repo else []
        run([maven, *repository, "--batch-mode", "--no-transfer-progress", "-f", MODULE / "pom.xml",
             "clean", "package"], env, LOGS / "build-facade.log", timeout=600)
        # Maven may omit this directory when there are zero runtime dependencies.
        (DIST / "dependencies").mkdir(exist_ok=True)
        runtime = smoke(java_home, env)
        version = ET.parse(MODULE / "pom.xml").getroot().findtext("{http://maven.apache.org/POM/4.0.0}version")
        if args.source_revision is not None:
            revision = args.source_revision.lower()
            dirty = args.source_dirty == "true"
        else:
            revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
            dirty = bool(subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).strip())
        manifest = {"schemaVersion": 1, "facadeVersion": version, "apiVersion": 2,
                    "capabilities": EXPECTED, "source": {"gitCommit": revision, "dirty": dirty},
                    "testedRuntime": runtime, "dcm4cheVersion": "5.33.1",
                    "artifacts": artifact_hashes()}
        manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    else:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        if manifest["schemaVersion"] != 1 or manifest["apiVersion"] != 2 or manifest["capabilities"] != EXPECTED:
            raise RuntimeError("Distribution manifest contract mismatch")
        if manifest["artifacts"] != artifact_hashes():
            raise RuntimeError("Distribution JAR checksum/inventory mismatch")
        runtime = smoke(java_home, env)
    print(f"PASS: packaged facade API 2, download operations; native codecs require corpus verification; runtime {runtime}")
    print(f"Distribution: {DIST}")


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, ValueError, KeyError, subprocess.SubprocessError, zipfile.BadZipFile) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        sys.exit(1)
