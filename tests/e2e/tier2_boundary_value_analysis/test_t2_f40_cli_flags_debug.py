import unittest
from tests.e2e.e2e_harness import AirCtrlCliParser


class TestT2F40CliFlagsDebug(unittest.TestCase):
    """
    Feature 40: CLI Remote Control & Startup Flags (Boundary Value Analysis)
    """

    def test_empty_argv_defaults_boundary(self):
        """BVA: Empty argv produces clean defaults (all False/None)."""
        opts = AirCtrlCliParser.parse_args([])
        self.assertFalse(opts["toggle_transcription"])
        self.assertFalse(opts["toggle_post_process"])
        self.assertFalse(opts["cancel"])
        self.assertFalse(opts["start_hidden"])
        self.assertFalse(opts["no_tray"])
        self.assertFalse(opts["debug"])
        self.assertIsNone(opts["model"])

    def test_all_flags_simultaneous_boundary(self):
        """BVA: All supported flags passed simultaneously in a single command."""
        argv = [
            "--toggle-transcription",
            "--toggle-post-process",
            "--cancel",
            "--start-hidden",
            "--no-tray",
            "--debug",
            "--model", "whisper-large.en"
        ]
        opts = AirCtrlCliParser.parse_args(argv)
        self.assertTrue(opts["toggle_transcription"])
        self.assertTrue(opts["toggle_post_process"])
        self.assertTrue(opts["cancel"])
        self.assertTrue(opts["start_hidden"])
        self.assertTrue(opts["no_tray"])
        self.assertTrue(opts["debug"])
        self.assertEqual(opts["model"], "whisper-large.en")

    def test_unknown_arbitrary_flag_tolerance(self):
        """BVA: Unknown flags are tolerated without crashing parser."""
        opts = AirCtrlCliParser.parse_args(["--unknown-flag-xyz", "--toggle-transcription"])
        self.assertTrue(opts["toggle_transcription"])

    def test_missing_model_value_at_argv_end(self):
        """BVA: --model specified at end of argv with missing trailing value handles cleanly."""
        opts = AirCtrlCliParser.parse_args(["--model"])
        self.assertIsNone(opts["model"])

    def test_case_sensitive_flag_recognition(self):
        """BVA: Uppercase '--DEBUG' is not recognized as '--debug'."""
        opts = AirCtrlCliParser.parse_args(["--DEBUG"])
        self.assertFalse(opts["debug"])
