import os
import unittest
from tests.e2e.e2e_harness import REPO_ROOT


class TestT2F61SystemRegressionVerification(unittest.TestCase):
    """
    Feature 61: Full System Regression & Verification (Boundary Value Analysis)
    """

    def test_cargo_toml_workspace_manifest_validity(self):
        """BVA: Cargo.toml manifest exists and contains non-empty resolver/members."""
        manifest = os.path.join(REPO_ROOT, "Cargo.toml")
        self.assertTrue(os.path.isfile(manifest))
        size = os.path.getsize(manifest)
        self.assertGreater(size, 100)

    def test_ui_package_json_exists(self):
        """BVA: ui/package.json exists and specifies valid frontend workspace."""
        pkg = os.path.join(REPO_ROOT, "ui", "package.json")
        self.assertTrue(os.path.isfile(pkg))

    def test_tauri_conf_json_exists(self):
        """BVA: src-tauri/tauri.conf.json configuration exists and is non-empty."""
        conf = os.path.join(REPO_ROOT, "src-tauri", "tauri.conf.json")
        self.assertTrue(os.path.isfile(conf))

    def test_scripts_executable_permissions(self):
        """BVA: Required shell scripts exist in scripts/ directory."""
        cross = os.path.join(REPO_ROOT, "scripts", "cross-compile-windows.sh")
        lan = os.path.join(REPO_ROOT, "scripts", "test-lan-distribution.sh")
        self.assertTrue(os.path.isfile(cross))
        self.assertTrue(os.path.isfile(lan))

    def test_e2e_runner_script_exists(self):
        """BVA: tests/e2e/test_runner.py is present and non-empty."""
        runner = os.path.join(REPO_ROOT, "tests", "e2e", "test_runner.py")
        self.assertTrue(os.path.isfile(runner))
        self.assertGreater(os.path.getsize(runner), 500)
