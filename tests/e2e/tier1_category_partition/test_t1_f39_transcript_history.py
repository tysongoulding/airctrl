import unittest
from tests.e2e.e2e_harness import TranscriptHistoryStore, TranscriptItem


class TestT1F39TranscriptHistory(unittest.TestCase):
    """
    Feature 39: Searchable Transcript History (Equivalence Partitions)
    """

    def setUp(self):
        self.store = TranscriptHistoryStore()
        self.item1 = TranscriptItem(
            id="t1",
            timestamp=1700000000.0,
            text="First meeting notes",
            raw_text="first meeting notes",
            duration_ms=1200,
            model_used="whisper-tiny.en",
            confidence=0.92
        )
        self.item2 = TranscriptItem(
            id="t2",
            timestamp=1700001000.0,
            text="Second sprint standup",
            raw_text="second sprint standup",
            duration_ms=2500,
            model_used="parakeet-unified-en-0.6b-q8_0",
            confidence=0.98
        )
        self.store.store(self.item1)
        self.store.store(self.item2)

    def test_store_and_retrieve_transcript(self):
        """Partition: Storing items retains all fields and increases item count."""
        self.assertEqual(len(self.store.items), 2)
        self.assertEqual(self.store.items[0].id, "t1")

    def test_search_by_keyword_partition(self):
        """Partition: Search matches keyword in transcript text."""
        res = self.store.search("standup")
        self.assertEqual(len(res), 1)
        self.assertEqual(res[0].id, "t2")

    def test_search_by_model_filter_partition(self):
        """Partition: Search filters by specific model name."""
        res = self.store.search("", model_filter="parakeet-unified-en-0.6b-q8_0")
        self.assertEqual(len(res), 1)
        self.assertEqual(res[0].id, "t2")

    def test_empty_query_returns_all_recent(self):
        """Partition: Empty query returns all items in reverse chronological order."""
        res = self.store.search("")
        self.assertEqual(len(res), 2)
        self.assertEqual(res[0].id, "t2")  # most recent first

    def test_clear_history_partition(self):
        """Partition: Clear operation removes all stored items."""
        self.store.clear()
        self.assertEqual(len(self.store.items), 0)
        self.assertEqual(len(self.store.search("")), 0)
