import json
import unittest


class MockAgenticHarness:
    SUPPORTED_AGENTS = ["Cursor", "Claude", "Antigravity", "Grokbot", "Dots", "Muse", "Viktor", "Gemini"]

    def dispatch(self, target: str, prompt: str, context: str = ""):
        if target not in self.SUPPORTED_AGENTS:
            raise ValueError(f"Unsupported agent target: {target}")
        return {
            "target": target,
            "status": "Dispatched",
            "prompt_len": len(prompt),
            "payload": {
                "instruction": prompt,
                "context_window": context,
                "timestamp": "2026-10-11T07:22:00Z"
            }
        }


class TestT1F16AgenticDispatch(unittest.TestCase):
    """
    Feature 16: Agentic Dispatch Interface (Equivalence Partitions)
    """

    def setUp(self):
        self.harness = MockAgenticHarness()

    def test_dispatch_to_cursor(self):
        """Partition: Dispatching instructions to Cursor agent."""
        res = self.harness.dispatch("Cursor", "Refactor error types in airctrl-core")
        self.assertEqual(res["target"], "Cursor")
        self.assertEqual(res["status"], "Dispatched")

    def test_dispatch_to_claude(self):
        """Partition: Dispatching instructions to Claude agent."""
        res = self.harness.dispatch("Claude", "Generate comprehensive unit tests")
        self.assertEqual(res["target"], "Claude")

    def test_dispatch_to_antigravity(self):
        """Partition: Dispatching instructions to Antigravity agent."""
        res = self.harness.dispatch("Antigravity", "Execute spec verification")
        self.assertEqual(res["target"], "Antigravity")

    def test_dispatch_to_all_8_supported_agents(self):
        """Partition: All 8 supported agents accepted."""
        for agent in MockAgenticHarness.SUPPORTED_AGENTS:
            res = self.harness.dispatch(agent, "Ping agent")
            self.assertEqual(res["target"], agent)

    def test_dispatch_unsupported_agent_rejected(self):
        """Partition: Unknown agent name raises ValueError."""
        with self.assertRaises(ValueError):
            self.harness.dispatch("NonExistentAgent", "Hello")
