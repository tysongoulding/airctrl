import unittest
from tests.e2e.e2e_harness import DictionaryService


class TestT2F38DictionaryPostprocess(unittest.TestCase):
    """
    Feature 38: Custom Dictionary & Post-Processing (Boundary Value Analysis)
    """

    def test_zero_rules_dictionary_preserves_text(self):
        """BVA: Dictionary with 0 rules returns input text verbatim (with capitalization if enabled)."""
        service = DictionaryService({})
        service.post_process_enabled = False
        res = service.replace_words("verbatim unedited text")
        self.assertEqual(res, "verbatim unedited text")

    def test_word_boundary_isolation_prevents_substring_corruption(self):
        """BVA: Word boundary check prevents 'air' rule from mangling 'airplane' or 'fair'."""
        service = DictionaryService({"air": "atmosphere"})
        res = service.replace_words("fresh air at the airport is fair")
        self.assertIn("Fresh atmosphere", res)
        self.assertIn("airport", res)
        self.assertIn("fair", res)

    def test_case_insensitive_matching_boundary(self):
        """BVA: Case-insensitive matching replaces 'AIR CONTROL', 'Air Control', 'air control'."""
        service = DictionaryService({"air control": "airctrl"})
        res1 = service.replace_words("AIR CONTROL system")
        res2 = service.replace_words("Air Control system")
        self.assertIn("Airctrl", res1)
        self.assertIn("Airctrl", res2)

    def test_unicode_accents_and_symbols(self):
        """BVA: Vocabulary substitution handling accents (café -> coffeehouse)."""
        service = DictionaryService({"café": "coffeehouse"})
        res = service.replace_words("meet at the café")
        self.assertIn("coffeehouse", res)

    def test_empty_input_string_boundary(self):
        """BVA: Empty input string returns empty string without error."""
        service = DictionaryService({"foo": "bar"})
        self.assertEqual(service.replace_words(""), "")
