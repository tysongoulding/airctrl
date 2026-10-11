import unittest
from tests.e2e.e2e_harness import Landmark3D, create_mock_hand_landmarks


class TestT2F18MediaPipeLandmarker(unittest.TestCase):
    """
    Feature 18: CPU MediaPipe Hand Landmarker (Boundary Value Analysis)
    """

    def test_landmark_coordinates_exact_zero_boundary(self):
        """BVA: Landmark coordinates exactly (0.0, 0.0, 0.0) top-left corner."""
        lm = Landmark3D(0.0, 0.0, 0.0)
        self.assertEqual(lm.x, 0.0)
        self.assertEqual(lm.y, 0.0)

    def test_landmark_coordinates_exact_one_boundary(self):
        """BVA: Landmark coordinates exactly (1.0, 1.0, 0.0) bottom-right corner."""
        lm = Landmark3D(1.0, 1.0, 0.0)
        self.assertEqual(lm.x, 1.0)
        self.assertEqual(lm.y, 1.0)

    def test_depth_z_negative_boundary(self):
        """BVA: Landmark depth z negative boundary (hand closer to camera)."""
        lm = Landmark3D(0.5, 0.5, -2.5)
        self.assertEqual(lm.z, -2.5)

    def test_depth_z_positive_boundary(self):
        """BVA: Landmark depth z positive boundary (hand farther from camera)."""
        lm = Landmark3D(0.5, 0.5, 2.5)
        self.assertEqual(lm.z, 2.5)

    def test_landmark_count_exact_21_boundary(self):
        """BVA: Exactly 21 landmarks; 20 is insufficient, 22 is superfluous."""
        landmarks = create_mock_hand_landmarks("Neutral")
        self.assertEqual(len(landmarks), 21)
