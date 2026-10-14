import unittest
from tests.e2e.e2e_harness import HandsFreeCommandParser, MouseButton, ScrollDirection, UiTarget


class TestT2F45UiNavigationClicking(unittest.TestCase):
    """
    Feature 45: Hands-Free UI Navigation & Clicking (Boundary Value Analysis)
    """

    def setUp(self):
        self.parser = HandsFreeCommandParser()

    def test_scroll_amount_boundary_explicit_values(self):
        """BVA: 'scroll up 1' and 'scroll down 100' amounts parsed accurately."""
        c1 = self.parser.parse("scroll up 1")
        self.assertEqual(c1.payload["amount"], 1)

        c100 = self.parser.parse("scroll down 100")
        self.assertEqual(c100.payload["amount"], 100)

    def test_uppercase_ui_navigation_keystrokes(self):
        """BVA: 'PRESS ENTER', 'PRESS TAB', 'PRESS ESCAPE' parsed cleanly."""
        for u, t in [("PRESS ENTER", UiTarget.ENTER), ("PRESS TAB", UiTarget.TAB), ("PRESS ESCAPE", UiTarget.ESCAPE)]:
            cmd = self.parser.parse(u)
            self.assertEqual(cmd.payload["element"], t.value)

    def test_page_up_and_down_defaults(self):
        """BVA: 'page up' and 'page down' default to amount=1."""
        c_pu = self.parser.parse("page up")
        self.assertEqual(c_pu.payload["direction"], ScrollDirection.PAGE_UP.value)
        self.assertEqual(c_pu.payload["amount"], 1)

        c_pd = self.parser.parse("page down")
        self.assertEqual(c_pd.payload["direction"], ScrollDirection.PAGE_DOWN.value)
        self.assertEqual(c_pd.payload["amount"], 1)

    def test_mouse_double_click_count_invariant(self):
        """BVA: 'double click' maps to button=Double, count=2."""
        cmd = self.parser.parse("double click")
        self.assertEqual(cmd.payload["button"], MouseButton.DOUBLE.value)
        self.assertEqual(cmd.payload["count"], 2)

    def test_context_click_alias_for_right_click(self):
        """BVA: 'context click' maps to button=Right."""
        cmd = self.parser.parse("context click")
        self.assertEqual(cmd.payload["button"], MouseButton.RIGHT.value)
