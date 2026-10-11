import unittest
from tests.e2e.tier1_category_partition.test_t1_f11_hotkey_ctrl_space import MockHotkeyListener


class TestT2F11HotkeyCtrlSpace(unittest.TestCase):
    """
    Feature 11: Global Hotkey Ctrl+Spacebar (Boundary Value Analysis)
    """

    def setUp(self):
        self.listener = MockHotkeyListener()
        self.listener.mode = "PushToTalk"

    def test_press_duration_99ms_cancelled(self):
        """BVA: Exactly 99ms press (< 100ms) is cancelled back to Idle."""
        self.listener.on_key_down(1000)
        self.listener.on_key_up(1099)
        self.assertEqual(self.listener.state, "Idle")

    def test_press_duration_100ms_accepted(self):
        """BVA: Exactly 100ms press (>= 100ms) transitions to Processing."""
        self.listener.on_key_down(1000)
        self.listener.on_key_up(1100)
        self.assertEqual(self.listener.state, "Processing")

    def test_press_duration_101ms_accepted(self):
        """BVA: Exactly 101ms press transitions to Processing."""
        self.listener.on_key_down(1000)
        self.listener.on_key_up(1101)
        self.assertEqual(self.listener.state, "Processing")

    def test_press_duration_1ms_tap_cancelled(self):
        """BVA: 1ms micro-tap cancelled to Idle."""
        self.listener.on_key_down(1000)
        self.listener.on_key_up(1001)
        self.assertEqual(self.listener.state, "Idle")

    def test_long_press_duration_30000ms_handled(self):
        """BVA: 30-second continuous press handled cleanly."""
        self.listener.on_key_down(1000)
        self.listener.on_key_up(31000)
        self.assertEqual(self.listener.state, "Processing")
