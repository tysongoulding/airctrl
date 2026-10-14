import unittest
from tests.e2e.e2e_harness import WhisperModelVariant


class TestT1F33WhisperParakeetModels(unittest.TestCase):
    """
    Feature 33: Whisper Extended Models & Parakeet V3 (Equivalence Partitions)
    """

    def test_whisper_extended_variants_partition(self):
        """Partition: Extended Whisper models (Small, Medium, Turbo, Large) use .bin GGML."""
        for v in [WhisperModelVariant.SMALL_EN, WhisperModelVariant.MEDIUM_EN,
                  WhisperModelVariant.TURBO_EN, WhisperModelVariant.LARGE_EN]:
            self.assertEqual(v.file_extension, ".bin")
            self.assertGreater(v.parameter_count_millions, 100)

    def test_parakeet_v3_gguf_partition(self):
        """Partition: Parakeet V3 uses .gguf format and CPU optimization."""
        pv3 = WhisperModelVariant.PARAKEET_V3
        self.assertEqual(pv3.file_extension, ".gguf")
        self.assertTrue(pv3.is_cpu_optimized)
        self.assertEqual(pv3.parameter_count_millions, 600)

    def test_auto_language_detection_partition(self):
        """Partition: Auto-language detection supported on Medium, Turbo, Large, and Parakeet."""
        for v in [WhisperModelVariant.MEDIUM_EN, WhisperModelVariant.TURBO_EN,
                  WhisperModelVariant.LARGE_EN, WhisperModelVariant.PARAKEET_V3]:
            self.assertTrue(v.supports_auto_language_detection)

    def test_custom_model_variant_partition(self):
        """Partition: Custom model variant retains custom label and fallback properties."""
        custom = WhisperModelVariant.CUSTOM
        self.assertEqual(custom.value, "custom")
        self.assertFalse(custom.supports_auto_language_detection)

    def test_cpu_optimization_flag_partition(self):
        """Partition: Tiny, Base, Small, and Parakeet are designated as CPU-optimized."""
        for v in [WhisperModelVariant.TINY_EN, WhisperModelVariant.BASE_EN,
                  WhisperModelVariant.SMALL_EN, WhisperModelVariant.PARAKEET_V3]:
            self.assertTrue(v.is_cpu_optimized)
