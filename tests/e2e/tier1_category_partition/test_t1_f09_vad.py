import unittest
from tests.e2e.e2e_harness import VoiceActivityDetector


class TestT1F09Vad(unittest.TestCase):
    """
    Feature 9: Voice Activity Detection (VAD) (Equivalence Partitions)
    """

    def setUp(self):
        self.vad = VoiceActivityDetector(energy_threshold=0.05, min_samples=1600)

    def test_clear_speech_partition_is_detected(self):
        """Partition: High RMS energy signals detected as active speech."""
        high_energy_speech = [0.3 * ((i % 5) - 2) for i in range(2000)]
        self.assertTrue(self.vad.is_speech(high_energy_speech))

    def test_silence_partition_is_filtered(self):
        """Partition: Near-zero RMS energy signals filtered as silence."""
        silence = [0.001 * (i % 2) for i in range(2000)]
        self.assertFalse(self.vad.is_speech(silence))

    def test_sub_100ms_accidental_press_filtered(self):
        """Partition: Audio buffer < min_samples (100ms / 1600 samples) rejected."""
        short_burst = [0.5 for _ in range(800)]
        self.assertFalse(self.vad.is_speech(short_burst))

    def test_rms_energy_calculation(self):
        """Partition: Accurate root-mean-square energy calculation."""
        constant_signal = [0.2 for _ in range(1000)]
        rms = self.vad.calculate_rms_energy(constant_signal)
        self.assertAlmostEqual(rms, 0.2, places=4)

    def test_empty_buffer_energy_zero(self):
        """Partition: Empty buffer yields 0.0 RMS energy."""
        self.assertEqual(self.vad.calculate_rms_energy([]), 0.0)
        self.assertFalse(self.vad.is_speech([]))
