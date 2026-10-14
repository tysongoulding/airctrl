import unittest
from tests.e2e.e2e_harness import HandsFreeCommandParser


class TestT1F44AppWindowFocusCycling(unittest.TestCase):
    """
    Feature 44: Hands-Free App & Window Focus Cycling (Equivalence Partitions)
    """

    def setUp(self):
        self.parser = HandsFreeCommandParser()

    def test_focus_app_partition(self):
        """Partition: 'focus {app}' parses into SwitchFocus action."""
        cmd = self.parser.parse("focus firefox")
        self.assertEqual(cmd.action_type, "SwitchFocus")
        self.assertEqual(cmd.payload["app_name"], "firefox")

    def test_switch_to_app_alias(self):
        """Partition: 'switch to {app}' parsed as SwitchFocus action."""
        cmd = self.parser.parse("switch to terminal")
        self.assertEqual(cmd.action_type, "SwitchFocus")
        self.assertEqual(cmd.payload["app_name"], "terminal")

    def test_next_window_cycling(self):
        """Partition: 'next window' advances window z-order stack."""
        cmd = self.parser.parse("next window")
        self.assertEqual(cmd.action_type, "CycleWindow")
        self.assertEqual(cmd.payload["direction"], "Next")

    def test_previous_window_cycling(self):
        """Partition: 'previous window' traverses window z-order backwards."""
        cmd = self.parser.parse("previous window")
        self.assertEqual(cmd.action_type, "CycleWindow")
        self.assertEqual(cmd.payload["direction"], "Previous")

    def test_prev_window_abbreviation(self):
        """Partition: 'prev window' abbreviation maps to CycleWindow(Previous)."""
        cmd = self.parser.parse("prev window")
        self.assertEqual(cmd.action_type, "CycleWindow")
        self.assertEqual(cmd.payload["direction"], "Previous")
