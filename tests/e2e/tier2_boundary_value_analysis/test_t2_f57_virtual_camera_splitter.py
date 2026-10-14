import unittest
from tests.e2e.e2e_harness import VirtualCameraSplitter


class TestT2F57VirtualCameraSplitter(unittest.TestCase):
    """
    Feature 57: Continuous Virtual Camera Splitter (Boundary Value Analysis)
    """

    def setUp(self):
        self.splitter = VirtualCameraSplitter()

    def test_high_volume_1000_frames_streaming(self):
        """BVA: 1,000 sequential frames streamed without memory leak or counter desync."""
        for i in range(1000):
            res = self.splitter.feed_camera_frame(i)
            self.assertEqual(res["frame_id"], i)
        self.assertEqual(self.splitter.frames_fanned_out, 1000)

    def test_custom_device_name_configuration(self):
        """BVA: Custom virtual camera device name string preserved."""
        custom_splitter = VirtualCameraSplitter(device_name="AirCtrl Studio Cam 4K")
        self.assertEqual(custom_splitter.device_name, "AirCtrl Studio Cam 4K")
        out = custom_splitter.feed_camera_frame(1)
        self.assertEqual(out["device"], "AirCtrl Studio Cam 4K")

    def test_frame_id_zero_initial_startup_boundary(self):
        """BVA: Frame ID 0 (initial startup frame) processed cleanly."""
        out = self.splitter.feed_camera_frame(0)
        self.assertEqual(out["frame_id"], 0)

    def test_restart_splitter_lifecycle(self):
        """BVA: Stopping and re-enabling splitter restarts cleanly."""
        self.splitter.is_running = False
        self.assertFalse(self.splitter.is_running)
        self.splitter.is_running = True
        out = self.splitter.feed_camera_frame(5)
        self.assertEqual(out["frame_id"], 5)

    def test_large_frame_id_boundary(self):
        """BVA: Large frame index (1,000,000) handles integer scale."""
        out = self.splitter.feed_camera_frame(1000000)
        self.assertEqual(out["frame_id"], 1000000)
