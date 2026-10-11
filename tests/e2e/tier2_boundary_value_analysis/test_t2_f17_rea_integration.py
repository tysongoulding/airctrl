import unittest
from tests.e2e.tier1_category_partition.test_t1_f17_rea_integration import MockReaInspector


class TestT2F17ReaIntegration(unittest.TestCase):
    """
    Feature 17: REA Integration Hook (Boundary Value Analysis)
    """

    def setUp(self):
        self.rea = MockReaInspector()

    def test_pid_lowest_valid_boundary_1(self):
        """BVA: PID 1 inspection."""
        res = self.rea.inspect(1)
        self.assertEqual(res["target_pid"], 1)

    def test_pid_upper_boundary_2147483647(self):
        """BVA: 32-bit signed integer max PID 2,147,483,647."""
        res = self.rea.inspect(2147483647)
        self.assertEqual(res["target_pid"], 2147483647)

    def test_empty_binary_path_defaults(self):
        """BVA: Empty binary path falls back to default binary."""
        res = self.rea.inspect(500, "")
        self.assertEqual(res["binary_path"], "/usr/bin/mock-app")

    def test_binary_path_with_spaces_and_quotes(self):
        """BVA: Binary path with spaces 'C:\\Program Files\\App\\run.exe'."""
        path = "C:\\Program Files\\App\\run.exe"
        res = self.rea.inspect(600, path)
        self.assertEqual(res["binary_path"], path)

    def test_schema_with_zero_interactive_elements(self):
        """BVA: Application without interactable UI controls handled safely."""
        res = self.rea.inspect(700)
        self.assertIsInstance(res["action_schema"]["buttons"], list)
