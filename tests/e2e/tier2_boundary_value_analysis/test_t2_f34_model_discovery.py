import os
import shutil
import tempfile
import unittest
from tests.e2e.e2e_harness import ModelDiscoveryService


class TestT2F34ModelDiscovery(unittest.TestCase):
    """
    Feature 34: Multi-Directory Model Discovery (Boundary Value Analysis)
    """

    def setUp(self):
        self.temp_dir = tempfile.mkdtemp(prefix="airctrl_bva_models_")

    def tearDown(self):
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_file_size_boundary_1023_vs_1024_bytes(self):
        """BVA: Exactly 1023 bytes (invalid) vs 1024 bytes (valid minimal model file)."""
        under_path = os.path.join(self.temp_dir, "under_whisper-tiny.en.bin")
        valid_path = os.path.join(self.temp_dir, "valid_whisper-tiny.en.bin")

        with open(under_path, "wb") as f:
            f.write(b"\x00" * 1023)
        with open(valid_path, "wb") as f:
            f.write(b"\x00" * 1024)

        models = ModelDiscoveryService.discover_models([self.temp_dir])
        by_name = {m.name: m for m in models}
        self.assertFalse(by_name["under_whisper-tiny.en.bin"].is_valid)
        self.assertTrue(by_name["valid_whisper-tiny.en.bin"].is_valid)

    def test_deep_subfolder_traversal_boundary(self):
        """BVA: 5-level nested subdirectory model file discovery."""
        nested = os.path.join(self.temp_dir, "level1", "level2", "level3", "level4", "level5")
        os.makedirs(nested, exist_ok=True)
        deep_model = os.path.join(nested, "whisper-medium.en.bin")
        with open(deep_model, "wb") as f:
            f.write(b"\x00" * 2048)

        models = ModelDiscoveryService.discover_models([self.temp_dir])
        self.assertEqual(len(models), 1)
        self.assertEqual(models[0].name, "whisper-medium.en.bin")

    def test_duplicate_file_in_different_directories(self):
        """BVA: Model files with identical filenames in two directories both tracked if paths differ."""
        dir2 = tempfile.mkdtemp(prefix="airctrl_bva_models2_")
        try:
            p1 = os.path.join(self.temp_dir, "model.gguf")
            p2 = os.path.join(dir2, "model.gguf")
            with open(p1, "wb") as f: f.write(b"\x00" * 2048)
            with open(p2, "wb") as f: f.write(b"\x00" * 2048)

            models = ModelDiscoveryService.discover_models([self.temp_dir, dir2])
            self.assertEqual(len(models), 2)
        finally:
            shutil.rmtree(dir2, ignore_errors=True)

    def test_uppercase_extensions_boundary(self):
        """BVA: Uppercase extensions (.BIN, .GGUF) discovered correctly."""
        p = os.path.join(self.temp_dir, "whisper-turbo.en.BIN")
        with open(p, "wb") as f:
            f.write(b"\x00" * 2048)

        models = ModelDiscoveryService.discover_models([self.temp_dir])
        self.assertEqual(len(models), 1)
        self.assertEqual(models[0].format, "bin")

    def test_nonexistent_search_path_boundary(self):
        """BVA: Passing non-existent directories completes without error."""
        models = ModelDiscoveryService.discover_models(["/nonexistent/path/for/airctrl/tests"])
        self.assertEqual(len(models), 0)
