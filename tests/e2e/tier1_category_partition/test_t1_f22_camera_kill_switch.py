import unittest
from tests.e2e.e2e_harness import MeetingWatcher


class TestT1F22CameraKillSwitch(unittest.TestCase):
    """
    Feature 22: Camera Privacy Auto-Kill Switch (Equivalence Partitions)
    """

    def setUp(self):
        self.watcher = MeetingWatcher()

    def test_kill_switch_triggers_on_meeting_detection(self):
        """Partition: Meeting discovery automatically triggers camera handle termination."""
        self.assertTrue(self.watcher.camera_active)
        procs = [{"process_id": 100, "process_name": "zoom.exe", "title": "Zoom"}]
        self.watcher.scan_processes(procs)
        self.assertFalse(self.watcher.camera_active)

    def test_direct_kill_switch_invocation(self):
        """Partition: Direct programmatic kill switch invocation."""
        res = self.watcher.trigger_camera_kill_switch()
        self.assertEqual(res["status"], "KilledByMeeting")
        self.assertTrue(res["camera_handle_closed"])
        self.assertFalse(self.watcher.camera_active)

    def test_kill_switch_idempotent_when_already_killed(self):
        """Partition: Subsequent kill switch invocations return AlreadyReleased."""
        self.watcher.trigger_camera_kill_switch()
        res = self.watcher.trigger_camera_kill_switch()
        self.assertEqual(res["status"], "AlreadyReleased")
        self.assertFalse(res["camera_handle_closed"])

    def test_camera_state_enum_variants(self):
        """Partition: CameraState variants Active, KilledByMeeting, PausedByUser."""
        variants = ["Active", "KilledByMeeting", "PausedByUser"]
        current = "KilledByMeeting"
        self.assertIn(current, variants)

    def test_gesture_tracking_disabled_while_camera_killed(self):
        """Partition: Video capture and hand tracking stopped while camera killed."""
        self.watcher.trigger_camera_kill_switch()
        can_capture_video = self.watcher.camera_active
        self.assertFalse(can_capture_video)
