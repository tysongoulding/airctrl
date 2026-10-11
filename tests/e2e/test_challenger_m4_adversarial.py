"""
test_challenger_m4_adversarial.py - Empirical Adversarial Tests for Milestone M4

Adversarially stress-tests:
1. SpatialRadialDialMapper polar boundaries:
   - Center coordinate (0.5, 0.5): verify radius r = 0.0 < 0.10 results in active_sector = None
   - Deadband boundary fuzzing: r = 0.099 (None), r = 0.100 and r = 0.101 (valid sector)
   - Sector angle boundaries: exact transitions at multiples of pi / 3 (0°, 60°, 120°, 180°, 240°, 300°)
   - Angular wrap-around from 2pi - eps to 0 + eps
   - Screen corner coordinates: (0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0), verifying radius clamping
2. Real-time CPU benchmark stress:
   - Run benchmark on 100+ frames of synthetic video streams
   - Assert throughput satisfies >= 30 FPS (latency <= 33.3ms) and >= 60 FPS (latency <= 16.6ms)
   - Concurrency stress: multi-threaded mapper invocations
"""

import math
import os
import sys
import time
import threading
import unittest
from typing import List, Tuple, Dict, Any

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
if REPO_ROOT not in sys.path:
    sys.path.insert(0, REPO_ROOT)

from tests.e2e.e2e_harness import SpatialRadialDialMapper, Landmark3D, GestureClassifier


