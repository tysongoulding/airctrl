import unittest
from tests.e2e.e2e_harness import SemanticActionRouter


class TestT2F13ActionRouter(unittest.TestCase):
    """
    Feature 13: Speech-to-Action Router (Boundary Value Analysis)
    """

    def setUp(self):
        self.router = SemanticActionRouter()

    def test_single_character_utterance(self):
        """BVA: 1-character utterance 'a' returns Unknown safely."""
        res = self.router.parse("a")
        self.assertEqual(res["type"], "Unknown")
        self.assertEqual(res["payload"]["query"], "a")

    def test_all_uppercase_command(self):
        """BVA: Case-insensitivity: 'SNAP LEFT' matches WindowSnap."""
        res = self.router.parse("SNAP LEFT")
        self.assertEqual(res["type"], "WindowSnap")
        self.assertEqual(res["payload"]["direction"], "Left")

    def test_very_long_utterance_1000_chars(self):
        """BVA: 1000-character utterance parsed without buffer error."""
        long_prompt = "ask claude: " + ("analyze this code " * 50)
        res = self.router.parse(long_prompt)
        self.assertEqual(res["type"], "AgentDispatch")
        self.assertEqual(res["payload"]["target"], "Claude")

    def test_snap_direction_boundary_maximize(self):
        """BVA: Snap direction maximize."""
        res = self.router.parse("snap maximize")
        self.assertEqual(res["type"], "WindowSnap")
        self.assertEqual(res["payload"]["direction"], "Maximize")

    def test_colon_spacing_variations_for_agent(self):
        """BVA: Agent command with multiple colons and spaces 'ask cursor: : do task'."""
        res = self.router.parse("ask cursor: : do task")
        self.assertEqual(res["type"], "AgentDispatch")
        self.assertEqual(res["payload"]["target"], "Cursor")
