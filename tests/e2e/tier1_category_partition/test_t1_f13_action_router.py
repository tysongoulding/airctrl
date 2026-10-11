import unittest
from tests.e2e.e2e_harness import SemanticActionRouter


class TestT1F13ActionRouter(unittest.TestCase):
    """
    Feature 13: Speech-to-Action Router (Equivalence Partitions)
    """

    def setUp(self):
        self.router = SemanticActionRouter()

    def test_window_snap_partition(self):
        """Partition: Utterance parsed to WindowSnap action."""
        res = self.router.parse("snap left")
        self.assertEqual(res["type"], "WindowSnap")
        self.assertEqual(res["payload"]["direction"], "Left")

        res_max = self.router.parse("dock maximize")
        self.assertEqual(res_max["type"], "WindowSnap")
        self.assertEqual(res_max["payload"]["direction"], "Maximize")

    def test_window_focus_partition(self):
        """Partition: Utterance parsed to WindowFocus action."""
        res = self.router.parse("switch to Chrome")
        self.assertEqual(res["type"], "WindowFocus")
        self.assertEqual(res["payload"]["query"], "chrome")

    def test_window_close_and_minimize_partition(self):
        """Partition: Utterances for closing and minimizing window."""
        res_close = self.router.parse("close window")
        self.assertEqual(res_close["type"], "WindowClose")

        res_min = self.router.parse("minimize window")
        self.assertEqual(res_min["type"], "WindowMinimize")

    def test_agent_dispatch_partition(self):
        """Partition: Agent query parsing for Claude and Cursor."""
        res_claude = self.router.parse("ask claude: explain rust traits")
        self.assertEqual(res_claude["type"], "AgentDispatch")
        self.assertEqual(res_claude["payload"]["target"], "Claude")
        self.assertEqual(res_claude["payload"]["prompt"], "explain rust traits")

    def test_unknown_command_fallback(self):
        """Partition: Unmatched speech utterances return Unknown with raw query."""
        res = self.router.parse("unrecognized gibberish command")
        self.assertEqual(res["type"], "Unknown")
        self.assertEqual(res["payload"]["query"], "unrecognized gibberish command")
