import unittest
from tests.e2e.e2e_harness import MeetingWatcher


class TestT1F21MeetingWatcher(unittest.TestCase):
    """
    Feature 21: Meeting Process & Audio Watcher (Equivalence Partitions)
    """

    def setUp(self):
        self.watcher = MeetingWatcher()

    def test_zoom_meeting_detected(self):
        """Partition: Active Zoom process triggers meeting detection."""
        procs = [{"process_id": 100, "process_name": "zoom.exe", "title": "Zoom Meeting"}]
        meeting = self.watcher.scan_processes(procs)
        self.assertIsNotNone(meeting)
        self.assertEqual(meeting["platform"], "Zoom")

    def test_microsoft_teams_detected(self):
        """Partition: Active Microsoft Teams process triggers meeting detection."""
        procs = [{"process_id": 200, "process_name": "teams.exe", "title": "Teams Call"}]
        meeting = self.watcher.scan_processes(procs)
        self.assertIsNotNone(meeting)
        self.assertEqual(meeting["platform"], "Teams")

    def test_google_meet_in_browser_detected(self):
        """Partition: Google Meet browser tab detected via title signature."""
        procs = [{"process_id": 300, "process_name": "chrome.exe", "title": "Daily Standup - Google Meet"}]
        meeting = self.watcher.scan_processes(procs)
        self.assertIsNotNone(meeting)
        self.assertEqual(meeting["platform"], "Meet")

    def test_webex_and_discord_detected(self):
        """Partition: Webex and Discord meetings detected."""
        procs_webex = [{"process_id": 400, "process_name": "ciscowebex.exe", "title": "Webex"}]
        m_webex = self.watcher.scan_processes(procs_webex)
        self.assertEqual(m_webex["platform"], "Webex")

        watcher2 = MeetingWatcher()
        procs_discord = [{"process_id": 500, "process_name": "Discord.exe", "title": "General Voice"}]
        m_discord = watcher2.scan_processes(procs_discord)
        self.assertEqual(m_discord["platform"], "Discord")

    def test_non_meeting_processes_ignored(self):
        """Partition: Normal apps (Code, Notepad) do not trigger meeting state."""
        procs = [
            {"process_id": 10, "process_name": "Code.exe", "title": "Visual Studio Code"},
            {"process_id": 11, "process_name": "notepad.exe", "title": "Untitled"}
        ]
        meeting = self.watcher.scan_processes(procs)
        self.assertIsNone(meeting)
