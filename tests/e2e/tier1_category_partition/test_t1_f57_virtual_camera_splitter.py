import unittest
from tests.e2e.e2e_harness import VirtualCameraSplitter


class TestT1F57VirtualCameraSplitter(unittest.TestCase):
    """
    Feature 57: Continuous Virtual Camera Splitter (Equivalence Partitions)
    """

    def setUp(self):
        self.splitter = VirtualCameraSplitter()

    def test_continuous_splitter_startup(self):
        """Partition: Splitter runs continuously in the background upon startup."""
        self.assertTrue(self.splitter.is_running)

    def test_persistent_device_name(self):
        """Partition: Persistent virtual camera device named 'AirCtrl Virtual Camera'."""
        self.assertEqual(self.splitter.device_name, "AirCtrl Virtual Camera")

    def test_feed_frame_generates_outputs(self):
        """Partition: Ingested physical camera frame generates stream outputs."""
        out = self.splitter.feed_camera_frame(1)
        self.assertEqual(out["frame_id"], 1)
        self.assertEqual(out["device"], "AirCtrl Virtual Camera")

    def test_frame_counter_advancement(self):
        """Partition: Continuous streaming advances frame counter monotonically."""
        self.splitter.feed_camera_frame(1)
        self.splitter.feed_camera_frame(2)
        self.assertEqual(self.splitter.frames_fanned_out, 2)

    def test_stopped_splitter_error(self):
        """Partition: Feeding frames to terminated splitter raises RuntimeError."""
        self.splitter.is_running = False
        with self.assertRaises(RuntimeError):
            self.splitter.feed_camera_frame(10)
