import unittest
from tests.e2e.e2e_harness import HandsFreeCommandParser, MouseButton, ScrollDirection, UiTarget


class TestT1F45UiNavigationClicking(unittest.TestCase):
    """
    Feature 45: Hands-Free UI Navigation & Clicking (Equivalence Partitions)
    """

    def setUp(self):
        self.parser = HandsFreeCommandParser()

    def test_ui_field_navigation_partitions(self):
        """Partition: Form navigation commands (next field, previous field)."""
        cmd_next = self.parser.parse("next field")
        self.assertEqual(cmd_next.action_type, "NavigateUi")
        self.assertEqual(cmd_next.payload["element"], UiTarget.NEXT_FIELD.value)

        cmd_prev = self.parser.parse("previous field")
        self.assertEqual(cmd_prev.action_type, "NavigateUi")
        self.assertEqual(cmd_prev.payload["element"], UiTarget.PREV_FIELD.value)

    def test_ui_key_actions_partitions(self):
        """Partition: Keystroke emulation commands (enter, tab, escape)."""
        for utterance, target in [("press enter", UiTarget.ENTER), ("press tab", UiTarget.TAB), ("press escape", UiTarget.ESCAPE)]:
            cmd = self.parser.parse(utterance)
            self.assertEqual(cmd.action_type, "NavigateUi")
            self.assertEqual(cmd.payload["element"], target.value)

    def test_directional_scrolling_partitions(self):
        """Partition: Scroll up, down, page up, page down."""
        cmd_up = self.parser.parse("scroll up 5")
        self.assertEqual(cmd_up.action_type, "Scroll")
        self.assertEqual(cmd_up.payload["direction"], ScrollDirection.UP.value)
        self.assertEqual(cmd_up.payload["amount"], 5)

        cmd_pd = self.parser.parse("page down")
        self.assertEqual(cmd_pd.action_type, "Scroll")
        self.assertEqual(cmd_pd.payload["direction"], ScrollDirection.PAGE_DOWN.value)

    def test_mouse_left_and_right_click_partitions(self):
        """Partition: Primary left click and secondary right click."""
        cmd_left = self.parser.parse("left click")
        self.assertEqual(cmd_left.action_type, "MouseClick")
        self.assertEqual(cmd_left.payload["button"], MouseButton.LEFT.value)

        cmd_right = self.parser.parse("right click")
        self.assertEqual(cmd_right.action_type, "MouseClick")
        self.assertEqual(cmd_right.payload["button"], MouseButton.RIGHT.value)

    def test_mouse_double_and_middle_click_partitions(self):
        """Partition: Double click and middle click."""
        cmd_double = self.parser.parse("double click")
        self.assertEqual(cmd_double.payload["button"], MouseButton.DOUBLE.value)
        self.assertEqual(cmd_double.payload["count"], 2)

        cmd_middle = self.parser.parse("middle click")
        self.assertEqual(cmd_middle.payload["button"], MouseButton.MIDDLE.value)
