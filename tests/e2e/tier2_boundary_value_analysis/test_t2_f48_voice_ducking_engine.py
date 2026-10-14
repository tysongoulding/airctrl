import unittest
from tests.e2e.e2e_harness import DualAudioMixer


class TestT2F48VoiceDuckingEngine(unittest.TestCase):
    """
    Feature 48: Dynamic Voice Ducking Engine (Boundary Value Analysis)
    """

    def test_instant_step_attack_response(self):
        """BVA: Instant transition from 0 to 1.0 engages envelope follower monotonically."""
        mixer = DualAudioMixer(sample_rate=16000, attack_ms=30.0)
        _ = mixer.mix([0.0] * 100, [0.5] * 100)
        env_before = mixer.current_envelope

        _ = mixer.mix([1.0] * 100, [0.5] * 100)
        self.assertGreater(mixer.current_envelope, env_before)

    def test_instant_release_decay_response(self):
        """BVA: Envelope follower decays smoothly after sudden drop to silence."""
        mixer = DualAudioMixer(sample_rate=16000, release_ms=500.0)
        _ = mixer.mix([1.0] * 1000, [0.5] * 1000)
        peak_env = mixer.current_envelope

        _ = mixer.mix([0.0] * 500, [0.5] * 500)
        self.assertLess(mixer.current_envelope, peak_env)

    def test_exact_ducking_threshold_boundary(self):
        """BVA: Mic envelope at threshold (0.05) triggers ducking target gain."""
        mixer = DualAudioMixer(ducking_threshold=0.05)
        _ = mixer.mix([0.051] * 2000, [0.5] * 2000)
        self.assertLess(mixer.current_duck_gain, 0.95)

    def test_ducking_gain_custom_floor_boundary_010(self):
        """BVA: Custom ducking gain floor = 0.10 (-20dB) respected."""
        mixer = DualAudioMixer(ducking_gain=0.10)
        _ = mixer.mix([1.0] * 5000, [1.0] * 5000)
        self.assertAlmostEqual(mixer.current_duck_gain, 0.10, delta=0.05)

    def test_high_sample_rate_48khz_boundary(self):
        """BVA: 48,000Hz sample rate stream scales ducking alphas appropriately."""
        mixer = DualAudioMixer(sample_rate=48000, attack_ms=30.0, release_ms=500.0)
        _ = mixer.mix([0.8] * 3000, [0.5] * 3000)
        self.assertLess(mixer.current_duck_gain, 0.50)
