import unittest
from tests.e2e.e2e_harness import DualAudioMixer


class TestT1F47DualAudioMixer(unittest.TestCase):
    """
    Feature 47: Dual Mic + System Loopback Mixer (Equivalence Partitions)
    """

    def setUp(self):
        self.mixer = DualAudioMixer(sample_rate=16000)

    def test_mic_only_mixing_partition(self):
        """Partition: Mic input with silent system audio outputs mic signal."""
        mic = [0.2] * 100
        loopback = [0.0] * 100
        mixed = self.mixer.mix(mic, loopback)
        self.assertEqual(len(mixed), 100)
        self.assertAlmostEqual(mixed[0], 0.197, delta=0.05)

    def test_loopback_only_mixing_partition(self):
        """Partition: System loopback audio with silent mic passes through un-ducked."""
        mic = [0.0] * 100
        loopback = [0.3] * 100
        mixed = self.mixer.mix(mic, loopback)
        self.assertEqual(len(mixed), 100)
        self.assertAlmostEqual(mixed[0], 0.291, delta=0.05)

    def test_simultaneous_dual_stream_partition(self):
        """Partition: Simultaneous dual streams mixed correctly."""
        mic = [0.4] * 50
        loopback = [0.4] * 50
        mixed = self.mixer.mix(mic, loopback)
        self.assertEqual(len(mixed), 50)
        for s in mixed:
            self.assertTrue(-1.0 <= s <= 1.0)

    def test_mismatched_buffer_lengths_partition(self):
        """Partition: Mismatched buffer lengths padded to longest stream without index error."""
        mic = [0.1] * 200
        loopback = [0.2] * 100
        mixed = self.mixer.mix(mic, loopback)
        self.assertEqual(len(mixed), 200)

    def test_empty_audio_buffers_partition(self):
        """Partition: Empty buffers return empty list."""
        mixed = self.mixer.mix([], [])
        self.assertEqual(mixed, [])
