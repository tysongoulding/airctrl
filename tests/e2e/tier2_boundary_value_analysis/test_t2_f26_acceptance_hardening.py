import unittest
from tests.e2e.e2e_harness import SemanticActionRouter


class TestT2F26AcceptanceHardening(unittest.TestCase):
    """
    Feature 26: E2E Acceptance & Adversarial Hardening (Boundary Value Analysis)
    """

    def setUp(self):
        self.router = SemanticActionRouter()

    def test_buffer_overflow_length_utterance_10000_chars(self):
        """BVA: 10,000 character utterance handled without stack overflow."""
        overflow_utterance = "focus " + ("a" * 10000)
        res = self.router.parse(overflow_utterance)
        self.assertEqual(res["type"], "WindowFocus")
        self.assertEqual(len(res["payload"]["query"]), 10000)

    def test_null_byte_in_utterance_boundary(self):
        """BVA: Utterance with null byte handled safely."""
        res = self.router.parse("snap left\x00extra")
        self.assertIn("type", res)

    def test_repetitive_rapid_parsing_stress(self):
        """BVA: Parsing 1000 distinct utterances in tight loop."""
        for i in range(1000):
            res = self.router.parse(f"launch app_{i}")
            self.assertEqual(res["type"], "LaunchApp")

    def test_extreme_punctuation_utterance(self):
        """BVA: Utterance consisting entirely of punctuation marks."""
        res = self.router.parse("!@#$%^&*()_+{}:<>?")
        self.assertEqual(res["type"], "Unknown")

    def test_mixed_language_scripts_in_prompt(self):
        """BVA: Utterance with mixed Arabic, Cyrillic, Chinese, and Greek scripts."""
        mixed = "ask claude: مرحبا Привет 你好 Γειά σου"
        res = self.router.parse(mixed)
        self.assertEqual(res["type"], "AgentDispatch")
        self.assertIn("مرحبا Привет 你好 Γειά σου", res["payload"]["prompt"])
