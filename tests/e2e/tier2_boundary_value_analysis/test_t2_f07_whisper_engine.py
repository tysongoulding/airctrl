import unittest
from tests.e2e.e2e_harness import MockWhisperEngine


class TestT2F07WhisperEngine(unittest.TestCase):
    """
    Feature 7: Offline Whisper STT Engine (Boundary Value Analysis)
    """

    def setUp(self):
        self.engine = MockWhisperEngine()

    def test_single_sample_audio_boundary(self):
        """BVA: Exactly 1 audio sample processed without index error."""
        res = self.engine.transcribe([0.05])
        self.assertEqual(res["sample_count"], 1)

    def test_minimum_vad_chunk_1600_samples(self):
        """BVA: Exactly 1600 samples (100ms at 16kHz) processed."""
        samples = [0.1] * 1600
        res = self.engine.transcribe(samples)
        self.assertEqual(res["sample_count"], 1600)

    def test_zero_energy_silence_confidence_floor(self):
        """BVA: Zero energy audio clamped to confidence floor 0.40."""
        samples = [0.0] * 1000
        res = self.engine.transcribe(samples)
        self.assertGreaterEqual(res["confidence"], 0.40)

    def test_high_energy_audio_confidence_ceiling(self):
        """BVA: High energy audio clamped to confidence ceiling 0.99."""
        samples = [1.0] * 1000
        res = self.engine.transcribe(samples)
        self.assertLessEqual(res["confidence"], 0.99)

    def test_large_audio_buffer_320000_samples(self):
        """BVA: 20-second audio buffer (320,000 samples) processed within memory limits."""
        samples = [0.01] * 320000
        res = self.engine.transcribe(samples)
        self.assertEqual(res["sample_count"], 320000)
