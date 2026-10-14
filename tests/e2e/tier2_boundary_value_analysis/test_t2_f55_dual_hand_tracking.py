import unittest
from tests.e2e.e2e_harness import Landmark3D, MultiHandLandmarks


class TestT2F55DualHandTracking(unittest.TestCase):
    """
    Feature 55: Dual-Hand 21-Landmark CPU Tracking (Boundary Value Analysis)
    """

    def test_landmark_coordinates_at_screen_corners_0_and_1(self):
        """BVA: Landmark coordinates at extreme screen borders (0.0 and 1.0)."""
        corner_lm = [Landmark3D(0.0, 0.0, 0.0)] * 10 + [Landmark3D(1.0, 1.0, 0.0)] * 11
        multi = MultiHandLandmarks(left_hand=corner_lm)
        self.assertEqual(multi.hand_count, 1)
        self.assertEqual(multi.left_hand[0].x, 0.0)
        self.assertEqual(multi.left_hand[10].x, 1.0)

    def test_negative_depth_z_coordinates_boundary(self):
        """BVA: Negative z coordinates (hand protruding closer to lens) handled accurately."""
        close_hand = [Landmark3D(0.5, 0.5, -0.75)] * 21
        multi = MultiHandLandmarks(right_hand=close_hand)
        self.assertEqual(multi.right_hand[0].z, -0.75)

    def test_positive_depth_z_coordinates_boundary(self):
        """BVA: Positive z coordinates (hand recessed farther from lens) handled accurately."""
        far_hand = [Landmark3D(0.5, 0.5, +0.95)] * 21
        multi = MultiHandLandmarks(right_hand=far_hand)
        self.assertEqual(multi.right_hand[0].z, 0.95)

    def test_truncated_landmark_list_tolerance(self):
        """BVA: Incomplete landmark array (< 21 landmarks) does not crash hand count."""
        partial = [Landmark3D(0.5, 0.5, 0.0)] * 15
        multi = MultiHandLandmarks(left_hand=partial)
        self.assertEqual(multi.hand_count, 1)
        self.assertEqual(len(multi.left_hand), 15)

    def test_rapid_hand_detection_transitions(self):
        """BVA: Rapid frame transitions (0 hands -> 2 hands -> 1 hand -> 0 hands)."""
        hand = [Landmark3D(0.5, 0.5, 0.0)] * 21
        m0 = MultiHandLandmarks(None, None)
        self.assertEqual(m0.hand_count, 0)

        m2 = MultiHandLandmarks(hand, hand)
        self.assertEqual(m2.hand_count, 2)

        m1 = MultiHandLandmarks(hand, None)
        self.assertEqual(m1.hand_count, 1)
