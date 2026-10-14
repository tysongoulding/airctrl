import unittest
from tests.e2e.e2e_harness import AdvancedGestureHeuristics, Landmark3D


class TestT2F56AdvancedGestureHeuristics(unittest.TestCase):
    """
    Feature 56: Advanced Gesture Heuristics (Boundary Value Analysis)
    """

    def setUp(self):
        self.heuristics = AdvancedGestureHeuristics()

    def test_hysteresis_pinch_exact_boundary_005(self):
        """BVA: Exactly 0.05 distance engages pinch state."""
        t_tip = Landmark3D(0.50, 0.50, 0.0)
        i_tip_50 = Landmark3D(0.55, 0.50, 0.0)  # distance = 0.05
        self.assertTrue(self.heuristics.evaluate_pinch(t_tip, i_tip_50))

    def test_hysteresis_pinch_exact_boundary_007(self):
        """BVA: Exactly 0.07 distance maintains engaged pinch; 0.0701 releases."""
        t_tip = Landmark3D(0.50, 0.50, 0.0)
        i_tip_close = Landmark3D(0.54, 0.50, 0.0)
        self.heuristics.evaluate_pinch(t_tip, i_tip_close)  # engage

        # At 0.07, stays engaged
        i_tip_70 = Landmark3D(0.57, 0.50, 0.0)  # d = 0.07
        self.assertTrue(self.heuristics.evaluate_pinch(t_tip, i_tip_70))

        # At 0.0701, releases
        i_tip_71 = Landmark3D(0.5701, 0.50, 0.0)
        self.assertFalse(self.heuristics.evaluate_pinch(t_tip, i_tip_71))

    def test_swipe_delta_threshold_boundary_015(self):
        """BVA: Delta x = 0.1501 triggers SwipeRight; 0.1499 does not."""
        self.heuristics.last_swipe_time = 0.0
        no_swipe = self.heuristics.detect_swipe(prev_x=0.20, curr_x=0.3499, timestamp=1.0)
        self.assertIsNone(no_swipe)

        yes_swipe = self.heuristics.detect_swipe(prev_x=0.20, curr_x=0.3501, timestamp=2.0)
        self.assertEqual(yes_swipe, "SwipeRight")

    def test_swipe_cooldown_boundary_03s(self):
        """BVA: Swipe within 0.29s of prior swipe is rejected by cooldown."""
        self.heuristics.detect_swipe(0.20, 0.40, timestamp=10.0)  # first swipe at t=10.0
        quick_swipe = self.heuristics.detect_swipe(0.20, 0.40, timestamp=10.29)  # 0.29s later
        self.assertIsNone(quick_swipe)

        valid_swipe = self.heuristics.detect_swipe(0.20, 0.40, timestamp=10.31)  # 0.31s later
        self.assertEqual(valid_swipe, "SwipeRight")

    def test_two_finger_distance_boundary_008(self):
        """BVA: Distance between fingers < 0.08 yields scroll position; >= 0.08 returns 0.0."""
        i_tip = Landmark3D(0.50, 0.40, 0.0)
        m_close = Landmark3D(0.57, 0.40, 0.0)  # d = 0.07 < 0.08
        self.assertGreater(self.heuristics.evaluate_two_finger_scroll(i_tip, m_close), 0.0)

        m_far = Landmark3D(0.59, 0.40, 0.0)  # d = 0.09 >= 0.08
        self.assertEqual(self.heuristics.evaluate_two_finger_scroll(i_tip, m_far), 0.0)
