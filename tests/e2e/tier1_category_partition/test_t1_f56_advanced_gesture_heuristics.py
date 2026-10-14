import unittest
from tests.e2e.e2e_harness import (
    AdvancedGestureHeuristics,
    Landmark3D,
    create_mock_hand_landmarks,
)


class TestT1F56AdvancedGestureHeuristics(unittest.TestCase):
    """
    Feature 56: Advanced Gesture Heuristics (Equivalence Partitions)
    """

    def setUp(self):
        self.heuristics = AdvancedGestureHeuristics()

    def test_hysteresis_pinch_engage_at_005(self):
        """Partition: Thumb-index distance <= 0.05 engages pinch state."""
        t_tip = Landmark3D(0.50, 0.50, 0.0)
        i_tip = Landmark3D(0.53, 0.50, 0.0)  # distance = 0.03 <= 0.05
        is_pinching = self.heuristics.evaluate_pinch(t_tip, i_tip)
        self.assertTrue(is_pinching)

    def test_hysteresis_pinch_release_at_007(self):
        """Partition: Hysteresis retains engaged pinch at d=0.06; releases only when d > 0.07."""
        t_tip = Landmark3D(0.50, 0.50, 0.0)
        i_tip_close = Landmark3D(0.53, 0.50, 0.0)
        self.heuristics.evaluate_pinch(t_tip, i_tip_close)  # engage

        # Separate slightly to d = 0.06 (should stay engaged due to hysteresis)
        i_tip_hyst = Landmark3D(0.56, 0.50, 0.0)
        self.assertTrue(self.heuristics.evaluate_pinch(t_tip, i_tip_hyst))

        # Separate beyond 0.07 (d = 0.08) -> releases
        i_tip_open = Landmark3D(0.58, 0.50, 0.0)
        self.assertFalse(self.heuristics.evaluate_pinch(t_tip, i_tip_open))

    def test_pointing_ray_vector_computation(self):
        """Partition: Pointing ray calculates normalized 3D unit vector from PIP to Tip."""
        pip = Landmark3D(0.50, 0.50, 0.0)
        tip = Landmark3D(0.50, 0.20, 0.0)  # pointing directly upward
        ray = self.heuristics.compute_pointing_ray(pip, tip)
        self.assertEqual(ray["dx"], 0.0)
        self.assertEqual(ray["dy"], -1.0)
        self.assertEqual(ray["dz"], 0.0)

    def test_directional_swipe_right_and_left(self):
        """Partition: Rapid lateral displacement detects swipe events."""
        swipe_r = self.heuristics.detect_swipe(prev_x=0.20, curr_x=0.45, timestamp=1.0)
        self.assertEqual(swipe_r, "SwipeRight")

        swipe_l = self.heuristics.detect_swipe(prev_x=0.60, curr_x=0.35, timestamp=2.0)
        self.assertEqual(swipe_l, "SwipeLeft")

    def test_two_finger_scroll_gesture(self):
        """Partition: Two adjacent fingers calculate vertical scroll center position."""
        i_tip = Landmark3D(0.50, 0.40, 0.0)
        m_tip = Landmark3D(0.52, 0.40, 0.0)  # close together
        scroll_pos = self.heuristics.evaluate_two_finger_scroll(i_tip, m_tip)
        self.assertAlmostEqual(scroll_pos, 0.40)
