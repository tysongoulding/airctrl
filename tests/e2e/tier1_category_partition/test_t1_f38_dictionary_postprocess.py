import unittest
from tests.e2e.e2e_harness import DictionaryService


class TestT1F38DictionaryPostprocess(unittest.TestCase):
    """
    Feature 38: Custom Dictionary & Post-Processing (Equivalence Partitions)
    """

    def test_exact_word_replacement_partition(self):
        """Partition: Replaces defined vocabulary phrase."""
        service = DictionaryService({"air control": "airctrl"})
        res = service.replace_words("I am testing air control today")
        self.assertIn("airctrl", res)
        self.assertNotIn("air control", res)

    def test_multiple_rules_replacement_partition(self):
        """Partition: Multiple vocabulary replacement rules applied cleanly."""
        service = DictionaryService({
            "tauri to": "Tauri 2.0",
            "rust lane": "airctrl-lan",
        })
        res = service.replace_words("building tauri to on rust lane")
        self.assertEqual(res, "Building Tauri 2.0 on airctrl-lan")

    def test_acoustic_feedback_cues_partition(self):
        """Partition: Acoustic feedback cues triggered on recording lifecycle events."""
        service = DictionaryService()
        service.play_cue("recording_start")
        service.play_cue("recording_stop")
        self.assertEqual(service.played_cues, ["recording_start", "recording_stop"])

    def test_post_processing_capitalization_partition(self):
        """Partition: Post-processing automatically capitalizes first letter and normalizes whitespace."""
        service = DictionaryService()
        service.post_process_enabled = True
        res = service.replace_words("  hello   world  ")
        self.assertEqual(res, "Hello world")

    def test_disabled_post_processing_raw_partition(self):
        """Partition: Disabling post-processing outputs uncapitalized text with original spaces."""
        service = DictionaryService()
        service.post_process_enabled = False
        res = service.replace_words("lowercase raw text")
        self.assertEqual(res, "lowercase raw text")
