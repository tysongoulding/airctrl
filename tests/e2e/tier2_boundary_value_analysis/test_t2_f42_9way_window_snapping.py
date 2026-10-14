import unittest
from tests.e2e.e2e_harness import HandsFreeCommandParser, SnapPosition, WindowGeometryEngine


class TestT2F429WayWindowSnapping(unittest.TestCase):
    """
    Feature 42: 9-Way Window Snapping & Docking (Boundary Value Analysis)
    """

    def setUp(self):
        self.parser = HandsFreeCommandParser()
        self.engine = WindowGeometryEngine(1920, 1080)

    def test_all_uppercase_snap_command(self):
        """BVA: Uppercase 'SNAP FULLSCREEN' parses cleanly."""
        cmd = self.parser.parse("SNAP FULLSCREEN")
        self.assertEqual(cmd.action_type, "SnapWindow")
        self.assertEqual(cmd.payload["position"], "Fullscreen")

    def test_dock_alias_with_irregular_spacing(self):
        """BVA: 'dock   left' with irregular whitespace parses cleanly."""
        cmd = self.parser.parse("dock   left")
        self.assertEqual(cmd.action_type, "SnapWindow")
        self.assertEqual(cmd.payload["position"], "Left")

    def test_all_9_snap_positions_boundary_loop(self):
        """BVA: All 9 snap positions computed within valid screen boundaries."""
        for pos in SnapPosition:
            bounds = self.engine.compute_snap(pos)
            self.assertTrue(0 <= bounds["x"] <= 1920)
            self.assertTrue(0 <= bounds["y"] <= 1080)
            self.assertTrue(0 <= bounds["width"] <= 1920)
            self.assertTrue(0 <= bounds["height"] <= 1080)

    def test_minimize_zero_area_invariant(self):
        """BVA: Minimize always yields width=0, height=0."""
        b = self.engine.compute_snap(SnapPosition.MINIMIZE)
        self.assertEqual(b["width"], 0)
        self.assertEqual(b["height"], 0)

    def test_center_bounds_on_small_display_1024x768(self):
        """BVA: Center bounds computation on legacy 1024x768 display."""
        eng_small = WindowGeometryEngine(1024, 768)
        b = eng_small.compute_snap(SnapPosition.CENTER)
        self.assertEqual(b["width"], int(1024 * 0.70))
        self.assertEqual(b["height"], int(768 * 0.70))
        self.assertGreater(b["x"], 0)
        self.assertGreater(b["y"], 0)
