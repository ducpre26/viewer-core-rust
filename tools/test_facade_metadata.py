"""Test build provenance without Maven/JVM; does not certify facade packaging."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("facade", Path(__file__).with_name("facade.py"))
facade = importlib.util.module_from_spec(spec)
spec.loader.exec_module(facade)


class SourceMetadataTests(unittest.TestCase):
    def build_manifest(self, extra, git_outputs=None):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            java = root / "java"
            (java / "bin").mkdir(parents=True)
            suffix = ".exe" if sys.platform == "win32" else ""
            for executable in ("java", "javac"):
                (java / "bin" / (executable + suffix)).touch()
            module = root / "module"
            dist = module / "target" / "distribution"
            dist.mkdir(parents=True)
            (module / "pom.xml").write_text(
                '<project xmlns="http://maven.apache.org/POM/4.0.0"><version>0.1.0</version></project>',
                encoding="utf-8",
            )
            with contextlib.ExitStack() as stack:
                for name, value in (("ROOT", root), ("MODULE", module), ("DIST", dist), ("LOGS", root / "logs")):
                    stack.enter_context(patch.object(facade, name, value))
                stack.enter_context(patch.object(sys, "argv", ["facade.py", "build", "--java-home", str(java), *extra]))
                stack.enter_context(patch.object(facade.shutil, "which", return_value="mvn"))
                stack.enter_context(patch.object(facade, "run"))
                stack.enter_context(patch.object(facade, "smoke", return_value={"version": "25"}))
                stack.enter_context(patch.object(facade, "artifact_hashes", return_value={}))
                git = stack.enter_context(patch.object(facade.subprocess, "check_output", side_effect=git_outputs or AssertionError("Git must not be called")))
                stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
                facade.main()
                return json.loads((dist / "manifest.json").read_text()), git.call_count

    def test_explicit_metadata_does_not_need_git(self):
        for dirty in ("true", "false"):
            with self.subTest(dirty=dirty):
                manifest, calls = self.build_manifest(["--source-revision", "A" * 40, "--source-dirty", dirty])
                self.assertEqual(manifest["source"], {"gitCommit": "a" * 40, "dirty": dirty == "true"})
                self.assertEqual(calls, 0)

    def test_local_build_retains_git_fallback(self):
        manifest, calls = self.build_manifest([], ["b" * 40 + "\n", " M tools/facade.py\n"])
        self.assertEqual(manifest["source"], {"gitCommit": "b" * 40, "dirty": True})
        self.assertEqual(calls, 2)

    def test_invalid_or_partial_metadata_fails_before_build(self):
        for extra in (["--source-revision", "a" * 40], ["--source-dirty", "false"],
                      ["--source-revision", "HEAD", "--source-dirty", "false"]):
            with self.subTest(extra=extra), contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit) as caught:
                    self.build_manifest(extra)
                self.assertEqual(caught.exception.code, 2)


if __name__ == "__main__":
    unittest.main()
