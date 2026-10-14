import unittest
from tests.e2e.e2e_harness import HandsFreeCommandParser


class TestT2F44AppWindowFocusCycling(unittest.TestCase):
    """
    Feature 44: Hands-Free App & Window Focus Cycling (Boundary Value Analysis)
    """

    def setUp(self):
        self.parser = HandsFreeCommandParser()

    def test_focus_command_with_short_acronym(self):
        """BVA: 2-character acronym 'focus id' parsed cleanly."""
        cmd = self.parser.parse("focus id")
        self.assertEqual(cmd.action_type, "SwitchFocus")
        self.assertEqual(cmd.payload["app_name"], "id")

    def test_case_insensitivity_focus_command(self):
        """BVA: 'SWITCH TO DISCORD' uppercase parsed accurately."""
        cmd = self.parser.parse("SWITCH TO DISCORD")
        self.assertEqual(cmd.action_type, "SwitchFocus")
        self.assertEqual(cmd.payload["app_name"], "discord")

    def test_cycling_window_repetitive_boundary(self):
        """BVA: 20 sequential next window commands parse deterministically."""
        for _ in range(20):
            cmd = self.parser.parse("next window")
            self.assertEqual(cmd.payload["direction"], "Next")

    def test_focus_command_with_trailing_punctuation(self):
        """BVA: Trailing punctuation 'switch to VS Code.' parsed cleanly."""
        cmd = self.parser.parse("switch to VS Code.")
        self.assertEqual(cmd.action_type, "SwitchFocus")
        self.assertIn("vs code", cmd.payload["app_name"])

    def test_empty_focus_target_returns_unknown(self):
        """BVA: 'focus' with empty target returns Unknown."""
        cmd = self.parser.parse("focus")
        self.assertEqual(cmd.action_type, "Unknown")
