import unittest
from tests.e2e.e2e_harness import ReliablePasteManager


class TestT2F36ReliablePaste(unittest.TestCase):
    """
    Feature 36: Reliable Paste & Delay Controls (Boundary Value Analysis)
    """

    def test_zero_delay_boundaries(self):
        """BVA: 0ms delay_before and 0ms delay_after paste execution."""
        mgr = ReliablePasteManager(delay_before_ms=0, delay_after_ms=0)
        event = mgr.paste_text("instant paste")
        self.assertEqual(event["delay_before_ms"], 0)
        self.assertEqual(event["delay_after_ms"], 0)

    def test_large_delay_boundary_5000ms(self):
        """BVA: 5000ms delay values recorded without truncation."""
        mgr = ReliablePasteManager(delay_before_ms=5000, delay_after_ms=5000)
        event = mgr.paste_text("delayed paste")
        self.assertEqual(event["delay_before_ms"], 5000)
        self.assertEqual(event["delay_after_ms"], 5000)

    def test_empty_string_paste_boundary(self):
        """BVA: Empty string paste preserves clipboard state."""
        mgr = ReliablePasteManager(reliable_paste=True)
        mgr.clipboard_content = "original"
        event = mgr.paste_text("")
        self.assertEqual(event["pasted_text"], "")
        self.assertEqual(mgr.clipboard_content, "original")

    def test_large_text_paste_100k_chars(self):
        """BVA: 100,000-character payload pasted and restored reliably."""
        large_text = "A" * 100000
        mgr = ReliablePasteManager(reliable_paste=True)
        mgr.clipboard_content = "small clipboard"
        event = mgr.paste_text(large_text)
        self.assertEqual(len(event["pasted_text"]), 100000)
        self.assertEqual(mgr.clipboard_content, "small clipboard")

    def test_special_characters_and_emojis_paste(self):
        """BVA: Multi-byte unicode emojis and special escape chars in clipboard."""
        special_str = "🚀 airctrl: \n\t\\\"' $PATH <> &"
        mgr = ReliablePasteManager(reliable_paste=True)
        mgr.clipboard_content = "prior"
        event = mgr.paste_text(special_str)
        self.assertEqual(event["pasted_text"], special_str)
        self.assertTrue(event["verified"])
