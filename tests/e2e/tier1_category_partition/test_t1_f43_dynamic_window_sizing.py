import unittest
from tests.e2e.e2e_harness import HandsFreeCommandParser, WindowGeometryEngine


class TestT1F43DynamicWindowSizing(unittest.TestCase):
    """
    Feature 43: Dynamic Window Sizing Engine (Equivalence Partitions)
    """

    def setUp(self):
        self.parser = HandsFreeCommandParser()
        self.engine = WindowGeometryEngine(1920, 1080)
        self.initial_bounds = {"x": 200, "y": 200, "width": 800, "height": 600}

    def test_delta_larger_and_smaller_partitions(self):
        """Partition: Delta larger and smaller symmetrically modifies width and height."""
        cmd_larger = self.parser.parse("make window larger")
        b_larger = self.engine.compute_resize(self.initial_bounds, cmd_larger)
        self.assertEqual(b_larger["width"], 900)
        self.assertEqual(b_larger["height"], 700)

        cmd_smaller = self.parser.parse("make window smaller")
        b_smaller = self.engine.compute_resize(self.initial_bounds, cmd_smaller)
        self.assertEqual(b_smaller["width"], 700)
        self.assertEqual(b_smaller["height"], 500)

    def test_delta_wider_and_narrower_partitions(self):
        """Partition: Delta wider and narrower adjusts horizontal dimension exclusively."""
        cmd_wider = self.parser.parse("make window wider")
        b_wider = self.engine.compute_resize(self.initial_bounds, cmd_wider)
        self.assertEqual(b_wider["width"], 900)
        self.assertEqual(b_wider["height"], 600)

        cmd_narrower = self.parser.parse("make window narrower")
        b_narrower = self.engine.compute_resize(self.initial_bounds, cmd_narrower)
        self.assertEqual(b_narrower["width"], 700)
        self.assertEqual(b_narrower["height"], 600)

    def test_delta_taller_and_shorter_partitions(self):
        """Partition: Delta taller and shorter adjusts vertical dimension exclusively."""
        cmd_taller = self.parser.parse("make window taller")
        b_taller = self.engine.compute_resize(self.initial_bounds, cmd_taller)
        self.assertEqual(b_taller["width"], 800)
        self.assertEqual(b_taller["height"], 700)

        cmd_shorter = self.parser.parse("make window shorter")
        b_shorter = self.engine.compute_resize(self.initial_bounds, cmd_shorter)
        self.assertEqual(b_shorter["width"], 800)
        self.assertEqual(b_shorter["height"], 500)

    def test_percentage_scaling_partition(self):
        """Partition: Percentage scaling (50%, 80%) recalculates dimensions from display resolution."""
        cmd_50 = self.parser.parse("resize window 50%")
        b_50 = self.engine.compute_resize(self.initial_bounds, cmd_50)
        self.assertEqual(b_50["width"], 960)
        self.assertEqual(b_50["height"], 540)

    def test_explicit_dimensions_partition(self):
        """Partition: Explicit dimensions command sets exact target resolution (e.g. 1280x720)."""
        cmd_exp = self.parser.parse("resize 1280 by 720")
        b_exp = self.engine.compute_resize(self.initial_bounds, cmd_exp)
        self.assertEqual(b_exp["width"], 1280)
        self.assertEqual(b_exp["height"], 720)
