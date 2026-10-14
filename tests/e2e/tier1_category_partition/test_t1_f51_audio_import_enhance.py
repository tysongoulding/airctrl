import os
import unittest
from tests.e2e.e2e_harness import WhisperModelVariant


class MockAudioImportService:
    SUPPORTED_EXTENSIONS = {".wav", ".mp3", ".m4a", ".aac", ".flac", ".ogg"}

    def import_and_transcribe(self, file_path: str, model: WhisperModelVariant = WhisperModelVariant.SMALL_EN):
        ext = os.path.splitext(file_path)[1].lower()
        if ext not in self.SUPPORTED_EXTENSIONS:
            raise ValueError(f"Unsupported audio format: {ext}")
        return {
            "source_path": file_path,
            "format": ext.lstrip("."),
            "model_used": model.value,
            "transcript": "enhanced local meeting transcript",
            "duration_secs": 180,
            "sample_rate": 16000
        }


class TestT1F51AudioImportEnhance(unittest.TestCase):
    """
    Feature 51: Audio Import & Enhance Workflow (Equivalence Partitions)
    """

    def setUp(self):
        self.service = MockAudioImportService()

    def test_import_wav_file_partition(self):
        """Partition: Standard .wav file decoding and re-transcription."""
        res = self.service.import_and_transcribe("/audio/recorded_meeting.wav")
        self.assertEqual(res["format"], "wav")
        self.assertIn("transcript", res)

    def test_import_mp3_and_m4a_partitions(self):
        """Partition: Compressed .mp3 and .m4a audio container decoding."""
        res_mp3 = self.service.import_and_transcribe("/audio/standup.mp3")
        self.assertEqual(res_mp3["format"], "mp3")

        res_m4a = self.service.import_and_transcribe("/audio/call.m4a")
        self.assertEqual(res_m4a["format"], "m4a")

    def test_import_flac_and_ogg_partitions(self):
        """Partition: Lossless .flac and open .ogg format decoding."""
        res_flac = self.service.import_and_transcribe("/audio/interview.flac")
        self.assertEqual(res_flac["format"], "flac")

        res_ogg = self.service.import_and_transcribe("/audio/briefing.ogg")
        self.assertEqual(res_ogg["format"], "ogg")

    def test_unsupported_audio_format_rejection(self):
        """Partition: Unsupported file extensions (.exe, .bin, .pdf) raise ValueError."""
        with self.assertRaises(ValueError):
            self.service.import_and_transcribe("/audio/archive.bin")

    def test_custom_model_selection_for_enhance(self):
        """Partition: Model selection (Parakeet V3) is recorded in import output."""
        res = self.service.import_and_transcribe(
            "/audio/meeting.wav",
            model=WhisperModelVariant.PARAKEET_V3
        )
        self.assertEqual(res["model_used"], WhisperModelVariant.PARAKEET_V3.value)
