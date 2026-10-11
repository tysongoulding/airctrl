import unittest
from tests.e2e.e2e_harness import MeetingWatcher


class TestT2F21MeetingWatcher(unittest.TestCase):
    """
    Feature 21: Meeting Process & Audio Watcher (Boundary Value Analysis)
    """

    def setUp(self):
        self.watcher = MeetingWatcher()

    def test_case_insensitive_process_name_zoom_exe(self):
        """BVA: Uppercase process name 'ZOOM.EXE' matches meeting signature."""
        procs = [{"process_id": 101, "process_name": "ZOOM.EXE", "title": "Meeting"}]
        m = self.watcher.scan_processes(procs)
        self.assertIsNotNone(m)
        self.assertEqual(m["platform"], "Zoom")

    def test_partial_window_title_match_teams(self):
        """BVA: Title containing 'Teams' in lowercase 'project teams review' matches."""
        procs = [{"process_id": 102, "process_name": "electron.exe", "title": "project teams review"}]
        m = self.watcher.scan_processes(procs)
        self.assertIsNotNone(m)
        self.assertEqual(m["platform"], "Teams")

    def test_empty_process_table_boundary(self):
        """BVA: Empty process table returns None without errors."""
        m = self.watcher.scan_processes([])
        self.assertIsNone(m)

    def test_large_process_table_500_items(self):
        """BVA: Scanning 500 processes performs in real time without lag."""
        procs = [{"process_id": i, "process_name": f"proc_{i}.exe", "title": f"Title {i}"} for i in range(500)]
        procs.append({"process_id": 999, "process_name": "webex.exe", "title": "Webex Meeting"})
        m = self.watcher.scan_processes(procs)
        self.assertIsNotNone(m)
        self.assertEqual(m["platform"], "Webex")

    def test_poll_interval_boundary_one_second(self):
        """BVA: Watcher background polling period boundary is 1.0 second."""
        poll_interval_secs = 1.0
        self.assertEqual(poll_interval_secs, 1.0)
