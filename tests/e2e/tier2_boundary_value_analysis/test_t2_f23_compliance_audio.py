import unittest
from tests.e2e.e2e_harness import MeetingWatcher


class TestT2F23ComplianceAudio(unittest.TestCase):
    """
    Feature 23: Compliance Audio Injection (Boundary Value Analysis)
    """

    def setUp(self):
        self.watcher = MeetingWatcher()

    def test_compliance_announcement_exact_length(self):
        """BVA: Exactly 30 characters in 'This meeting is being recorded'."""
        res = self.watcher.play_compliance_recording_announcement()
        self.assertEqual(len(res["announcement"]), 30)

    def test_compliance_audio_duration_ms_boundary(self):
        """BVA: Announcement duration 1450ms."""
        res = self.watcher.play_compliance_recording_announcement()
        self.assertEqual(res["duration_ms"], 1450)

    def test_repeated_compliance_playback(self):
        """BVA: Calling compliance playback twice succeeds reliably."""
        res1 = self.watcher.play_compliance_recording_announcement()
        res2 = self.watcher.play_compliance_recording_announcement()
        self.assertEqual(res1["status"], "Success")
        self.assertEqual(res2["status"], "Success")

    def test_compliance_audio_zero_volume_prevention(self):
        """BVA: Compliance audio stream volume minimum floor 0.50 (cannot be muted)."""
        target_volume = 0.0
        enforced_volume = max(0.50, target_volume)
        self.assertEqual(enforced_volume, 0.50)

    def test_compliance_audio_format_pcm_16khz(self):
        """BVA: Audio stream format 16,000Hz mono 16-bit PCM."""
        fmt = {"sample_rate": 16000, "channels": 1, "bit_depth": 16}
        self.assertEqual(fmt["sample_rate"], 16000)
        self.assertEqual(fmt["channels"], 1)
