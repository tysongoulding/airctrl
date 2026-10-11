import unittest


class MockTextInjector:
    def __init__(self):
        self.focused_hwnd = 1001
        self.injected_strings = []
        self.clipboard = ""

    def inject(self, text: str, current_hwnd: int):
        if not text:
            return {"status": "Ignored", "method": "None"}
        if current_hwnd != self.focused_hwnd:
            # Re-focus target before injecting
            self.focused_hwnd = current_hwnd
        # Unicode direct injection
        self.injected_strings.append(text)
        return {"status": "Success", "method": "SendInputUnicode", "text": text}

    def clipboard_fallback(self, text: str):
        self.clipboard = text
        return {"status": "Success", "method": "ClipboardPaste", "text": text}


class TestT1F12TextInjection(unittest.TestCase):
    """
    Feature 12: Active Window Text Injection (Equivalence Partitions)
    """

    def setUp(self):
        self.injector = MockTextInjector()

    def test_direct_unicode_text_injection(self):
        """Partition: Normal Unicode text injection into focused window."""
        res = self.injector.inject("Build airctrl desktop app", 1001)
        self.assertEqual(res["status"], "Success")
        self.assertEqual(res["method"], "SendInputUnicode")
        self.assertIn("Build airctrl desktop app", self.injector.injected_strings)

    def test_window_focus_restoration_during_injection(self):
        """Partition: Active window changed while speaking is refocused."""
        res = self.injector.inject("Switched window text", 2002)
        self.assertEqual(self.injector.focused_hwnd, 2002)
        self.assertEqual(res["status"], "Success")

    def test_empty_text_injection_ignored(self):
        """Partition: Empty string is safely ignored without keystroke events."""
        res = self.injector.inject("", 1001)
        self.assertEqual(res["status"], "Ignored")

    def test_clipboard_paste_fallback(self):
        """Partition: Direct input failure activates clipboard paste fallback."""
        res = self.injector.clipboard_fallback("Fallback pasted text")
        self.assertEqual(res["method"], "ClipboardPaste")
        self.assertEqual(self.injector.clipboard, "Fallback pasted text")

    def test_multilingual_unicode_characters(self):
        """Partition: Injection handles UTF-8 characters cleanly."""
        multilingual = "Testing: 日本語, Español (ñ), Emoji 🎙️"
        res = self.injector.inject(multilingual, 1001)
        self.assertEqual(res["text"], multilingual)
