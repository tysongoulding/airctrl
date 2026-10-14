import unittest
from tests.e2e.e2e_harness import (
    Landmark3D,
    MultiHandLandmarks,
    create_mock_hand_landmarks,
)


class TestT1F55DualHandTracking(unittest.TestCase):
    """
    Feature 55: Dual-Hand 21-Landmark CPU Tracking (Equivalence Partitions)
    """

    def test_single_hand_left_tracking(self):
        """Partition: Left hand only tracking returns 21 landmarks."""
        l_hand = create_mock_hand_landmarks("Neutral")
        multi = MultiHandLandmarks(left_hand=l_hand, right_hand=None)
        self.assertEqual(multi.hand_count, 1)
        self.assertIsNotNone(multi.left_hand)
        self.assertEqual(len(multi.left_hand), 21)

    def test_single_hand_right_tracking(self):
        """Partition: Right hand only tracking returns 21 landmarks."""
        r_hand = create_mock_hand_landmarks("Pointing")
        multi = MultiHandLandmarks(left_hand=None, right_hand=r_hand)
        self.assertEqual(multi.hand_count, 1)
        self.assertIsNotNone(multi.right_hand)
        self.assertEqual(len(multi.right_hand), 21)

    def test_dual_hand_simultaneous_tracking(self):
        """Partition: Simultaneous tracking of up to 2 hands (42 landmarks total)."""
        l_hand = create_mock_hand_landmarks("Neutral")
        r_hand = create_mock_hand_landmarks("Pinch")
        multi = MultiHandLandmarks(left_hand=l_hand, right_hand=r_hand)
        self.assertEqual(multi.hand_count, 2)
        self.assertEqual(len(multi.left_hand) + len(multi.right_hand), 42)

    def test_no_hands_detected_partition(self):
        """Partition: Zero hands in camera frame returns hand_count=0."""
        multi = MultiHandLandmarks(left_hand=None, right_hand=None)
        self.assertEqual(multi.hand_count, 0)

    def test_landmark_3d_coordinates_partition(self):
        """Partition: 3D coordinates verify x, y in [0.0, 1.0] and z float."""
        hand = create_mock_hand_landmarks("Pointing")
        for pt in hand:
            self.assertTrue(0.0 <= pt.x <= 1.0)
            self.assertTrue(0.0 <= pt.y <= 1.0)
            self.assertIsInstance(pt.z, float)
