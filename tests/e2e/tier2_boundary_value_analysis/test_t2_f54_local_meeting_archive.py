import unittest
from tests.e2e.e2e_harness import (
    LiveTranscriptChunk,
    MeetingArchiveEntry,
    MeetingArchiveStore,
)


class TestT2F54LocalMeetingArchive(unittest.TestCase):
    """
    Feature 54: Local Meeting Archive & Playback (Boundary Value Analysis)
    """

    def setUp(self):
        self.store = MeetingArchiveStore()
        for i in range(5):
            entry = MeetingArchiveEntry(
                id=f"meet_{i}",
                title=f"Standup Meeting {i}",
                date_iso=f"2026-10-1{i}T10:00:00Z",
                duration_secs=600,
                transcript_chunks=[
                    LiveTranscriptChunk(f"c_{i}_1", "Host", f"Host topic {i}", 0, 5000, 0.95),
                    LiveTranscriptChunk(f"c_{i}_2", "Participant", f"Participant reply {i}", 5000, 10000, 0.90)
                ],
                summary_markdown=f"# Summary for meeting {i}",
                audio_path=f"/archive/meet_{i}.wav",
                participants=[f"Dev_{i}", "TechLead"]
            )
            self.store.save_meeting(entry)

    def test_search_no_matches_returns_empty_list(self):
        """BVA: Search with zero matches returns empty list."""
        res = self.store.search("nonexistent_discussion_topic")
        self.assertEqual(res, [])

    def test_search_by_participant_topic(self):
        """BVA: Searching transcript text of a specific participant matches."""
        res = self.store.search("Participant reply 3")
        self.assertEqual(len(res), 1)
        self.assertEqual(res[0].id, "meet_3")

    def test_seek_nonexistent_chunk_returns_none(self):
        """BVA: Seeking invalid chunk ID in existing meeting returns None."""
        offset = self.store.seek_audio_offset("meet_1", "invalid_chunk_id")
        self.assertIsNone(offset)

    def test_seek_nonexistent_meeting_returns_none(self):
        """BVA: Seeking in non-existent meeting returns None."""
        offset = self.store.seek_audio_offset("invalid_meeting_id", "c_0_1")
        self.assertIsNone(offset)

    def test_archive_search_case_insensitivity(self):
        """BVA: Case-insensitive search matches lowercase and uppercase query."""
        res = self.store.search("STANDUP MEETING")
        self.assertEqual(len(res), 5)