class TestChallengerM4SpatialDialMapper(unittest.TestCase):
    """
    Adversarial polar boundary and deadband stress tests.
    """

    def setUp(self):
        self.mapper = SpatialRadialDialMapper(sector_count=6, center=(0.5, 0.5), deadzone=0.10)

    def test_center_coordinate_boundary_and_perturbations(self):
        """1. Exact Center (0.5, 0.5): radius 0.0 < 0.10 -> active_sector = None."""
        res = self.mapper.map_coords(0.5, 0.5)
        self.assertTrue(res["in_deadzone"], "Center must be in deadzone")
        self.assertIsNone(res["active_sector"], "Center must have active_sector = None")
        self.assertEqual(res["normalized_radius"], 0.0, "Center normalized radius must be 0.0")
        self.assertEqual(res["angle_rad"], 0.0, "Center angle must be 0.0")

        # Sub-millimeter perturbations
        perturbations = [
            (0.50001, 0.50000),
            (0.49999, 0.50000),
            (0.50000, 0.50001),
            (0.50000, 0.49999),
            (0.50005, 0.50005),
            (0.52000, 0.50000),  # radius = 0.02 / 0.5 = 0.04 < 0.10
        ]
        for px, py in perturbations:
            p_res = self.mapper.map_coords(px, py)
            self.assertTrue(p_res["in_deadzone"], f"({px}, {py}) must be in deadzone")
            self.assertIsNone(p_res["active_sector"], f"({px}, {py}) must have active_sector = None")
            self.assertLess(p_res["normalized_radius"], 0.10)

    def test_deadband_boundary_fuzzing(self):
        """2. Deadband boundary: r=0.099 (None), r=0.100 and r=0.101 (valid sector)."""
        # dx = r * 0.5
        # r = 0.099 -> dx = 0.0495 -> x = 0.5495
        res_0099 = self.mapper.map_coords(0.5495, 0.5000)
        self.assertTrue(res_0099["in_deadzone"], f"r=0.099 must be inside deadzone (was {res_0099['normalized_radius']})")
        self.assertIsNone(res_0099["active_sector"], "r=0.099 must have active_sector = None")
        self.assertEqual(res_0099["normalized_radius"], 0.099)

        # r = 0.100 -> dx = 0.0500 -> x = 0.5500
        res_0100 = self.mapper.map_coords(0.5500, 0.5000)
        self.assertFalse(res_0100["in_deadzone"], f"r=0.100 must activate sector (was {res_0100['normalized_radius']})")
        self.assertIsNotNone(res_0100["active_sector"], "r=0.100 must have valid sector")
        self.assertEqual(res_0100["active_sector"], 0, "r=0.100 at angle 0 must be Sector 0")
        self.assertEqual(res_0100["normalized_radius"], 0.100)

        # r = 0.101 -> dx = 0.0505 -> x = 0.5505
        res_0101 = self.mapper.map_coords(0.5505, 0.5000)
        self.assertFalse(res_0101["in_deadzone"], f"r=0.101 must activate sector (was {res_0101['normalized_radius']})")
        self.assertIsNotNone(res_0101["active_sector"], "r=0.101 must have valid sector")
        self.assertEqual(res_0101["active_sector"], 0, "r=0.101 at angle 0 must be Sector 0")
        self.assertEqual(res_0101["normalized_radius"], 0.101)

        # Fuzzing across 16 angles around the perimeter
        for step in range(16):
            angle = step * (2.0 * math.pi / 16.0)
            cos_a = math.cos(angle)
            sin_a = math.sin(angle)

            # Inside deadzone r = 0.099
            x_in = 0.5 + (0.099 * 0.5) * cos_a
            y_in = 0.5 + (0.099 * 0.5) * sin_a
            s_in = self.mapper.map_coords(x_in, y_in)
            self.assertTrue(s_in["in_deadzone"], f"Angle {angle:.3f} at r=0.099 must be inside deadzone")
            self.assertIsNone(s_in["active_sector"])

            # Outside deadzone r = 0.101
            x_out = 0.5 + (0.101 * 0.5) * cos_a
            y_out = 0.5 + (0.101 * 0.5) * sin_a
            s_out = self.mapper.map_coords(x_out, y_out)
            self.assertFalse(s_out["in_deadzone"], f"Angle {angle:.3f} at r=0.101 must be outside deadzone")
            self.assertIsNotNone(s_out["active_sector"])

    def test_sector_angle_exact_transitions_at_multiples_of_pi_over_3(self):
        """3. Exact transitions at multiples of pi / 3 (0°, 60°, 120°, 180°, 240°, 300°)."""
        r = 0.35
        eps = 1e-4

        boundaries = [
            (0, 0.0 * math.pi / 3.0),
            (1, 1.0 * math.pi / 3.0),
            (2, 2.0 * math.pi / 3.0),
            (3, 3.0 * math.pi / 3.0),
            (4, 4.0 * math.pi / 3.0),
            (5, 5.0 * math.pi / 3.0),
        ]

        for target_sector, boundary_rad in boundaries:
            # 1. Immediately before boundary (boundary_rad - eps) -> previous sector
            angle_before = (2.0 * math.pi - eps) if boundary_rad == 0.0 else (boundary_rad - eps)
            expected_before = (target_sector + 5) % 6
            x_b = 0.5 + r * math.cos(angle_before)
            y_b = 0.5 + r * math.sin(angle_before)
            res_b = self.mapper.map_coords(x_b, y_b)
            self.assertEqual(
                res_b["active_sector"],
                expected_before,
                f"Angle {angle_before:.5f} before boundary #{target_sector} expected {expected_before}, got {res_b['active_sector']}",
            )

            # 2. Immediately after boundary (boundary_rad + eps) -> target sector
            angle_after = boundary_rad + eps
            x_a = 0.5 + r * math.cos(angle_after)
            y_a = 0.5 + r * math.sin(angle_after)
            res_a = self.mapper.map_coords(x_a, y_a)
            self.assertEqual(
                res_a["active_sector"],
                target_sector,
                f"Angle {angle_after:.5f} after boundary #{target_sector} expected {target_sector}, got {res_a['active_sector']}",
            )

    def test_angular_wrap_around_2pi_minus_eps_to_zero_plus_eps(self):
        """4. Angular wrap-around from 2pi - eps to 0 + eps."""
        r = 0.35
        eps = 1e-4

        # Angle 2pi - eps -> Sector 5
        x_5 = 0.5 + r * math.cos(2.0 * math.pi - eps)
        y_5 = 0.5 + r * math.sin(2.0 * math.pi - eps)
        res_5 = self.mapper.map_coords(x_5, y_5)
        self.assertEqual(res_5["active_sector"], 5, "2pi - eps must map to Sector 5")

        # Angle 0 + eps -> Sector 0
        x_0 = 0.5 + r * math.cos(eps)
        y_0 = 0.5 + r * math.sin(eps)
        res_0 = self.mapper.map_coords(x_0, y_0)
        self.assertEqual(res_0["active_sector"], 0, "0 + eps must map to Sector 0")

        # Rapid oscillation back and forth across 2pi <-> 0 branch cut
        for cycle in range(50):
            s5 = self.mapper.map_coords(0.5 + r * math.cos(2.0 * math.pi - eps), 0.5 + r * math.sin(2.0 * math.pi - eps))
            self.assertEqual(s5["active_sector"], 5)

            s0 = self.mapper.map_coords(0.5 + r * math.cos(eps), 0.5 + r * math.sin(eps))
            self.assertEqual(s0["active_sector"], 0)

    def test_screen_corner_coordinates_and_radius_clamping(self):
        """5. Screen corner coordinates (0,0), (1,0), (0,1), (1,1) radius clamping."""
        # Top-Left: (0.0, 0.0) -> Sector 3
        tl = self.mapper.map_coords(0.0, 0.0)
        self.assertFalse(tl["in_deadzone"])
        self.assertEqual(tl["normalized_radius"], 1.0)
        self.assertEqual(tl["active_sector"], 3)

        # Top-Right: (1.0, 0.0) -> Sector 5
        tr = self.mapper.map_coords(1.0, 0.0)
        self.assertFalse(tr["in_deadzone"])
        self.assertEqual(tr["normalized_radius"], 1.0)
        self.assertEqual(tr["active_sector"], 5)

        # Bottom-Left: (0.0, 1.0) -> Sector 2
        bl = self.mapper.map_coords(0.0, 1.0)
        self.assertFalse(bl["in_deadzone"])
        self.assertEqual(bl["normalized_radius"], 1.0)
        self.assertEqual(bl["active_sector"], 2)

        # Bottom-Right: (1.0, 1.0) -> Sector 0
        br = self.mapper.map_coords(1.0, 1.0)
        self.assertFalse(br["in_deadzone"])
        self.assertEqual(br["normalized_radius"], 1.0)
        self.assertEqual(br["active_sector"], 0)

        # Far outside coordinates
        for ex, ey in [(-10.0, -10.0), (100.0, 100.0), (5.0, -5.0)]:
            ext = self.mapper.map_coords(ex, ey)
            self.assertFalse(ext["in_deadzone"])
            self.assertEqual(ext["normalized_radius"], 1.0)
            self.assertIsNotNone(ext["active_sector"])


