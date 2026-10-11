import unittest


class MockPlatformWindowOps:
    def __init__(self):
        self.focused_window_id = 101

    def get_active_window(self):
        return {"id": self.focused_window_id, "title": "Mock Focused Window"}

    def focus_window(self, window_id: int):
        self.focused_window_id = window_id
        return True


class MockPlatformInputOps:
    def __init__(self):
        self.typed_log = []

    def type_text(self, text: str):
        self.typed_log.append(text)
        return True

    def paste_text(self, text: str):
        self.typed_log.append(f"[PASTE]{text}")
        return True


class MockPlatformAudioOps:
    def __init__(self):
        self.played_streams = []

    def play_audio_stream(self, samples, sample_rate: int):
        self.played_streams.append({"samples": len(samples), "rate": sample_rate})
        return True


class TestT1F06PlatformStubs(unittest.TestCase):
    """
    Feature 6: Cross-Platform Abstraction Stubs (Equivalence Partitions)
    """

    def test_platform_window_ops_get_active(self):
        """Partition: WindowOps trait returns active window."""
        ops = MockPlatformWindowOps()
        win = ops.get_active_window()
        self.assertEqual(win["id"], 101)

    def test_platform_window_ops_focus(self):
        """Partition: WindowOps trait switches focus to target ID."""
        ops = MockPlatformWindowOps()
        success = ops.focus_window(202)
        self.assertTrue(success)
        self.assertEqual(ops.focused_window_id, 202)

    def test_platform_input_ops_type_text(self):
        """Partition: InputOps types text into active window."""
        ops = MockPlatformInputOps()
        ops.type_text("Hello airctrl!")
        self.assertIn("Hello airctrl!", ops.typed_log)

    def test_platform_input_ops_paste_fallback(self):
        """Partition: InputOps uses paste clipboard fallback."""
        ops = MockPlatformInputOps()
        ops.paste_text("Fallback content")
        self.assertIn("[PASTE]Fallback content", ops.typed_log)

    def test_platform_audio_ops_playback(self):
        """Partition: AudioOps plays audio stream at 16kHz."""
        ops = MockPlatformAudioOps()
        success = ops.play_audio_stream([0.1, 0.2, 0.3], 16000)
        self.assertTrue(success)
        self.assertEqual(ops.played_streams[0]["rate"], 16000)
