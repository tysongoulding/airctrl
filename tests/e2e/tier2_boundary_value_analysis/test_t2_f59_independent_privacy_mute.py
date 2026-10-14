import unittest
from tests.e2e.e2e_harness import VirtualCameraSplitter


class TestT2F59IndependentPrivacyMute(unittest.TestCase):
    """
    Feature 59: Independent Privacy Mute Toggle (Boundary Value Analysis)
    """

    def setUp(self):
        self.splitter = VirtualCameraSplitter()

    def test_rapid_privacy_mute_toggle_cycling(self):
        """BVA: 10 rapid alternating mute toggles maintain correct final state."""
        for i in range(10):
            self.splitter.set_privacy_mute(i % 2 == 0)
        # Last iteration i=9 (9 % 2 == 1 -> False)
        self.assertFalse(self.splitter.privacy_mute_active)

    def test_privacy_slate_frame_constant_invariant(self):
        """BVA: During privacy mute, Consumer 2 always outputs 'privacy_slate_frame'."""
        self.splitter.set_privacy_mute(True)
        for f in range(20):
            res = self.splitter.feed_camera_frame(f)
            self.assertEqual(res["consumer_2_meeting"], "privacy_slate_frame")

    def test_consumer_1_uninterrupted_across_50_privacy_muted_frames(self):
        """BVA: Consumer 1 raw frame ingestion is 100% uninterrupted across 50 frames during privacy mute."""
        self.splitter.set_privacy_mute(True)
        for f in range(50):
            res = self.splitter.feed_camera_frame(f)
            self.assertEqual(res["consumer_1_mediapipe"], f"raw_frame_{f}")

    def test_dual_mute_and_privacy_mute_interplay(self):
        """BVA: Dual mute supersedes privacy mute, silencing Consumer 1 as well."""
        self.splitter.set_privacy_mute(True)
        self.splitter.set_dual_mute(True)
        out = self.splitter.feed_camera_frame(1)
        self.assertIsNone(out["consumer_1_mediapipe"])
        self.assertEqual(out["consumer_2_meeting"], "privacy_slate_frame")

    def test_idempotent_privacy_mute_calls(self):
        """BVA: Calling set_privacy_mute(True) repeatedly is idempotent."""
        self.splitter.set_privacy_mute(True)
        self.splitter.set_privacy_mute(True)
        self.splitter.set_privacy_mute(True)
        self.assertTrue(self.splitter.privacy_mute_active)
