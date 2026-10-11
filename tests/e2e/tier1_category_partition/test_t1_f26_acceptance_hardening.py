import unittest
from tests.e2e.e2e_harness import SemanticActionRouter


class TestT1F26AcceptanceHardening(unittest.TestCase):
    """
    Feature 26: E2E Acceptance & Adversarial Hardening (Equivalence Partitions)
    """

    def setUp(self):
        self.router = SemanticActionRouter()

    def test_unicode_escaping_integrity(self):
        """Partition: Utterances with multibyte Unicode characters."""
        res = self.router.parse("ask claude: こんにちは世界 🚀")
        self.assertEqual(res["type"], "AgentDispatch")
        self.assertIn("こんにちは世界 🚀", res["payload"]["prompt"])

    def test_shell_metacharacters_sanitization(self):
        """Partition: Utterances containing command injection metacharacters."""
        malicious = "ask cursor: echo hello; rm -rf / | iex & calc.exe"
        res = self.router.parse(malicious)
        self.assertEqual(res["type"], "AgentDispatch")
        self.assertIn("rm -rf /", res["payload"]["prompt"])  # Stored safely as data string

    def test_whitespace_and_newline_padding(self):
        """Partition: Excessive whitespace and newlines stripped cleanly."""
        padded = "\n\t   snap left   \r\n"
        res = self.router.parse(padded)
        self.assertEqual(res["type"], "WindowSnap")
        self.assertEqual(res["payload"]["direction"], "Left")

    def test_special_symbols_and_sql_injection(self):
        """Partition: SQL injection strings treated as plain utterance data."""
        sql_injection = "switch to ' OR '1'='1"
        res = self.router.parse(sql_injection)
        self.assertEqual(res["type"], "WindowFocus")
        self.assertIn("' or '1'='1", res["payload"]["query"])

    def test_empty_string_adversarial_safe_fallback(self):
        """Partition: Pure whitespace string returns Unknown safely."""
        res = self.router.parse("    \t\n  ")
        self.assertEqual(res["type"], "Unknown")
