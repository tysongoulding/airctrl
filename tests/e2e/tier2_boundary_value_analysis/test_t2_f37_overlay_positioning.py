import unittest
from tests.e2e.e2e_harness import OverlayGeometryManager, OverlayPosition


class TestT2F37OverlayPositioning(unittest.TestCase):
    """
    Feature 37: 4-State Overlay Positioning (Boundary Value Analysis)
    """

    def test_minimal_screen_resolution_800x600(self):
        """BVA: Minimal 800x600 screen resolution computes valid non-negative bounds."""
        bounds = OverlayGeometryManager.calculate_bounds(OverlayPosition.BOTTOM, 800, 600, 400, 60, 20)
        self.assertEqual(bounds["x"], 200)
        self.assertEqual(bounds["y"], 520)
        self.assertGreater(bounds["x"], 0)
        self.assertGreater(bounds["y"], 0)

    def test_massive_ultrawide_screen_7680x2160(self):
        """BVA: Dual 4K ultrawide screen (7680x2160) centers HUD accurately."""
        bounds = OverlayGeometryManager.calculate_bounds(OverlayPosition.BOTTOM, 7680, 2160, 400, 60, 30)
        self.assertEqual(bounds["x"], (7680 - 400) // 2)
        self.assertEqual(bounds["y"], 2160 - 60 - 30)

    def test_zero_margin_boundary(self):
        """BVA: Zero margin sits flush with the screen border."""
        bounds = OverlayGeometryManager.calculate_bounds(OverlayPosition.TOP, 1920, 1080, margin=0)
        self.assertEqual(bounds["y"], 0)

    def test_large_margin_boundary_300px(self):
        """BVA: 300px margin offsets HUD deeply inside viewport."""
        bounds = OverlayGeometryManager.calculate_bounds(OverlayPosition.BOTTOM, 1920, 1080, margin=300)
        self.assertEqual(bounds["y"], 1080 - 60 - 300)

    def test_none_overlay_invariants_across_all_resolutions(self):
        """BVA: None overlay always yields width=0, height=0, visible=False."""
        for res in [(640, 480), (1920, 1080), (3840, 2160), (7680, 4320)]:
            b = OverlayGeometryManager.calculate_bounds(OverlayPosition.NONE, res[0], res[1])
            self.assertFalse(b["visible"])
            self.assertEqual(b["width"], 0)
            self.assertEqual(b["height"], 0)
