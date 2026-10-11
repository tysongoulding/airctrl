import unittest
from tests.e2e.e2e_harness import GestureClassifier, Landmark3D, create_mock_hand_landmarks


class TestT2F19GestureClassifier(unittest.TestCase):
    """
    Feature 19: Landmark Gesture Classifier (Boundary Value Analysis)
    """

    def setUp(self):
        self.classifier = GestureClassifier()

    def test_pinch_distance_boundary_at_threshold_0_050(self):
        """BVA: Euclidean distance between thumb and index tips exactly 0.050 classifies as Pinch."""
        landmarks = create_mock_hand_landmarks("Neutral")
        landmarks[4] = Landmark3D(0.50, 0.50, 0.0)
        landmarks[8] = Landmark3D(0.55, 0.50, 0.0)  # distance = 0.050
        self.assertEqual(self.classifier.classify(landmarks), "Pinch")

    def test_pinch_distance_boundary_below_threshold_0_049(self):
        """BVA: Euclidean distance 0.049 (< 0.05) classifies as Pinch."""
        landmarks = create_mock_hand_landmarks("Neutral")
        landmarks[4] = Landmark3D(0.50, 0.50, 0.0)
        landmarks[8] = Landmark3D(0.549, 0.50, 0.0)  # distance = 0.049
        self.assertEqual(self.classifier.classify(landmarks), "Pinch")

    def test_pinch_distance_boundary_above_threshold_0_051(self):
        """BVA: Euclidean distance 0.051 (> 0.05) does not classify as Pinch."""
        landmarks = create_mock_hand_landmarks("Neutral")
        landmarks[4] = Landmark3D(0.50, 0.50, 0.0)
        landmarks[8] = Landmark3D(0.551, 0.50, 0.0)  # distance = 0.051
        self.assertNotEqual(self.classifier.classify(landmarks), "Pinch")

    def test_open_palm_extension_boundary_0_449(self):
        """BVA: All fingertips y at 0.449 (< 0.45) classifies as OpenPalm."""
        landmarks = create_mock_hand_landmarks("Neutral")
        for idx in [4, 8, 12, 16, 20]:
            landmarks[idx] = Landmark3D(0.1 * idx, 0.449, 0.0)
        self.assertEqual(self.classifier.classify(landmarks), "OpenPalm")

    def test_open_palm_extension_boundary_0_451(self):
        """BVA: Fingertips y at 0.451 (> 0.45) does not classify as OpenPalm."""
        landmarks = create_mock_hand_landmarks("Neutral")
        for idx in [4, 8, 12, 16, 20]:
            landmarks[idx] = Landmark3D(0.1 * idx, 0.451, 0.0)
        self.assertNotEqual(self.classifier.classify(landmarks), "OpenPalm")
