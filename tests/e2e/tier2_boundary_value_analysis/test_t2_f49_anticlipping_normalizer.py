import math
import random
import unittest
from tests.e2e.e2e_harness import DualAudioMixer


class TestT2F49AnticlippingNormalizer(unittest.TestCase):
    """
    Feature 49: Anti-Clipping Dynamic Range Normalizer (Boundary Value Analysis)
    """

    def setUp(self):
        self.mixer = DualAudioMixer()

    def test_random_samples_strict_invariant_in_minus1_to_plus1(self):
        """BVA: 10,000 random samples in [-100.0, 100.0] all bound strictly in [-1.0, 1.0]."""
        random.seed(42)
        mic = [random.uniform(-100.0, 100.0) for _ in range(10000)]
        lp = [random.uniform(-100.0, 100.0) for _ in range(10000)]
        mixed = self.mixer.mix(mic, lp)
        for s in mixed:
            self.assertTrue(-1.0 <= s <= 1.0)
            self.assertFalse(math.isnan(s))

    def test_subnormal_floating_point_numbers(self):
        """BVA: Subnormal numbers (1e-35) processed without math domain error."""
        mixed = self.mixer.mix([1e-35] * 10, [1e-35] * 10)
        self.assertEqual(len(mixed), 10)
        for s in mixed:
            self.assertFalse(math.isnan(s))

    def test_extreme_million_level_amplitude_saturation(self):
        """BVA: Extreme peak inputs (1,000,000.0) saturate cleanly to +/-1.0."""
        mixed = self.mixer.mix([1e6, -1e6], [0.0, 0.0])
        self.assertEqual(mixed[0], 1.0)
        self.assertEqual(mixed[1], -1.0)

    def test_near_zero_transitions(self):
        """BVA: Infinitesimal transitions around zero (-1e-7 to +1e-7)."""
        mixed = self.mixer.mix([-1e-7, 1e-7], [0.0, 0.0])
        self.assertAlmostEqual(mixed[0], -1e-7, delta=1e-8)
        self.assertAlmostEqual(mixed[1], 1e-7, delta=1e-8)

    def test_no_nan_or_inf_generated(self):
        """BVA: Mixed audio stream contains zero NaN or Infinity values."""
        mixed = self.mixer.mix([0.0, -0.0, 1.0, -1.0], [0.5, -0.5, 0.5, -0.5])
        for s in mixed:
            self.assertFalse(math.isnan(s))
            self.assertFalse(math.isinf(s))
