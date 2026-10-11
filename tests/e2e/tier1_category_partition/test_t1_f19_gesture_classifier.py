import unittest
from tests.e2e.e2e_harness import GestureClassifier, create_mock_hand_landmarks


class TestT1F19GestureClassifier(unittest.TestCase):
    """
    Feature 19: Landmark Gesture Classifier (Equivalence Partitions)
    """

    def setUp(self):
        self.classifier = GestureClassifier()

    def test_classify_pinch_gesture(self):
        """Partition: Thumb and index tips close (dist <= 0.05) classifies as Pinch."""
        landmarks = create_mock_hand_landmarks("Pinch")
        gesture = self.classifier.classify(landmarks)
        self.assertEqual(gesture, "Pinch")

    def test_classify_pointing_gesture(self):
        """Partition: Index tip extended while others curled classifies as Pointing."""
        landmarks = create_mock_hand_landmarks("Pointing")
        gesture = self.classifier.classify(landmarks)
        self.assertEqual(gesture, "Pointing")

    def test_classify_open_palm_gesture(self):
        """Partition: All fingertips extended classifies as OpenPalm."""
        landmarks = create_mock_hand_landmarks("OpenPalm")
        gesture = self.classifier.classify(landmarks)
        self.assertEqual(gesture, "OpenPalm")

    def test_classify_neutral_gesture(self):
        """Partition: Relaxed hand pose classifies as Neutral."""
        landmarks = create_mock_hand_landmarks("Neutral")
        gesture = self.classifier.classify(landmarks)
        self.assertEqual(gesture, "Neutral")

    def test_classify_insufficient_landmarks_returns_neutral(self):
        """Partition: Landmark array < 21 items returns Neutral safely."""
        gesture = self.classifier.classify([])
        self.assertEqual(gesture, "Neutral")
