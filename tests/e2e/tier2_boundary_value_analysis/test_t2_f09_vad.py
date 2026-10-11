import unittest
from tests.e2e.e2e_harness import VoiceActivityDetector


class TestT2F09Vad(unittest.TestCase):
    """
    Feature 9: Voice Activity Detection (VAD) (Boundary Value Analysis)
    """

    def setUp(self):
        self.vad = VoiceActivityDetector(energy_threshold=0.05, min_samples=1600)

    def test_samples_boundary_1599_rejected(self):
        """BVA: Exactly 1599 samples (< 1600) rejected even with high energy."""
        samples = [1.0] * 1599
        self.assertFalse(self.vad.is_speech(samples))

    def test_samples_boundary_1600_accepted(self):
        """BVA: Exactly 1600 samples (>= 1600) with high energy accepted."""
        samples = [1.0] * 1600
        self.assertTrue(self.vad.is_speech(samples))

    def test_energy_boundary_just_below_threshold(self):
        """BVA: RMS energy 0.049 (< 0.05 threshold) classified as silence."""
        samples = [0.049] * 1600
        self.assertFalse(self.vad.is_speech(samples))

    def test_energy_boundary_at_threshold(self):
        """BVA: RMS energy 0.050 (>= 0.05 threshold) classified as speech."""
        samples = [0.050] * 1600
        self.assertTrue(self.vad.is_speech(samples))

    def test_energy_boundary_just_above_threshold(self):
        """BVA: RMS energy 0.051 (> 0.05 threshold) classified as speech."""
        samples = [0.051] * 1600
        self.assertTrue(self.vad.is_speech(samples))
