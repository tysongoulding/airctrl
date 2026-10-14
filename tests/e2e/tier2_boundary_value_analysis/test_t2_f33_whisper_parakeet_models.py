import unittest
from tests.e2e.e2e_harness import WhisperModelVariant


class TestT2F33WhisperParakeetModels(unittest.TestCase):
    """
    Feature 33: Whisper Extended Models & Parakeet V3 (Boundary Value Analysis)
    """

    def test_min_and_max_parameter_boundaries(self):
        """BVA: Min model (Tiny: 39M) vs Max model (Large: 1550M) parameters."""
        self.assertEqual(WhisperModelVariant.TINY_EN.parameter_count_millions, 39)
        self.assertEqual(WhisperModelVariant.LARGE_EN.parameter_count_millions, 1550)

    def test_parakeet_v3_gguf_boundary_checks(self):
        """BVA: Parakeet V3 boundary - exactly 600M params and gguf format."""
        pv3 = WhisperModelVariant.PARAKEET_V3
        self.assertEqual(pv3.parameter_count_millions, 600)
        self.assertTrue(pv3.file_extension.startswith(".g"))

    def test_unknown_custom_model_boundary(self):
        """BVA: Custom model variant defaults gracefully."""
        c = WhisperModelVariant.CUSTOM
        self.assertEqual(c.parameter_count_millions, 100)
        self.assertFalse(c.is_cpu_optimized)

    def test_non_cpu_optimized_boundary(self):
        """BVA: Large and Medium models require server/GPU acceleration."""
        self.assertFalse(WhisperModelVariant.LARGE_EN.is_cpu_optimized)
        self.assertFalse(WhisperModelVariant.MEDIUM_EN.is_cpu_optimized)

    def test_string_conversion_roundtrip(self):
        """BVA: Enum string value matches expected identifier."""
        self.assertEqual(str(WhisperModelVariant.TURBO_EN.value), "whisper-turbo.en")
