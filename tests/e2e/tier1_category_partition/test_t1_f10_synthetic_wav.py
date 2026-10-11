import unittest
from tests.e2e.e2e_harness import create_synthetic_wav, parse_wav_samples


class TestT1F10SyntheticWav(unittest.TestCase):
    """
    Feature 10: Synthetic WAV STT Processing (Equivalence Partitions)
    """

    def test_synthetic_wav_valid_riff_header(self):
        """Partition: Generated WAV contains valid RIFF and WAVE magic headers."""
        wav_bytes = create_synthetic_wav(duration_secs=0.2, sample_rate=16000)
        self.assertEqual(wav_bytes[0:4], b"RIFF")
        self.assertEqual(wav_bytes[8:12], b"WAVE")

    def test_parse_wav_samples_count(self):
        """Partition: Sample count matches duration * sample_rate."""
        wav_bytes = create_synthetic_wav(duration_secs=0.5, sample_rate=16000)
        samples = parse_wav_samples(wav_bytes)
        self.assertEqual(len(samples), 8000)

    def test_sample_amplitude_normalized_range(self):
        """Partition: Parsed PCM samples strictly within [-1.0, 1.0]."""
        wav_bytes = create_synthetic_wav(duration_secs=0.1, amplitude=0.8)
        samples = parse_wav_samples(wav_bytes)
        self.assertTrue(all(-1.0 <= s <= 1.0 for s in samples))

    def test_corrupt_header_raises_value_error(self):
        """Partition: Corrupt header rejected with descriptive error."""
        corrupt_wav = create_synthetic_wav(duration_secs=0.1, corrupt_header=True)
        with self.assertRaises(ValueError):
            parse_wav_samples(corrupt_wav)

    def test_truncated_bytes_rejected(self):
        """Partition: Truncated WAV header (<44 bytes) raises error."""
        with self.assertRaises(ValueError):
            parse_wav_samples(b"RIFFtruncated")
