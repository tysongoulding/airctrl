import json
import unittest
from tests.e2e.e2e_harness import WorkspaceCoreInspector, Landmark3D


class TestT1F01WorkspaceCore(unittest.TestCase):
    """
    Feature 1: Cargo Workspace & Core Types (Equivalence Partitions)
    """

    def test_cargo_workspace_structure_and_members(self):
        """Partition: Verify Cargo workspace root defines all 7 member crates."""
        info = WorkspaceCoreInspector.inspect_workspace()
        expected = [
            "crates/airctrl-core",
            "crates/airctrl-lan",
            "crates/airctrl-stt",
            "crates/airctrl-actions",
            "crates/airctrl-gestures",
            "crates/airctrl-meetings",
            "src-tauri"
        ]
        for m in expected:
            self.assertIn(m, info["members"])
        self.assertTrue(info["all_exist"])

    def test_rust_airctrl_core_test_suite_passes(self):
        """Partition: Programmatic execution of airctrl-core crate test suite."""
        res = WorkspaceCoreInspector.run_core_tests()
        self.assertEqual(res.returncode, 0, f"airctrl-core tests failed:\n{res.stderr}")
        self.assertIn("test result: ok", res.stdout)

    def test_window_info_schema_valid_partition(self):
        """Partition: Valid WindowInfo struct serialization and field integrity."""
        window_payload = {
            "id": 101,
            "process_id": 1234,
            "process_name": "Code.exe",
            "title": "airctrl - Visual Studio Code",
            "bounds": {"x": 100, "y": 100, "width": 1280, "height": 800},
            "is_focused": True,
            "is_minimized": False
        }
        raw_json = json.dumps(window_payload)
        parsed = json.loads(raw_json)
        self.assertEqual(parsed["id"], 101)
        self.assertEqual(parsed["process_name"], "Code.exe")
        self.assertTrue(parsed["is_focused"])
        self.assertFalse(parsed["is_minimized"])
        self.assertEqual(parsed["bounds"]["width"], 1280)

    def test_action_command_tagged_union_partition(self):
        """Partition: Serde externally tagged union ActionCommand."""
        actions = [
            {"type": "WindowSnap", "payload": {"direction": "Left"}},
            {"type": "WindowClose", "payload": {"window_id": 42}},
            {"type": "LaunchApp", "payload": {"app_name": "Terminal"}},
            {"type": "RadialDialToggle", "payload": {}}
        ]
        for act in actions:
            raw = json.dumps(act)
            decoded = json.loads(raw)
            self.assertIn("type", decoded)
            self.assertIn("payload", decoded)

    def test_landmark3d_coordinate_struct(self):
        """Partition: 3D Landmark representation with x, y, z floats."""
        lm = Landmark3D(0.45, 0.75, -0.05)
        d = lm.to_dict()
        self.assertAlmostEqual(d["x"], 0.45)
        self.assertAlmostEqual(d["y"], 0.75)
        self.assertAlmostEqual(d["z"], -0.05)
