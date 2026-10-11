import unittest
from tests.e2e.e2e_harness import SpatialRadialDialMapper


class TestT2F20SpatialDialMapper(unittest.TestCase):
    """
    Feature 20: Spatial Radial Dial Mapper (Boundary Value Analysis)
    """

    def setUp(self):
        self.mapper = SpatialRadialDialMapper(sector_count=6, center=(0.5, 0.5), deadzone=0.10)

    def test_deadzone_boundary_at_0_099_in_deadzone(self):
        """BVA: Normalized radius 0.099 (< 0.10) is inside deadzone."""
        res = self.mapper.map_coords(0.50, 0.549)  # dy = 0.049 -> radius/0.5 = 0.098
        self.assertTrue(res["in_deadzone"])
        self.assertIsNone(res["active_sector"])

    def test_deadzone_boundary_at_0_101_active(self):
        """BVA: Normalized radius 0.101 (>= 0.10) activates sector."""
        res = self.mapper.map_coords(0.50, 0.552)  # dy = 0.052 -> radius/0.5 = 0.104
        self.assertFalse(res["in_deadzone"])
        self.assertIsNotNone(res["active_sector"])

    def test_exact_center_coordinates_0_5_0_5(self):
        """BVA: Exactly at center (0.5, 0.5) has radius 0.0 and in deadzone."""
        res = self.mapper.map_coords(0.5, 0.5)
        self.assertTrue(res["in_deadzone"])
        self.assertEqual(res["normalized_radius"], 0.0)

    def test_sector_boundary_at_angle_zero(self):
        """BVA: Angle 0.0 (directly right) belongs to sector 0."""
        res = self.mapper.map_coords(0.9, 0.5)  # dx = +0.4, dy = 0.0 -> angle = 0.0
        self.assertEqual(res["active_sector"], 0)

    def test_outer_perimeter_clamping_at_radius_one(self):
        """BVA: Far outside window coordinates clamp normalized radius to 1.0."""
        res = self.mapper.map_coords(2.0, 2.0)
        self.assertEqual(res["normalized_radius"], 1.0)
