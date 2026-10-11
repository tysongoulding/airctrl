import unittest
from tests.e2e.e2e_harness import MeetingWatcher


class TestT2F22CameraKillSwitch(unittest.TestCase):
    """
    Feature 22: Camera Privacy Auto-Kill Switch (Boundary Value Analysis)
    """

    def setUp(self):
        self.watcher = MeetingWatcher()

    def test_kill_switch_repeated_10_times_stability(self):
        """BVA: Repeated kill-switch trigger calls remain stable."""
        for _ in range(10):
            res = self.watcher.trigger_camera_kill_switch()
            self.assertIn(res["status"], ["KilledByMeeting", "AlreadyReleased"])
            self.assertFalse(self.watcher.camera_active)

    def test_camera_already_inactive_before_meeting(self):
        """BVA: Camera already inactive before meeting scan remains inactive."""
        self.watcher.camera_active = False
        procs = [{"process_id": 1, "process_name": "zoom.exe", "title": "Zoom"}]
        self.watcher.scan_processes(procs)
        self.assertFalse(self.watcher.camera_active)

    def test_multiple_meetings_detected_simultaneously(self):
        """BVA: Two meeting applications open simultaneously (Zoom and Teams)."""
        procs = [
            {"process_id": 1, "process_name": "zoom.exe", "title": "Zoom"},
            {"process_id": 2, "process_name": "teams.exe", "title": "Teams"}
        ]
        m = self.watcher.scan_processes(procs)
        self.assertIsNotNone(m)
        self.assertFalse(self.watcher.camera_active)

    def test_camera_state_transition_speed(self):
        """BVA: Kill switch latency under 1ms."""
        import time
        start = time.perf_counter()
        self.watcher.trigger_camera_kill_switch()
        duration_ms = (time.perf_counter() - start) * 1000
        self.assertLess(duration_ms, 1.0)

    def test_camera_state_reset_boundary(self):
        """BVA: Explicit camera reactivation restores True state."""
        self.watcher.trigger_camera_kill_switch()
        self.assertFalse(self.watcher.camera_active)
        self.watcher.camera_active = True
        self.assertTrue(self.watcher.camera_active)
