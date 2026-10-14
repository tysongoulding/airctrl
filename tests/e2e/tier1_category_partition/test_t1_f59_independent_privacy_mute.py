import unittest
from tests.e2e.e2e_harness import VirtualCameraSplitter


class TestT1F59IndependentPrivacyMute(unittest.TestCase):
    """
    Feature 59: Independent Privacy Mute Toggle (Equivalence Partitions)
    """

    def setUp(self):
        self.splitter = VirtualCameraSplitter()

    def test_privacy_mute_replaces_outbound_stream(self):
        """Partition: Privacy mute replaces outbound meeting stream with privacy slate."""
        self.splitter.set_privacy_mute(True)
        out = self.splitter.feed_camera_frame(1)
        self.assertEqual(out["consumer_2_meeting"], "privacy_slate_frame")
        self.assertTrue(out["privacy_muted"])

    def test_privacy_mute_keeps_mediapipe_active(self):
        """Partition: Local hand tracking (Consumer 1) CONTINUES receiving raw frames when meeting is muted."""
        self.splitter.set_privacy_mute(True)
        out = self.splitter.feed_camera_frame(5)
        self.assertEqual(out["consumer_1_mediapipe"], "raw_frame_5")
        self.assertEqual(out["consumer_2_meeting"], "privacy_slate_frame")

    def test_unmute_restores_live_stream(self):
        """Partition: Unmuting restores normal outbound live frames to meeting apps."""
        self.splitter.set_privacy_mute(True)
        self.splitter.set_privacy_mute(False)
        out = self.splitter.feed_camera_frame(10)
        self.assertEqual(out["consumer_2_meeting"], "outbound_frame_10")
        self.assertFalse(out["privacy_muted"])

    def test_explicit_dual_mute_shuts_down_both(self):
        """Partition: Explicit dual mute blocks camera frames to both local gestures and meeting sink."""
        self.splitter.set_dual_mute(True)
        out = self.splitter.feed_camera_frame(20)
        self.assertIsNone(out["consumer_1_mediapipe"])
        self.assertEqual(out["consumer_2_meeting"], "privacy_slate_frame")

    def test_privacy_mute_toggle_state_preservation(self):
        """Partition: Privacy mute state toggles correctly and preserves flag."""
        self.assertFalse(self.splitter.privacy_mute_active)
        self.splitter.set_privacy_mute(True)
        self.assertTrue(self.splitter.privacy_mute_active)
