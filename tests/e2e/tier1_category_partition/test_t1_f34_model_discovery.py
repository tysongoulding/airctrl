import os
import tempfile
import unittest
from tests.e2e.e2e_harness import ModelDiscoveryService, WhisperModelVariant


class TestT1F34ModelDiscovery(unittest.TestCase):
    """
    Feature 34: Multi-Directory Model Discovery (Equivalence Partitions)
    """

    def setUp(self):
        self.temp_dir = tempfile.mkdtemp(prefix="airctrl_models_")

    def tearDown(self):
        import shutil
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_discover_in_empty_directory(self):
        """Partition: Empty directory returns empty list."""
        res = ModelDiscoveryService.discover_models([self.temp_dir])
        self.assertEqual(len(res), 0)

    def test_discover_whisper_bin_models(self):
        """Partition: Discovers valid .bin Whisper models."""
        bin_path = os.path.join(self.temp_dir, "ggml-whisper-medium.en.bin")
        with open(bin_path, "wb") as f:
            f.write(b"\x00" * 2048)

        res = ModelDiscoveryService.discover_models([self.temp_dir])
        self.assertEqual(len(res), 1)
        self.assertEqual(res[0].format, "bin")
        self.assertEqual(res[0].variant, WhisperModelVariant.MEDIUM_EN)
        self.assertTrue(res[0].is_valid)

    def test_discover_parakeet_gguf_models(self):
        """Partition: Discovers valid .gguf Parakeet models."""
        gguf_path = os.path.join(self.temp_dir, "parakeet-unified-en-0.6b-q8_0.gguf")
        with open(gguf_path, "wb") as f:
            f.write(b"\x00" * 4096)

        res = ModelDiscoveryService.discover_models([self.temp_dir])
        self.assertEqual(len(res), 1)
        self.assertEqual(res[0].format, "gguf")
        self.assertEqual(res[0].variant, WhisperModelVariant.PARAKEET_V3)

    def test_deduplicate_identical_model_paths(self):
        """Partition: Deduplicates models found in overlapping search directories."""
        model_path = os.path.join(self.temp_dir, "whisper-small.en.bin")
        with open(model_path, "wb") as f:
            f.write(b"\x00" * 2048)

        # Pass same directory twice
        res = ModelDiscoveryService.discover_models([self.temp_dir, self.temp_dir])
        self.assertEqual(len(res), 1)

    def test_filter_invalid_non_model_files(self):
        """Partition: Filters out non-model extensions (.txt, .json) and undersized files."""
        txt_path = os.path.join(self.temp_dir, "notes.txt")
        with open(txt_path, "w") as f:
            f.write("not a model")

        small_bin = os.path.join(self.temp_dir, "corrupt.bin")
        with open(small_bin, "wb") as f:
            f.write(b"\x00" * 10)  # less than 1024 bytes -> invalid

        res = ModelDiscoveryService.discover_models([self.temp_dir])
        # Only corrupt.bin detected but marked invalid, notes.txt completely ignored
        self.assertEqual(len(res), 1)
        self.assertFalse(res[0].is_valid)
