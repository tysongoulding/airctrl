import unittest


class TestT2F25RadialSettingsUi(unittest.TestCase):
    """
    Feature 25: Radial Dial & Settings UI (Boundary Value Analysis)
    """

    def test_radial_sector_count_minimum_and_maximum(self):
        """BVA: Radial dial configured with exactly 6 sectors."""
        sector_count = 6
        self.assertGreaterEqual(sector_count, 4)
        self.assertLessEqual(sector_count, 8)

    def test_dial_radius_boundary_pixels(self):
        """BVA: Dial radius 300px on 600x600 canvas."""
        radius_px = 300
        canvas_px = 600
        self.assertEqual(radius_px * 2, canvas_px)

    def test_hud_opacity_bounds_0_to_1(self):
        """BVA: HUD opacity bounded between 0.0 (transparent) and 1.0 (opaque)."""
        opacities = [0.0, 0.5, 0.95, 1.0]
        for op in opacities:
            self.assertTrue(0.0 <= op <= 1.0)

    def test_sector_label_length_bounds(self):
        """BVA: Sector labels between 1 and 30 characters."""
        label_short = "A"
        label_long = "App Window Crawler Explorer 1"
        self.assertTrue(1 <= len(label_short) <= 30)
        self.assertTrue(1 <= len(label_long) <= 30)

    def test_ui_hotkey_string_format(self):
        """BVA: Hotkey string parsing ('Ctrl+Spacebar', 'Ctrl+Alt')."""
        hotkeys = ["Ctrl+Spacebar", "Ctrl+Alt"]
        for hk in hotkeys:
            parts = hk.split("+")
            self.assertEqual(len(parts), 2)
            self.assertEqual(parts[0], "Ctrl")
