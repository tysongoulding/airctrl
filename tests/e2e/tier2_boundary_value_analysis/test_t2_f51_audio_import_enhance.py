import unittest
from tests.e2e.tier1_category_partition.test_t1_f51_audio_import_enhance import MockAudioImportService
from tests.e2e.e2e_harness import WhisperModelVariant


class TestT2F51AudioImportEnhance(unittest.TestCase):
    """
    Feature 51: Audio Import & Enhance Workflow (Boundary Value Analysis)
    """

    def setUp(self):
        self.service = MockAudioImportService()

    def test_uppercase_audio_extension_boundary(self):
        """BVA: Uppercase extension (.WAV) parsed cleanly."""
        res = self.service.import_and_transcribe("/audio/RECORDING.WAV")
        self.assertEqual(res["format"], "wav")

    def test_hidden_dotfile_audio_path_boundary(self):
        """BVA: Hidden dotfile audio file (.hidden_meeting.mp3) parsed correctly."""
        res = self.service.import_and_transcribe("/audio/.hidden_meeting.mp3")
        self.assertEqual(res["format"], "mp3")

    def test_path_with_spaces_and_unicode_emojis(self):
        """BVA: Audio file path with spaces and emoji characters."""
        res = self.service.import_and_transcribe("/audio/Sprint Standup 🎙️ 2026.m4a")
        self.assertEqual(res["format"], "m4a")

    def test_file_without_extension_raises_error(self):
        """BVA: File path without extension raises ValueError."""
        with self.assertRaises(ValueError):
            self.service.import_and_transcribe("/audio/raw_audio_file")

    def test_large_whisper_model_selection_boundary(self):
        """BVA: Selection of Whisper LargeEn model recorded in result payload."""
        res = self.service.import_and_transcribe("/audio/meeting.flac", model=WhisperModelVariant.LARGE_EN)
        self.assertEqual(res["model_used"], "whisper-large.en")
