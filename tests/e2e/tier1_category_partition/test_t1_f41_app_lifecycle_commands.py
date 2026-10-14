import unittest
from tests.e2e.e2e_harness import HandsFreeCommandParser


class TestT1F41AppLifecycleCommands(unittest.TestCase):
    """
    Feature 41: Voice App Lifecycle Commands (Equivalence Partitions)
    """

    def setUp(self):
        self.parser = HandsFreeCommandParser()

    def test_open_app_auto_centered(self):
        """Partition: 'open {app}' command emits LaunchApp with auto_center=True."""
        cmd = self.parser.parse("open google chrome")
        self.assertEqual(cmd.action_type, "LaunchApp")
        self.assertEqual(cmd.payload["app_name"], "google chrome")
        self.assertTrue(cmd.payload["auto_center"])

    def test_launch_app_alias(self):
        """Partition: 'launch {app}' command parsed as LaunchApp."""
        cmd = self.parser.parse("launch code")
        self.assertEqual(cmd.action_type, "LaunchApp")
        self.assertEqual(cmd.payload["app_name"], "code")

    def test_close_specific_app(self):
        """Partition: 'close {app}' command emits CloseApp targeting named process."""
        cmd = self.parser.parse("close slack")
        self.assertEqual(cmd.action_type, "CloseApp")
        self.assertEqual(cmd.payload["target"], "App")
        self.assertEqual(cmd.payload["app_name"], "slack")

    def test_close_current_window(self):
        """Partition: 'close window' targets current active window."""
        cmd = self.parser.parse("close window")
        self.assertEqual(cmd.action_type, "CloseApp")
        self.assertEqual(cmd.payload["target"], "CurrentWindow")

    def test_quit_app_alias(self):
        """Partition: 'quit {app}' is parsed as CloseApp."""
        cmd = self.parser.parse("quit spotify")
        self.assertEqual(cmd.action_type, "CloseApp")
        self.assertEqual(cmd.payload["target"], "App")
        self.assertEqual(cmd.payload["app_name"], "spotify")
