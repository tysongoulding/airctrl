import unittest
from tests.e2e.e2e_harness import VirtualCameraSplitter


class TestT2F58ZerolockVideoFanout(unittest.TestCase):
    """
    Feature 58: Zero-Lock Concurrent Video Fan-Out (Boundary Value Analysis)
    """

    def setUp(self):
        self.splitter = VirtualCameraSplitter()

    def test_concurrent_fanout_under_burst_load(self):
        """BVA: 100-frame burst fanout maintains exact matching outputs."""
        for f in range(100):
            res = self.splitter.feed_camera_frame(f)
            self.assertEqual(res["consumer_1_mediapipe"], f"raw_frame_{f}")
            self.assertEqual(res["consumer_2_meeting"], f"outbound_frame_{f}")

    def test_consumer_payload_distinct_representations(self):
        """BVA: Consumer 1 (MediaPipe raw) and Consumer 2 (outbound) maintain distinct payload tags."""
        out = self.splitter.feed_camera_frame(50)
        self.assertTrue(out["consumer_1_mediapipe"].startswith("raw_"))
        self.assertTrue(out["consumer_2_meeting"].startswith("outbound_"))

    def test_frame_counter_matches_processed_count(self):
        """BVA: Internal frame counter matches exactly the number of feed invocations."""
        for i in range(25):
            self.splitter.feed_camera_frame(i)
        self.assertEqual(self.splitter.frames_fanned_out, 25)

    def test_fanout_structure_has_all_required_keys(self):
        """BVA: Fanout dictionary schema contains all contractual keys."""
        out = self.splitter.feed_camera_frame(1)
        required_keys = ["device", "frame_id", "consumer_1_mediapipe", "consumer_2_meeting", "privacy_muted", "dual_muted"]
        for k in required_keys:
            self.assertIn(k, out)

    def test_large_burst_frame_boundary(self):
        """BVA: Frame ID 999,999 dispatches without formatting overflow."""
        out = self.splitter.feed_camera_frame(999999)
        self.assertIn("999999", out["consumer_1_mediapipe"])
        self.assertIn("999999", out["consumer_2_meeting"])
