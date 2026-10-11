import unittest
from tests.e2e.e2e_harness import MockWhisperEngine


class TestT1F07WhisperEngine(unittest.TestCase):
    """
    Feature 7: Offline Whisper STT Engine (Equivalence Partitions)
    """

    def setUp(self):
        self.engine = MockWhisperEngine()

    def test_offline_initialization_with_local_weights(self):
        """Partition: Engine loads offline model weights."""
        self.assertTrue(self.engine.is_loaded)
        self.assertIn("whisper-tiny.en.bin", self.engine.model_path)

    def test_transcribe_valid_audio_samples(self):
        """Partition: Valid PCM samples transcribe to text."""
        samples = [0.1 * ((i % 10) - 5) for i in range(16000)]
        res = self.engine.transcribe(samples)
        self.assertIn("text", res)
        self.assertIn("airctrl", res["text"])
        self.assertGreater(res["confidence"], 0.0)

    def test_transcription_latency_sub_second(self):
        """Partition: Programmatic validation of sub-second inference latency."""
        samples = [0.05 for _ in range(8000)]
        res = self.engine.transcribe(samples)
        self.assertLess(res["duration_ms"], 1000)

    def test_transcribe_empty_buffer_error(self):
        """Partition: Empty buffer raises error."""
        with self.assertRaises(ValueError):
            self.engine.transcribe([])

    def test_unloaded_model_raises_runtime_error(self):
        """Partition: Attempting transcription without weights loaded raises error."""
        self.engine.is_loaded = False
        with self.assertRaises(RuntimeError):
            self.engine.transcribe([0.1, 0.2])
