import unittest
from tests.e2e.e2e_harness import AirCtrlCliParser


class TestT1F40CliFlagsDebug(unittest.TestCase):
    """
    Feature 40: CLI Remote Control & Startup Flags (Equivalence Partitions)
    """

    def test_toggle_transcription_flag_partition(self):
        """Partition: --toggle-transcription flag toggles transcription mode remotely."""
        opts = AirCtrlCliParser.parse_args(["--toggle-transcription"])
        self.assertTrue(opts["toggle_transcription"])
        self.assertFalse(opts["cancel"])

    def test_toggle_post_process_flag_partition(self):
        """Partition: --toggle-post-process flag toggles post-processing rules."""
        opts = AirCtrlCliParser.parse_args(["--toggle-post-process"])
        self.assertTrue(opts["toggle_post_process"])

    def test_cancel_and_hidden_flags_partition(self):
        """Partition: --cancel and --start-hidden flags parsed simultaneously."""
        opts = AirCtrlCliParser.parse_args(["--cancel", "--start-hidden"])
        self.assertTrue(opts["cancel"])
        self.assertTrue(opts["start_hidden"])

    def test_no_tray_and_debug_flags_partition(self):
        """Partition: --no-tray and --debug flags parsed cleanly."""
        opts = AirCtrlCliParser.parse_args(["--no-tray", "--debug"])
        self.assertTrue(opts["no_tray"])
        self.assertTrue(opts["debug"])

    def test_model_override_cli_flag_partition(self):
        """Partition: --model flag correctly extracts model name parameter."""
        opts = AirCtrlCliParser.parse_args(["--model", "parakeet-unified-en-0.6b-q8_0"])
        self.assertEqual(opts["model"], "parakeet-unified-en-0.6b-q8_0")
