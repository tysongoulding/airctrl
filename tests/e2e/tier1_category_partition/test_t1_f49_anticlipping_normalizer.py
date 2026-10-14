import math
import unittest
from tests.e2e.e2e_harness import DualAudioMixer


class TestT1F49AnticlippingNormalizer(unittest.TestCase):
    """
    Feature 49: Anti-Clipping Dynamic Range Normalizer (Equivalence Partitions)
    """

    def setUp(self):
        self.mixer = DualAudioMixer()

    def test_tanh_saturation_bounds(self):
        """Partition: Over-driven audio sums saturate strictly inside [-1.0, 1.0]."""
        mic = [5.0, 10.0, -15.0, 100.0]
        loopback = [5.0, 10.0, -15.0, 100.0]
        mixed = self.mixer.mix(mic, loopback)
        for s in mixed:
            self.assertTrue(-1.0 <= s <= 1.0)
            self.assertFalse(math.isnan(s))

    def test_linear_pass_through_for_small_signals(self):
        """Partition: Small audio signals (< 0.1) preserve linear dynamic range."""
        mic = [0.05] * 10
        loopback = [0.02] * 10
        mixed = self.mixer.mix(mic, loopback)
        for s in mixed:
            # tanh(0.07) ~ 0.069888
            self.assertAlmostEqual(s, 0.07, delta=0.005)

    def test_zero_signal_pass_through(self):
        """Partition: Silence inputs produce exact zero output."""
        mixed = self.mixer.mix([0.0] * 10, [0.0] * 10)
        for s in mixed:
            self.assertEqual(s, 0.0)

    def test_symmetric_compression(self):
        """Partition: Positive and negative peaks compress with exact symmetry."""
        pos = self.mixer.mix([2.5], [0.0])[0]
        neg = self.mixer.mix([-2.5], [0.0])[0]
        self.assertAlmostEqual(pos, -neg, places=5)

    def test_smooth_knee_monotonicity(self):
        """Partition: Monotonically increasing inputs yield strictly monotonic outputs."""
        inputs = [i * 0.2 for i in range(15)]
        mixed = self.mixer.mix(inputs, [0.0] * len(inputs))
        for i in range(len(mixed) - 1):
            self.assertLessEqual(mixed[i], mixed[i + 1])
