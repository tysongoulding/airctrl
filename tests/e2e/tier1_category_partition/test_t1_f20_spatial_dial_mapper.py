import math
import unittest
from tests.e2e.e2e_harness import SpatialRadialDialMapper


class TestT1F20SpatialDialMapper(unittest.TestCase):
    """
    Feature 20: Spatial Radial Dial Mapper (Equivalence Partitions)
    """

    def setUp(self):
        self.mapper = SpatialRadialDialMapper(sector_count=6, center=(0.5, 0.5), deadzone=0.10)

    def test_center_deadzone_partition(self):
        """Partition: Fingertip near center (0.5, 0.5) is inside deadzone."""
        res = self.mapper.map_coords(0.50, 0.51)
        self.assertTrue(res["in_deadzone"])
        self.assertIsNone(res["active_sector"])

    def test_sector_zero_partition(self):
        """Partition: Fingertip to the right (angle ~0) maps to sector 0."""
        res = self.mapper.map_coords(0.85, 0.50)  # dx = +0.35, dy = 0.0 -> angle = 0.0
        self.assertFalse(res["in_deadzone"])
        self.assertEqual(res["active_sector"], 0)

    def test_sector_two_partition(self):
        """Partition: Fingertip to the left (angle ~pi) maps to sector 3."""
        res = self.mapper.map_coords(0.15, 0.50)  # dx = -0.35, dy = 0.0 -> angle = pi (~3.14)
        self.assertFalse(res["in_deadzone"])
        self.assertEqual(res["active_sector"], 3)

    def test_all_6_sectors_covered(self):
        """Partition: 6 distinct sector angles produce sectors 0, 1, 2, 3, 4, 5."""
        sectors_found = set()
        for i in range(6):
            angle = (i * 2 * math.pi / 6) + 0.1  # slightly inside sector
            x = 0.5 + 0.3 * math.cos(angle)
            y = 0.5 + 0.3 * math.sin(angle)
            res = self.mapper.map_coords(x, y)
            sectors_found.add(res["active_sector"])
        self.assertEqual(sectors_found, {0, 1, 2, 3, 4, 5})

    def test_normalized_radius_clamped_to_one(self):
        """Partition: Coordinates far from center clamped to normalized radius 1.0."""
        res = self.mapper.map_coords(1.0, 1.0)
        self.assertEqual(res["normalized_radius"], 1.0)
