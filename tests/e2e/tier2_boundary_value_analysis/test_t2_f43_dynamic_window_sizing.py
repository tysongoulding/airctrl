import unittest
from tests.e2e.e2e_harness import HandsFreeCommandParser, WindowGeometryEngine


class TestT2F43DynamicWindowSizing(unittest.TestCase):
    """
    Feature 43: Dynamic Window Sizing Engine (Boundary Value Analysis)
    """

    def setUp(self):
        self.parser = HandsFreeCommandParser()
        self.engine = WindowGeometryEngine(1920, 1080)
        self.base_bounds = {"x": 100, "y": 100, "width": 800, "height": 600}

    def test_min_and_max_percentage_boundaries(self):
        """BVA: Exactly 10% (minimum clamp) and 100% (maximum clamp)."""
        cmd_min = self.parser.parse("resize window 10%")
        b_min = self.engine.compute_resize(self.base_bounds, cmd_min)
        self.assertEqual(b_min["width"], 192)
        self.assertEqual(b_min["height"], 108)

        cmd_max = self.parser.parse("resize window 100%")
        b_max = self.engine.compute_resize(self.base_bounds, cmd_max)
        self.assertEqual(b_max["width"], 1920)
        self.assertEqual(b_max["height"], 1080)

    def test_percentage_clamping_out_of_bounds(self):
        """BVA: 150% clamped to 100%; 5% clamped to 10%."""
        cmd_over = self.parser.parse("resize window 150%")
        self.assertEqual(cmd_over.payload["percentage"], 100)

        cmd_under = self.parser.parse("resize window 5%")
        self.assertEqual(cmd_under.payload["percentage"], 10)

    def test_minimum_window_size_floor_200px(self):
        """BVA: Sizing delta will not shrink window below 200x200 pixel safety floor."""
        small_window = {"x": 50, "y": 50, "width": 250, "height": 250}
        cmd = self.parser.parse("make window smaller by 100")
        b = self.engine.compute_resize(small_window, cmd)
        self.assertEqual(b["width"], 200)
        self.assertEqual(b["height"], 200)

    def test_explicit_dimension_matching_screen_resolution(self):
        """BVA: Explicit dimensions matching exact screen size (1920 by 1080)."""
        cmd = self.parser.parse("resize 1920 by 1080")
        b = self.engine.compute_resize(self.base_bounds, cmd)
        self.assertEqual(b["width"], 1920)
        self.assertEqual(b["height"], 1080)

    def test_explicit_dimension_exceeding_screen_resolution_clamped(self):
        """BVA: Explicit dimensions exceeding screen size (3000 by 2000) clamped to display limit."""
        cmd = self.parser.parse("resize 3000 by 2000")
        b = self.engine.compute_resize(self.base_bounds, cmd)
        self.assertEqual(b["width"], 1920)
        self.assertEqual(b["height"], 1080)
