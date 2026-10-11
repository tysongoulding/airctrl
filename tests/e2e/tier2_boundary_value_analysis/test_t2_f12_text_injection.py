import unittest
from tests.e2e.tier1_category_partition.test_t1_f12_text_injection import MockTextInjector


class TestT2F12TextInjection(unittest.TestCase):
    """
    Feature 12: Active Window Text Injection (Boundary Value Analysis)
    """

    def setUp(self):
        self.injector = MockTextInjector()

    def test_null_byte_character_in_string(self):
        """BVA: String containing null byte '\\x00'."""
        text = "before\x00after"
        res = self.injector.inject(text, 1001)
        self.assertEqual(res["status"], "Success")
        self.assertIn("\x00", res["text"])

    def test_single_surrogate_pair_emoji_boundary(self):
        """BVA: 4-byte UTF-8 emoji boundary (e.g. 🎙️)."""
        res = self.injector.inject("🎙️", 1001)
        self.assertEqual(res["text"], "🎙️")

    def test_large_payload_20000_characters(self):
        """BVA: 20,000 characters payload."""
        large_text = "A" * 20000
        res = self.injector.inject(large_text, 1001)
        self.assertEqual(len(res["text"]), 20000)

    def test_only_newlines_and_spaces_boundary(self):
        """BVA: Text consisting solely of whitespace and newlines."""
        res = self.injector.inject("   \n\r\t   ", 1001)
        self.assertEqual(res["status"], "Success")

    def test_window_handle_id_maximum_boundary(self):
        """BVA: Maximum 32-bit HWND handle 4294967295."""
        res = self.injector.inject("test", 4294967295)
        self.assertEqual(self.injector.focused_hwnd, 4294967295)
