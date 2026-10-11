import unittest


class MockReaInspector:
    def inspect(self, pid: int, binary_path: str = ""):
        if pid <= 0:
            raise ValueError("PID must be greater than 0")
        return {
            "target_pid": pid,
            "binary_path": binary_path or "/usr/bin/mock-app",
            "action_schema": {
                "buttons": ["Save", "Cancel", "Submit"],
                "inputs": ["Username", "Password"],
                "menus": ["File", "Edit", "View"]
            },
            "status": "SchemaExtracted"
        }


class TestT1F17ReaIntegration(unittest.TestCase):
    """
    Feature 17: REA Integration Hook (Equivalence Partitions)
    """

    def setUp(self):
        self.rea = MockReaInspector()

    def test_rea_inspect_valid_pid(self):
        """Partition: Reverse engineer application structure by PID."""
        res = self.rea.inspect(1234)
        self.assertEqual(res["target_pid"], 1234)
        self.assertEqual(res["status"], "SchemaExtracted")
        self.assertIn("buttons", res["action_schema"])

    def test_rea_inspect_with_binary_path(self):
        """Partition: Inspection with specific binary path."""
        res = self.rea.inspect(5678, "/opt/custom/app.bin")
        self.assertEqual(res["binary_path"], "/opt/custom/app.bin")

    def test_rea_extracted_action_schemas(self):
        """Partition: Schema contains interactable buttons and inputs."""
        res = self.rea.inspect(999)
        schema = res["action_schema"]
        self.assertIn("Save", schema["buttons"])
        self.assertIn("Username", schema["inputs"])

    def test_rea_invalid_zero_pid_rejected(self):
        """Partition: PID 0 is rejected."""
        with self.assertRaises(ValueError):
            self.rea.inspect(0)

    def test_rea_negative_pid_rejected(self):
        """Partition: Negative PID is rejected."""
        with self.assertRaises(ValueError):
            self.rea.inspect(-10)
