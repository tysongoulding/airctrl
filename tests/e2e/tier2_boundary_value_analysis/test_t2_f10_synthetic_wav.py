import unittest
from tests.e2e.e2e_harness import create_synthetic_wav, parse_wav_samples


class TestT2F10SyntheticWav(unittest.TestCase):
    """
    Feature 10: Synthetic WAV STT Processing (Boundary Value Analysis)
    """

    def test_minimal_duration_wav_boundary_1ms(self):
        """BVA: 0.001 second (1ms) duration generates 16 samples at 16kHz."""
        wav_bytes = create_synthetic_wav(duration_secs=0.001, sample_rate=16000)
        samples = parse_wav_samples(wav_bytes)
        self.assertEqual(len(samples), 16)

    def test_amplitude_maximum_boundary_1_0(self):
        """BVA: Peak amplitude 1.0 clamped within 16-bit signed range [-32768, 32767]."""
        wav_bytes = create_synthetic_wav(duration_secs=0.01, amplitude=1.0)
        samples = parse_wav_samples(wav_bytes)
        self.assertTrue(all(-1.0 <= s <= 1.0 for s in samples))

    def test_amplitude_zero_boundary(self):
        """BVA: Amplitude 0.0 produces flatline silence with all zero samples."""
        wav_bytes = create_synthetic_wav(duration_secs=0.01, amplitude=0.0)
        samples = parse_wav_samples(wav_bytes)
        self.assertTrue(all(s == 0.0 for s in samples))

    def test_header_truncated_off_by_one_43_bytes(self):
        """BVA: 43 bytes header (1 byte short of 44) raises ValueError."""
        header_43 = b"RIFF" + b"\x00" * 39
        self.assertEqual(len(header_43), 43)
        with self.assertRaises(ValueError):
            parse_wav_samples(header_43)

    def test_frequency_nyquist_boundary_8000hz(self):
        """BVA: Frequency at Nyquist limit (8000Hz for 16kHz sample rate)."""
        wav_bytes = create_synthetic_wav(duration_secs=0.01, sample_rate=16000, freq=8000.0)
        samples = parse_wav_samples(wav_bytes)
        self.assertEqual(len(samples), 160)
