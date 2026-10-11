import unittest
from tests.e2e.tier1_category_partition.test_t1_f16_agentic_dispatch import MockAgenticHarness


class TestT2F16AgenticDispatch(unittest.TestCase):
    """
    Feature 16: Agentic Dispatch Interface (Boundary Value Analysis)
    """

    def setUp(self):
        self.harness = MockAgenticHarness()

    def test_single_character_prompt_boundary(self):
        """BVA: 1-character prompt '?' dispatched."""
        res = self.harness.dispatch("Claude", "?")
        self.assertEqual(res["prompt_len"], 1)

    def test_empty_context_string_boundary(self):
        """BVA: Empty context string handled safely."""
        res = self.harness.dispatch("Cursor", "Write unit test", context="")
        self.assertEqual(res["payload"]["context_window"], "")

    def test_large_context_window_50000_chars(self):
        """BVA: 50,000 character context buffer dispatched."""
        big_ctx = "Context " * 6000
        res = self.harness.dispatch("Antigravity", "Review code", context=big_ctx)
        self.assertGreater(len(res["payload"]["context_window"]), 40000)

    def test_multiline_prompt_boundary(self):
        """BVA: Prompt containing 100 newlines."""
        prompt = "\n".join([f"line {i}" for i in range(100)])
        res = self.harness.dispatch("Dots", prompt)
        self.assertEqual(res["target"], "Dots")

    def test_agent_name_case_sensitivity_strict(self):
        """BVA: Agent target case sensitivity 'cursor' vs 'Cursor'."""
        with self.assertRaises(ValueError):
            self.harness.dispatch("cursor", "test")
