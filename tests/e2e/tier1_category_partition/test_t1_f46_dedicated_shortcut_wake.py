import unittest
from tests.e2e.e2e_harness import HandsFreeCommandParser


class TestT1F46DedicatedShortcutWake(unittest.TestCase):
    """
    Feature 46: Hands-Free Dedicated Shortcut & Wake (Equivalence Partitions)
    """

    def setUp(self):
        self.parser = HandsFreeCommandParser()

    def test_voice_wake_prefix_hey_airctrl(self):
        """Partition: 'hey airctrl' prefix stripped and inner command executed."""
        cmd = self.parser.parse("hey airctrl snap left")
        self.assertEqual(cmd.action_type, "SnapWindow")
        self.assertEqual(cmd.payload["position"], "Left")

    def test_voice_wake_prefix_airctrl(self):
        """Partition: 'airctrl' prefix stripped and inner command executed."""
        cmd = self.parser.parse("airctrl close window")
        self.assertEqual(cmd.action_type, "CloseApp")
        self.assertEqual(cmd.payload["target"], "CurrentWindow")

    def test_configurable_shortcut_trigger(self):
        """Partition: Command parsed directly as invoked by dedicated Ctrl+Spacebar shortcut."""
        cmd = self.parser.parse("resize window 80%")
        self.assertEqual(cmd.action_type, "ResizeWindow")
        self.assertEqual(cmd.payload["percentage"], 80)

    def test_continuous_hands_free_utterance(self):
        """Partition: Continuous mode processes raw utterance without hotkeys or prefixes."""
        cmd = self.parser.parse("switch to Slack")
        self.assertEqual(cmd.action_type, "SwitchFocus")
        self.assertEqual(cmd.payload["app_name"], "slack")

    def test_empty_wake_prefix_only(self):
        """Partition: Utterance containing only wake word returns Unknown query."""
        cmd = self.parser.parse("hey airctrl")
        self.assertEqual(cmd.action_type, "Unknown")
        self.assertEqual(cmd.payload["query"], "")
