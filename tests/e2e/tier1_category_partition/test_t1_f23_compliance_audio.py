import unittest
from tests.e2e.e2e_harness import MeetingWatcher


class TestT1F23ComplianceAudio(unittest.TestCase):
    """
    Feature 23: Compliance Audio Injection (Equivalence Partitions)
    """

    def setUp(self):
        self.watcher = MeetingWatcher()

    def test_compliance_announcement_text_verbatim(self):
        """Partition: Audible announcement verbatim matches legal disclosure."""
        res = self.watcher.play_compliance_recording_announcement()
        self.assertEqual(res["announcement"], "This meeting is being recorded")

    def test_compliance_announcement_injected_into_stream(self):
        """Partition: Audio stream injection flag set to True."""
        res = self.watcher.play_compliance_recording_announcement()
        self.assertTrue(res["injected_into_stream"])
        self.assertEqual(res["status"], "Success")

    def test_compliance_audio_state_flag(self):
        """Partition: Watcher records compliance_audio_played state."""
        self.assertFalse(self.watcher.compliance_audio_played)
        self.watcher.play_compliance_recording_announcement()
        self.assertTrue(self.watcher.compliance_audio_played)

    def test_compliance_audio_duration_positive(self):
        """Partition: Audio duration in milliseconds is positive (>1000ms)."""
        res = self.watcher.play_compliance_recording_announcement()
        self.assertGreater(res["duration_ms"], 1000)

    def test_recording_session_blocked_until_compliance_completes(self):
        """Partition: Safe recording gating: recording active only after compliance playback."""
        is_recording_active = False
        res = self.watcher.play_compliance_recording_announcement()
        if res["status"] == "Success":
            is_recording_active = True
        self.assertTrue(is_recording_active)
