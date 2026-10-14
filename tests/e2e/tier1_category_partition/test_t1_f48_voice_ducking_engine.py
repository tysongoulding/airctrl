import unittest
from tests.e2e.e2e_harness import DualAudioMixer


class TestT1F48VoiceDuckingEngine(unittest.TestCase):
    """
    Feature 48: Dynamic Voice Ducking Engine (Equivalence Partitions)
    """

    def test_ducking_attenuates_system_audio(self):
        """Partition: Loud mic energy (> threshold) ducks system loopback gain towards 0.25."""
        mixer = DualAudioMixer(sample_rate=16000, ducking_threshold=0.05, ducking_gain=0.25)
        # Feed 1000 samples (~62.5ms) of active speech
        mic = [0.8] * 1000
        loopback = [0.5] * 1000
        _ = mixer.mix(mic, loopback)
        self.assertLess(mixer.current_duck_gain, 0.40)
        self.assertGreaterEqual(mixer.current_duck_gain, 0.25)

    def test_silent_mic_no_ducking(self):
        """Partition: Mic silence maintains loopback gain at unity (1.0)."""
        mixer = DualAudioMixer(sample_rate=16000, ducking_threshold=0.05, ducking_gain=0.25)
        mic = [0.001] * 500
        loopback = [0.5] * 500
        _ = mixer.mix(mic, loopback)
        self.assertAlmostEqual(mixer.current_duck_gain, 1.0, places=2)

    def test_ducking_attack_time(self):
        """Partition: Ducking attack begins attenuation within 30ms (480 samples)."""
        mixer = DualAudioMixer(sample_rate=16000, attack_ms=30.0)
        mic = [0.9] * 480
        loopback = [0.5] * 480
        _ = mixer.mix(mic, loopback)
        self.assertLess(mixer.current_envelope, 0.9)
        self.assertGreater(mixer.current_envelope, 0.5)

    def test_ducking_release_time(self):
        """Partition: Loopback gain begins recovering towards 1.0 when speech stops."""
        mixer = DualAudioMixer(sample_rate=16000, release_ms=500.0)
        # First duck it until it settles near ducking floor
        _ = mixer.mix([0.8] * 4000, [0.5] * 4000)
        ducked_gain = mixer.current_duck_gain
        self.assertAlmostEqual(ducked_gain, 0.25, delta=0.05)

        # Now feed 30,000 samples (~1.8s) of silence to trigger release recovery
        _ = mixer.mix([0.0] * 30000, [0.5] * 30000)
        self.assertGreater(mixer.current_duck_gain, 0.60)

    def test_ducking_gain_floor_invariant(self):
        """Partition: Gain never drops below statutory ducking floor 0.25."""
        mixer = DualAudioMixer(sample_rate=16000, ducking_gain=0.25)
        # Extreme mic input
        _ = mixer.mix([10.0] * 5000, [1.0] * 5000)
        self.assertGreaterEqual(mixer.current_duck_gain, 0.249)
