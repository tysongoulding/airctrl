import unittest


class MockRadialDialOverlay:
    def __init__(self):
        self.is_visible = False
        self.active_sector = None
        self.sector_count = 6
        self.timeout_secs = 3.0

    def toggle(self):
        self.is_visible = not self.is_visible
        if not self.is_visible:
            self.active_sector = None

    def hover_sector(self, sector_idx: int):
        if self.is_visible and 0 <= sector_idx < self.sector_count:
            self.active_sector = sector_idx

    def on_inactivity_timeout(self):
        self.is_visible = False
        self.active_sector = None


class TestT1F14RadialOverlay(unittest.TestCase):
    """
    Feature 14: Radial Dial Pointer Overlay (Equivalence Partitions)
    """

    def setUp(self):
        self.dial = MockRadialDialOverlay()

    def test_initial_state_is_hidden(self):
        """Partition: Radial dial starts hidden."""
        self.assertFalse(self.dial.is_visible)
        self.assertIsNone(self.dial.active_sector)

    def test_ctrl_alt_hotkey_toggles_visibility(self):
        """Partition: Hotkey Ctrl+Alt toggles dial visible then hidden."""
        self.dial.toggle()
        self.assertTrue(self.dial.is_visible)
        self.dial.toggle()
        self.assertFalse(self.dial.is_visible)

    def test_hover_valid_sector(self):
        """Partition: Hovering sector updates active_sector."""
        self.dial.toggle()
        self.dial.hover_sector(2)
        self.assertEqual(self.dial.active_sector, 2)

    def test_hover_invalid_sector_ignored(self):
        """Partition: Hovering out-of-range sector is ignored."""
        self.dial.toggle()
        self.dial.hover_sector(10)
        self.assertIsNone(self.dial.active_sector)

    def test_inactivity_timeout_auto_dismiss(self):
        """Partition: Inactivity timeout dismisses overlay."""
        self.dial.toggle()
        self.dial.hover_sector(3)
        self.dial.on_inactivity_timeout()
        self.assertFalse(self.dial.is_visible)
        self.assertIsNone(self.dial.active_sector)
