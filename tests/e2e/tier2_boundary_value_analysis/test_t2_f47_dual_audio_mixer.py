import unittest
from tests.e2e.e2e_harness import DualAudioMixer


class TestT2F47DualAudioMixer(unittest.TestCase):
    """
    Feature 47: Dual Mic + System Loopback Mixer (Boundary Value Analysis)
    """

    def setUp(self):
        self.mixer = DualAudioMixer()

    def test_single_sample_buffer_boundary(self):
        """BVA: 1-sample buffer processed without underflow."""
        mixed = self.mixer.mix([0.5], [0.5])
        self.assertEqual(len(mixed), 1)
        self.assertTrue(-1.0 <= mixed[0] <= 1.0)

    def test_large_buffer_160k_samples_boundary(self):
        """BVA: 160,000 samples (10 seconds @ 16kHz) processed continuously."""
        mic = [0.3] * 160000
        lp = [0.2] * 160000
        mixed = self.mixer.mix(mic, lp)
        self.assertEqual(len(mixed), 160000)

    def test_all_maximum_positive_amplitude_boundary(self):
        """BVA: 1.0 on both mic and loopback saturates safely without clipping."""
        mixed = self.mixer.mix([1.0] * 100, [1.0] * 100)
        for s in mixed:
            self.assertTrue(0.0 <= s <= 1.0)

    def test_all_maximum_negative_amplitude_boundary(self):
        """BVA: -1.0 on both channels saturates safely without negative underflow."""
        mixed = self.mixer.mix([-1.0] * 100, [-1.0] * 100)
        for s in mixed:
            self.assertTrue(-1.0 <= s <= 0.0)

    def test_alternating_sample_polarity_boundary(self):
        """BVA: High frequency alternating polarity [+1.0, -1.0] mixes cleanly."""
        mic = [1.0, -1.0] * 50
        lp = [-1.0, 1.0] * 50
        mixed = self.mixer.mix(mic, lp)
        self.assertEqual(len(mixed), 100)
