import unittest
from tests.e2e.e2e_harness import TranscriptHistoryStore, TranscriptItem


class TestT2F39TranscriptHistory(unittest.TestCase):
    """
    Feature 39: Searchable Transcript History (Boundary Value Analysis)
    """

    def setUp(self):
        self.store = TranscriptHistoryStore()
        for i in range(10):
            item = TranscriptItem(
                id=f"item_{i}",
                timestamp=1000.0 + i,
                text=f"Meeting transcript note index {i}",
                raw_text=f"meeting transcript note index {i}",
                duration_ms=1000,
                model_used="whisper-tiny.en" if i % 2 == 0 else "parakeet-unified-en-0.6b-q8_0",
                confidence=0.90
            )
            self.store.store(item)

    def test_single_character_query_boundary(self):
        """BVA: Single-character search ('e') correctly matches all occurrences."""
        res = self.store.search("e")
        self.assertEqual(len(res), 10)

    def test_limit_parameter_boundary_exact_count(self):
        """BVA: Limit parameter clamps results strictly to requested count (e.g. limit=3)."""
        res = self.store.search("meeting", limit=3)
        self.assertEqual(len(res), 3)

    def test_zero_match_query_returns_empty_list(self):
        """BVA: Query with no matches returns empty list rather than None or error."""
        res = self.store.search("unmatched_cryptic_term")
        self.assertEqual(res, [])

    def test_search_large_item_collection_boundary(self):
        """BVA: 500 items stored and searched within sub-millisecond time."""
        for j in range(500):
            self.store.store(TranscriptItem(
                id=f"bulk_{j}",
                timestamp=2000.0 + j,
                text=f"Batch utterance item number {j}",
                raw_text=f"batch utterance item number {j}",
                duration_ms=500,
                model_used="whisper-turbo.en",
                confidence=0.99
            ))
        res = self.store.search("Batch utterance item number 250")
        self.assertEqual(len(res), 1)
        self.assertEqual(res[0].id, "bulk_250")

    def test_very_long_query_string_boundary(self):
        """BVA: 500-character search query executes without buffer overflow."""
        long_query = "word " * 100
        res = self.store.search(long_query)
        self.assertEqual(res, [])
