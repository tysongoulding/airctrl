import unittest
from tests.e2e.e2e_harness import ReliablePasteManager


class TestT1F36ReliablePaste(unittest.TestCase):
    """
    Feature 36: Reliable Paste & Delay Controls (Equivalence Partitions)
    """

    def test_reliable_paste_preserves_clipboard(self):
        """Partition: Reliable paste restores user's original clipboard content."""
        mgr = ReliablePasteManager(reliable_paste=True)
        mgr.clipboard_content = "user original text"

        event = mgr.paste_text("transcript text to paste")
        self.assertEqual(event["pasted_text"], "transcript text to paste")
        self.assertTrue(event["restored_clipboard"])
        self.assertEqual(mgr.clipboard_content, "user original text")

    def test_unreliable_paste_overwrites_clipboard(self):
        """Partition: With reliable_paste=False, clipboard is left containing the new transcript."""
        mgr = ReliablePasteManager(reliable_paste=False)
        mgr.clipboard_content = "user original text"

        event = mgr.paste_text("new transcript")
        self.assertFalse(event["restored_clipboard"])
        self.assertEqual(mgr.clipboard_content, "new transcript")

    def test_paste_delay_before_configured(self):
        """Partition: Configurable delay before paste (paste_delay_before_ms)."""
        mgr = ReliablePasteManager(delay_before_ms=120)
        event = mgr.paste_text("hello")
        self.assertEqual(event["delay_before_ms"], 120)

    def test_paste_delay_after_configured(self):
        """Partition: Configurable delay after paste (paste_delay_after_ms)."""
        mgr = ReliablePasteManager(delay_after_ms=250)
        event = mgr.paste_text("hello")
        self.assertEqual(event["delay_after_ms"], 250)

    def test_clipboard_content_verification(self):
        """Partition: Clipboard verification passes when content matches before simulated paste."""
        mgr = ReliablePasteManager()
        event = mgr.paste_text("verified content")
        self.assertTrue(event["verified"])
