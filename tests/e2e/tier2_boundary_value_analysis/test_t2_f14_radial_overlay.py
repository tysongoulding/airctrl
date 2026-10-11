import unittest
from tests.e2e.tier1_category_partition.test_t1_f14_radial_overlay import MockRadialDialOverlay


class TestT2F14RadialOverlay(unittest.TestCase):
    """
    Feature 14: Radial Dial Pointer Overlay (Boundary Value Analysis)
    """

    def setUp(self):
        self.dial = MockRadialDialOverlay()

    def test_sector_lower_boundary_0(self):
        """BVA: Sector index 0 is valid."""
        self.dial.toggle()
        self.dial.hover_sector(0)
        self.assertEqual(self.dial.active_sector, 0)

    def test_sector_upper_boundary_5(self):
        """BVA: Sector index 5 (last sector for 6 sectors) is valid."""
        self.dial.toggle()
        self.dial.hover_sector(5)
        self.assertEqual(self.dial.active_sector, 5)

    def test_sector_off_by_one_below_negative_one(self):
        """BVA: Sector index -1 is rejected."""
        self.dial.toggle()
        self.dial.hover_sector(-1)
        self.assertIsNone(self.dial.active_sector)

    def test_sector_off_by_one_above_six(self):
        """BVA: Sector index 6 is rejected."""
        self.dial.toggle()
        self.dial.hover_sector(6)
        self.assertIsNone(self.dial.active_sector)

    def test_inactivity_timeout_duration_boundary_3_0s(self):
        """BVA: Inactivity timeout boundary is 3.0 seconds."""
        self.assertEqual(self.dial.timeout_secs, 3.0)
