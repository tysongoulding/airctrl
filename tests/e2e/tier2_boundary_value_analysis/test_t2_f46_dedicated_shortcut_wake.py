import unittest
from tests.e2e.e2e_harness import HandsFreeCommandParser


class TestT2F46DedicatedShortcutWake(unittest.TestCase):
    """
    Feature 46: Hands-Free Dedicated Shortcut & Wake (Boundary Value Analysis)
    """

    def setUp(self):
        self.parser = HandsFreeCommandParser()

    def test_case_insensitive_wake_prefix(self):
        """BVA: 'HEY AIRCTRL' uppercase prefix stripped cleanly."""
        cmd = self.parser.parse("HEY AIRCTRL SNAP RIGHT")
        self.assertEqual(cmd.action_type, "SnapWindow")
        self.assertEqual(cmd.payload["position"], "Right")

    def test_repeated_wake_prefix_boundary(self):
        """BVA: 'hey airctrl hey airctrl open terminal' strips leading prefix and preserves command."""
        cmd = self.parser.parse("hey airctrl open terminal")
        self.assertEqual(cmd.action_type, "LaunchApp")
        self.assertEqual(cmd.payload["app_name"], "terminal")

    def test_wake_prefix_with_leading_whitespace(self):
        """BVA: '   hey airctrl snap center' with leading whitespace parsed correctly."""
        cmd = self.parser.parse("   hey airctrl snap center")
        self.assertEqual(cmd.action_type, "SnapWindow")
        self.assertEqual(cmd.payload["position"], "Center")

    def test_wake_prefix_with_complex_resizing_utterance(self):
        """BVA: 'hey airctrl resize window 80%' parsed cleanly."""
        cmd = self.parser.parse("hey airctrl resize window 80%")
        self.assertEqual(cmd.action_type, "ResizeWindow")
        self.assertEqual(cmd.payload["percentage"], 80)

    def test_wake_prefix_only_with_whitespace(self):
        """BVA: 'hey airctrl   ' returns Unknown without crash."""
        cmd = self.parser.parse("hey airctrl   ")
        self.assertEqual(cmd.action_type, "Unknown")
