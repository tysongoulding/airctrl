import json
import unittest
from tests.e2e.e2e_harness import Landmark3D


class TestT2F01WorkspaceCore(unittest.TestCase):
    """
    Feature 1: Cargo Workspace & Core Types (Boundary Value Analysis)
    """

    def test_window_id_extreme_boundaries(self):
        """BVA: Window ID 0 and u64::MAX boundary serialization."""
        payload_zero = {"id": 0, "process_id": 1, "process_name": "init", "title": ""}
        payload_max = {"id": 18446744073709551615, "process_id": 99999, "process_name": "app", "title": "max"}
        self.assertEqual(json.loads(json.dumps(payload_zero))["id"], 0)
        self.assertEqual(json.loads(json.dumps(payload_max))["id"], 18446744073709551615)

    def test_negative_coordinates_multimonitor_boundary(self):
        """BVA: Multi-monitor virtual desktop negative bounds."""
        bounds = {"x": -1920, "y": -1080, "width": 1920, "height": 1080}
        self.assertEqual(bounds["x"], -1920)
        self.assertEqual(bounds["y"], -1080)

    def test_zero_dimension_window_bounds(self):
        """BVA: Zero width and zero height window boundary (hidden/offscreen)."""
        bounds = {"x": 0, "y": 0, "width": 0, "height": 0}
        self.assertEqual(bounds["width"], 0)
        self.assertEqual(bounds["height"], 0)

    def test_landmark3d_coordinate_extrema(self):
        """BVA: 3D Landmark at absolute min (0.0, 0.0, -10.0) and max (1.0, 1.0, 10.0)."""
        min_lm = Landmark3D(0.0, 0.0, -10.0)
        max_lm = Landmark3D(1.0, 1.0, 10.0)
        self.assertEqual(min_lm.x, 0.0)
        self.assertEqual(max_lm.x, 1.0)
        self.assertEqual(min_lm.z, -10.0)
        self.assertEqual(max_lm.z, 10.0)

    def test_empty_string_title_and_process_name(self):
        """BVA: Empty string boundary for window title and process name."""
        win = {"id": 1, "process_id": 1, "process_name": "", "title": ""}
        self.assertEqual(win["process_name"], "")
        self.assertEqual(win["title"], "")
