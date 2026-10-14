import unittest
from tests.e2e.e2e_harness import LiveMeetingTranscriber


class TestT2F50LiveMeetingTranscription(unittest.TestCase):
    """
    Feature 50: Live Streaming Meeting Transcription (Boundary Value Analysis)
    """

    def setUp(self):
        self.transcriber = LiveMeetingTranscriber()

    def test_equal_energy_ratio_boundary(self):
        """BVA: Exactly equal energy ratio (1.0) maps to 'Mixed' speaker."""
        c = self.transcriber.feed_audio(mic_energy=0.5, loopback_energy=0.5)
        self.assertEqual(c.speaker, "Mixed")

    def test_exact_2x_ratio_boundary_host(self):
        """BVA: Mic energy > 2.0x loopback attributes to Host."""
        c = self.transcriber.feed_audio(mic_energy=0.201, loopback_energy=0.10)
        self.assertEqual(c.speaker, "Host")

    def test_exact_2x_ratio_boundary_participant(self):
        """BVA: Loopback energy > 2.0x mic attributes to Participant."""
        c = self.transcriber.feed_audio(mic_energy=0.10, loopback_energy=0.201)
        self.assertEqual(c.speaker, "Participant")

    def test_zero_energy_silence_boundary(self):
        """BVA: Zero energy on both mic and loopback handles division safely."""
        c = self.transcriber.feed_audio(mic_energy=0.0, loopback_energy=0.0)
        self.assertEqual(c.speaker, "Mixed")

    def test_large_meeting_time_offset_accumulation(self):
        """BVA: Cumulative duration offset exceeding 36,000,000ms (10 hours) advances correctly."""
        self.transcriber.current_time_ms = 36000000
        c = self.transcriber.feed_audio(0.5, 0.1, duration_ms=2000)
        self.assertEqual(c.start_time_ms, 36000000)
        self.assertEqual(c.end_time_ms, 36002000)
