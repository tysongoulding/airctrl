import unittest
from tests.e2e.e2e_harness import LiveMeetingTranscriber


class TestT1F50LiveMeetingTranscription(unittest.TestCase):
    """
    Feature 50: Live Streaming Meeting Transcription (Equivalence Partitions)
    """

    def setUp(self):
        self.transcriber = LiveMeetingTranscriber()

    def test_host_speaker_attribution(self):
        """Partition: Mic dominant energy ratio attributes chunk to Host."""
        chunk = self.transcriber.feed_audio(mic_energy=0.8, loopback_energy=0.05, duration_ms=1000)
        self.assertEqual(chunk.speaker, "Host")

    def test_participant_speaker_attribution(self):
        """Partition: Loopback dominant energy ratio attributes chunk to Participant."""
        chunk = self.transcriber.feed_audio(mic_energy=0.02, loopback_energy=0.7, duration_ms=1000)
        self.assertEqual(chunk.speaker, "Participant")

    def test_mixed_speaker_attribution(self):
        """Partition: Simultaneous speech with balanced energy attributes chunk to Mixed."""
        chunk = self.transcriber.feed_audio(mic_energy=0.5, loopback_energy=0.5, duration_ms=1000)
        self.assertEqual(chunk.speaker, "Mixed")

    def test_chunk_time_progression(self):
        """Partition: Timestamp offsets advance monotonically with duration."""
        c1 = self.transcriber.feed_audio(0.5, 0.1, duration_ms=1200)
        self.assertEqual(c1.start_time_ms, 0)
        self.assertEqual(c1.end_time_ms, 1200)

        c2 = self.transcriber.feed_audio(0.1, 0.5, duration_ms=800)
        self.assertEqual(c2.start_time_ms, 1200)
        self.assertEqual(c2.end_time_ms, 2000)

    def test_high_confidence_transcription(self):
        """Partition: Streaming chunks maintain confidence metrics >= 0.80."""
        chunk = self.transcriber.feed_audio(0.6, 0.2, duration_ms=1000)
        self.assertGreaterEqual(chunk.confidence, 0.80)
