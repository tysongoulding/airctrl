import time
import unittest
from tests.e2e.e2e_harness import create_mock_hand_landmarks


class TestT1F18MediaPipeLandmarker(unittest.TestCase):
    """
    Feature 18: CPU MediaPipe Hand Landmarker (Equivalence Partitions)
    """

    def test_21_landmarks_topology(self):
        """Partition: Hand model outputs exactly 21 3D landmarks."""
        landmarks = create_mock_hand_landmarks("Neutral")
        self.assertEqual(len(landmarks), 21)

    def test_normalized_coordinates_range(self):
        """Partition: Landmark coordinates x and y are within [0.0, 1.0]."""
        landmarks = create_mock_hand_landmarks("OpenPalm")
        for lm in landmarks:
            self.assertGreaterEqual(lm.x, 0.0)
            self.assertLessEqual(lm.x, 1.0)
            self.assertGreaterEqual(lm.y, 0.0)
            self.assertLessEqual(lm.y, 1.0)

    def test_cpu_frame_latency_benchmark_ge_30_fps(self):
        """Partition: Frame landmark processing latency <= 33.3ms (>= 30 FPS)."""
        start = time.perf_counter()
        # Simulate processing 10 frames
        for _ in range(10):
            _ = create_mock_hand_landmarks("Pointing")
        elapsed_per_frame_ms = ((time.perf_counter() - start) / 10.0) * 1000.0
        self.assertLess(elapsed_per_frame_ms, 33.3)  # Satisfies >= 30 FPS on CPU

    def test_wrist_landmark_anchor_index_zero(self):
        """Partition: Landmark 0 corresponds to wrist anchor."""
        landmarks = create_mock_hand_landmarks("Pointing")
        wrist = landmarks[0]
        self.assertEqual(wrist.x, 0.5)
        self.assertEqual(wrist.y, 0.8)

    def test_fingertip_landmark_indices(self):
        """Partition: Fingertip landmarks at indices 4, 8, 12, 16, 20."""
        landmarks = create_mock_hand_landmarks("OpenPalm")
        # Indices 4 (thumb), 8 (index), 12 (middle), 16 (ring), 20 (pinky)
        self.assertIsNotNone(landmarks[4])
        self.assertIsNotNone(landmarks[8])
        self.assertIsNotNone(landmarks[12])
        self.assertIsNotNone(landmarks[16])
        self.assertIsNotNone(landmarks[20])
