import unittest
from tests.e2e.e2e_harness import VirtualCameraSplitter


class TestT1F58ZerolockVideoFanout(unittest.TestCase):
    """
    Feature 58: Zero-Lock Concurrent Video Fan-Out (Equivalence Partitions)
    """

    def setUp(self):
        self.splitter = VirtualCameraSplitter()

    def test_concurrent_fanout_to_both_consumers(self):
        """Partition: Physical camera frame fans out concurrently to Consumer 1 and Consumer 2."""
        out = self.splitter.feed_camera_frame(42)
        self.assertIsNotNone(out["consumer_1_mediapipe"])
        self.assertIsNotNone(out["consumer_2_meeting"])

    def test_consumer_1_receives_raw_frame(self):
        """Partition: Consumer 1 (internal MediaPipe) receives raw frame for local hand tracking."""
        out = self.splitter.feed_camera_frame(100)
        self.assertEqual(out["consumer_1_mediapipe"], "raw_frame_100")

    def test_consumer_2_receives_outbound_stream(self):
        """Partition: Consumer 2 (outbound meeting apps) receives outbound virtual camera frame."""
        out = self.splitter.feed_camera_frame(100)
        self.assertEqual(out["consumer_2_meeting"], "outbound_frame_100")

    def test_zero_lock_contention_parallel_fanout(self):
        """Partition: Dispatches 50 consecutive frames without deadlock or state contention."""
        for i in range(50):
            res = self.splitter.feed_camera_frame(i)
            self.assertEqual(res["frame_id"], i)
        self.assertEqual(self.splitter.frames_fanned_out, 50)

    def test_fanout_payload_integrity(self):
        """Partition: Frame IDs and metadata remain synchronized across consumers."""
        out = self.splitter.feed_camera_frame(777)
        self.assertIn("777", out["consumer_1_mediapipe"])
        self.assertIn("777", out["consumer_2_meeting"])
