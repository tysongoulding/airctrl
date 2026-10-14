import unittest
from tests.e2e.e2e_harness import (
    LiveTranscriptChunk,
    MeetingArchiveEntry,
    MeetingArchiveStore,
)


class TestT1F54LocalMeetingArchive(unittest.TestCase):
    """
    Feature 54: Local Meeting Archive & Playback (Equivalence Partitions)
    """

    def setUp(self):
        self.store = MeetingArchiveStore()
        chunk1 = LiveTranscriptChunk("c1", "Host", "Welcome everyone to sprint planning", 0, 3000, 0.95)
        chunk2 = LiveTranscriptChunk("c2", "Participant", "We need to discuss the database migration", 3000, 6500, 0.92)

        self.entry = MeetingArchiveEntry(
            id="meet_01",
            title="Sprint Planning Meeting",
            date_iso="2026-10-14T09:00:00Z",
            duration_secs=300,
            transcript_chunks=[chunk1, chunk2],
            summary_markdown="# Sprint Planning Summary",
            audio_path="/archive/meet_01.wav",
            participants=["Alice", "Bob"]
        )
        self.store.save_meeting(self.entry)

    def test_save_and_retrieve_meeting_entry(self):
        """Partition: Archive stores and lists meeting metadata."""
        self.assertEqual(len(self.store.entries), 1)
        self.assertEqual(self.store.entries[0].id, "meet_01")

    def test_search_meetings_by_title_keyword(self):
        """Partition: Full-text search matches keyword in meeting title."""
        res = self.store.search("Planning")
        self.assertEqual(len(res), 1)
        self.assertEqual(res[0].id, "meet_01")

    def test_search_meetings_by_transcript_content(self):
        """Partition: Search matches text inside nested transcript chunks."""
        res = self.store.search("database migration")
        self.assertEqual(len(res), 1)
        self.assertEqual(res[0].id, "meet_01")

    def test_seek_audio_offset_for_chunk(self):
        """Partition: Seeking chunk offset returns exact start timestamp for synchronized playback."""
        offset = self.store.seek_audio_offset("meet_01", "c2")
        self.assertEqual(offset, 3000)

    def test_empty_search_returns_all_meetings(self):
        """Partition: Empty search query returns all archived meetings."""
        res = self.store.search("")
        self.assertEqual(len(res), 1)
