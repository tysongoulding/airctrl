import unittest
from tests.e2e.e2e_harness import HandsFreeCommandParser, SnapPosition, WindowGeometryEngine


class TestT1F429WayWindowSnapping(unittest.TestCase):
    """
    Feature 42: 9-Way Window Snapping & Docking (Equivalence Partitions)
    """

    def setUp(self):
        self.parser = HandsFreeCommandParser()
        self.engine = WindowGeometryEngine(1920, 1080)

    def test_snap_left_and_right_partitions(self):
        """Partition: Snapping left and right allocates exact half-screen width."""
        cmd_left = self.parser.parse("snap left")
        self.assertEqual(cmd_left.payload["position"], SnapPosition.LEFT.value)
        bounds_left = self.engine.compute_snap(SnapPosition.LEFT)
        self.assertEqual(bounds_left, {"x": 0, "y": 0, "width": 960, "height": 1080})

        bounds_right = self.engine.compute_snap(SnapPosition.RIGHT)
        self.assertEqual(bounds_right, {"x": 960, "y": 0, "width": 960, "height": 1080})

    def test_snap_top_and_bottom_partitions(self):
        """Partition: Snapping top and bottom allocates exact half-screen height."""
        bounds_top = self.engine.compute_snap(SnapPosition.TOP)
        self.assertEqual(bounds_top, {"x": 0, "y": 0, "width": 1920, "height": 540})

        bounds_bottom = self.engine.compute_snap(SnapPosition.BOTTOM)
        self.assertEqual(bounds_bottom, {"x": 0, "y": 540, "width": 1920, "height": 540})

    def test_snap_center_partition(self):
        """Partition: Center command docks 70% viewport centered on display."""
        bounds_center = self.engine.compute_snap(SnapPosition.CENTER)
        expected_w = int(1920 * 0.70)
        expected_h = int(1080 * 0.70)
        self.assertEqual(bounds_center["width"], expected_w)
        self.assertEqual(bounds_center["height"], expected_h)
        self.assertEqual(bounds_center["x"], (1920 - expected_w) // 2)

    def test_snap_maximize_and_minimize_partitions(self):
        """Partition: Maximize occupies full desktop; minimize collapses geometry."""
        bounds_max = self.engine.compute_snap(SnapPosition.MAXIMIZE)
        self.assertEqual(bounds_max, {"x": 0, "y": 0, "width": 1920, "height": 1080})

        bounds_min = self.engine.compute_snap(SnapPosition.MINIMIZE)
        self.assertEqual(bounds_min, {"x": 0, "y": 0, "width": 0, "height": 0})

    def test_snap_restore_and_fullscreen_partitions(self):
        """Partition: Restore returns window to floating bounds; fullscreen occupies total display."""
        bounds_restore = self.engine.compute_snap(SnapPosition.RESTORE)
        self.assertEqual(bounds_restore, {"x": 100, "y": 100, "width": 1024, "height": 768})

        bounds_fs = self.engine.compute_snap(SnapPosition.FULLSCREEN)
        self.assertEqual(bounds_fs, {"x": 0, "y": 0, "width": 1920, "height": 1080})
