import unittest
from tests.e2e.e2e_harness import HandsFreeCommandParser


class TestT2F41AppLifecycleCommands(unittest.TestCase):
    """
    Feature 41: Voice App Lifecycle Commands (Boundary Value Analysis)
    """

    def setUp(self):
        self.parser = HandsFreeCommandParser()

    def test_app_name_with_special_symbols(self):
        """BVA: App names with symbols (C++, .NET, 3D Studio) parsed cleanly."""
        cmd = self.parser.parse("open C++ Builder")
        self.assertEqual(cmd.action_type, "LaunchApp")
        self.assertEqual(cmd.payload["app_name"], "c++ builder")

    def test_multi_word_app_name_boundary(self):
        """BVA: Multi-word application names parsed without truncation."""
        cmd = self.parser.parse("open google chrome canary edition")
        self.assertEqual(cmd.payload["app_name"], "google chrome canary edition")

    def test_close_app_with_trailing_spaces(self):
        """BVA: Trailing whitespace trimmed from target app name."""
        cmd = self.parser.parse("close slack    ")
        self.assertEqual(cmd.payload["app_name"], "slack")

    def test_app_name_containing_command_keyword(self):
        """BVA: App names starting with 'open' (OpenOffice, OpenShift) handled correctly."""
        cmd = self.parser.parse("open openoffice")
        self.assertEqual(cmd.payload["app_name"], "openoffice")

    def test_bare_close_command_returns_unknown(self):
        """BVA: Bare 'close' without target returns Unknown."""
        cmd = self.parser.parse("close")
        self.assertEqual(cmd.action_type, "Unknown")