class TestChallengerM4RealTimeCpuBenchmark(unittest.TestCase):
    """
    Real-time CPU benchmark stress & concurrency verification.
    """

    def test_synthetic_stream_cpu_benchmark_100_plus_frames(self):
        """Benchmark 100+ frames of synthetic hand tracking; assert >= 30 FPS and >= 60 FPS."""
        frame_counts = [100, 250, 500]
        classifier = GestureClassifier()

        for total_frames in frame_counts:
            # Generate 100+ synthetic frames (mock 21 landmarks pointing pose)
            latencies_ms = []
            start_total = time.perf_counter()

            for i in range(total_frames):
                angle = (i / total_frames) * 2.0 * math.pi
                landmarks = [Landmark3D(0.5, 0.7, 0.0) for _ in range(21)]
                # Pointing pose: index tip extended, middle/ring curled
                landmarks[8] = Landmark3D(0.5 + 0.3 * math.cos(angle), 0.25, -0.1)
                landmarks[12] = Landmark3D(0.5, 0.65, 0.0)
                landmarks[16] = Landmark3D(0.5, 0.65, 0.0)

                t0 = time.perf_counter()
                gesture = classifier.classify(landmarks)
                elapsed_ms = (time.perf_counter() - t0) * 1000.0
                latencies_ms.append(elapsed_ms)
                self.assertEqual(gesture, "Pointing")

            total_duration_sec = time.perf_counter() - start_total
            effective_fps = total_frames / total_duration_sec
            mean_latency_ms = sum(latencies_ms) / total_frames

            # Assert >= 30 FPS (latency <= 33.3ms)
            self.assertGreaterEqual(effective_fps, 30.0, f"Throughput {effective_fps:.1f} FPS must be >= 30 FPS")
            self.assertLessEqual(mean_latency_ms, 33.33, f"Mean latency {mean_latency_ms:.3f}ms must be <= 33.33ms")

            # Assert >= 60 FPS (latency <= 16.6ms)
            self.assertGreaterEqual(effective_fps, 60.0, f"Throughput {effective_fps:.1f} FPS must be >= 60 FPS")
            self.assertLessEqual(mean_latency_ms, 16.67, f"Mean latency {mean_latency_ms:.3f}ms must be <= 16.67ms")

    def test_multi_threaded_concurrency_stress(self):
        """Multi-threaded concurrent mapper invocations stress test."""
        num_threads = 16
        iterations_per_thread = 2000
        errors = []

        def worker(tid: int):
            try:
                mapper = SpatialRadialDialMapper(sector_count=6, center=(0.5, 0.5), deadzone=0.10)
                for i in range(iterations_per_thread):
                    angle = (i * 0.01) % (2.0 * math.pi)
                    radius = 0.05 + ((i % 100) / 100.0) * 0.45
                    x = 0.5 + radius * math.cos(angle)
                    y = 0.5 + radius * math.sin(angle)

                    res = mapper.map_coords(x, y)
                    if res["in_deadzone"]:
                        if res["active_sector"] is not None:
                            errors.append(f"Thread {tid} iter {i}: deadzone with active sector")
                    else:
                        if res["active_sector"] is None or res["active_sector"] not in range(6):
                            errors.append(f"Thread {tid} iter {i}: invalid sector {res['active_sector']}")
            except Exception as e:
                errors.append(f"Thread {tid} exception: {e}")

        threads = [threading.Thread(target=worker, args=(t,)) for t in range(num_threads)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()

        self.assertEqual(len(errors), 0, f"Concurrency errors encountered: {errors[:5]}")


if __name__ == "__main__":
    unittest.main()
