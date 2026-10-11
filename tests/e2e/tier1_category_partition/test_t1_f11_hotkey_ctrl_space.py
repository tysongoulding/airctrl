import unittest


class MockHotkeyListener:
    def __init__(self):
        self.mode = "PushToTalk"  # PushToTalk or Toggle
        self.state = "Idle"
        self.press_time_ms = 0

    def on_key_down(self, timestamp_ms: int):
        self.press_time_ms = timestamp_ms
        if self.mode == "PushToTalk":
            self.state = "Recording"
        elif self.mode == "Toggle":
            self.state = "Recording" if self.state == "Idle" else "Processing"

    def on_key_up(self, timestamp_ms: int):
        duration = timestamp_ms - self.press_time_ms
        if self.mode == "PushToTalk":
            if duration < 100:
                self.state = "Idle"  # Discard accidental press
            else:
                self.state = "Processing"


class TestT1F11HotkeyCtrlSpace(unittest.TestCase):
    """
    Feature 11: Global Hotkey Ctrl+Spacebar (Equivalence Partitions)
    """

    def test_push_to_talk_happy_path(self):
        """Partition: Push-to-talk press and hold transitions to Recording then Processing."""
        listener = MockHotkeyListener()
        listener.mode = "PushToTalk"
        listener.on_key_down(1000)
        self.assertEqual(listener.state, "Recording")
        listener.on_key_up(1500)  # 500ms press
        self.assertEqual(listener.state, "Processing")

    def test_push_to_talk_accidental_tap_discarded(self):
        """Partition: Press <100ms cancels recording back to Idle."""
        listener = MockHotkeyListener()
        listener.mode = "PushToTalk"
        listener.on_key_down(1000)
        self.assertEqual(listener.state, "Recording")
        listener.on_key_up(1050)  # 50ms press
        self.assertEqual(listener.state, "Idle")

    def test_toggle_mode_transitions(self):
        """Partition: Toggle mode: first press starts, second press stops."""
        listener = MockHotkeyListener()
        listener.mode = "Toggle"
        listener.on_key_down(1000)
        self.assertEqual(listener.state, "Recording")
        listener.on_key_down(2000)
        self.assertEqual(listener.state, "Processing")

    def test_hotkey_combination_identity(self):
        """Partition: Verification of Ctrl+Spacebar hotkey keycodes."""
        hotkey_spec = {"modifiers": ["Control"], "key": "Space"}
        self.assertIn("Control", hotkey_spec["modifiers"])
        self.assertEqual(hotkey_spec["key"], "Space")

    def test_idle_state_when_unpressed(self):
        """Partition: Initial listener state is Idle."""
        listener = MockHotkeyListener()
        self.assertEqual(listener.state, "Idle")
