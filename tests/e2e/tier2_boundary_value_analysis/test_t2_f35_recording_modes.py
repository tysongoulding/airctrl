import unittest
from tests.e2e.e2e_harness import RecordingMode, RecordingModeController, RecordingState


class TestT2F35RecordingModes(unittest.TestCase):
    """
    Feature 35: 4-Variant Recording Modes (Boundary Value Analysis)
    """

    def test_jitter_debounce_threshold_29ms_vs_30ms(self):
        """BVA: 29.0ms duration cancelled as jitter; 30.0ms accepted as genuine press."""
        ctrl = RecordingModeController(RecordingMode.SMART_DUAL_MODE)
        ctrl.on_key_down(1000.0)
        # 29ms release -> cancelled
        s1 = ctrl.on_key_up(1029.0)
        self.assertEqual(s1, RecordingState.CANCELLED)

        # 30ms release -> accepted into recording
        ctrl.on_key_down(2000.0)
        s2 = ctrl.on_key_up(2030.0)
        self.assertEqual(s2, RecordingState.RECORDING)

    def test_push_to_talk_boundary_249ms_vs_250ms(self):
        """BVA: 249ms latches into Toggle recording; 250ms triggers immediate PTT transcription."""
        ctrl = RecordingModeController(RecordingMode.SMART_DUAL_MODE)
        ctrl.on_key_down(1000.0)
        s1 = ctrl.on_key_up(1249.0)
        self.assertEqual(s1, RecordingState.RECORDING)  # Latch toggle

        ctrl.cancel()
        ctrl.on_key_down(2000.0)
        s2 = ctrl.on_key_up(2250.0)
        self.assertEqual(s2, RecordingState.TRANSCRIBING)  # PTT release

    def test_rapid_successive_toggles_boundary(self):
        """BVA: 10 successive rapid toggle presses switch state deterministically."""
        ctrl = RecordingModeController(RecordingMode.TOGGLE_ONLY)
        for i in range(10):
            ctrl.on_key_down(1000.0 + i * 100)
            ctrl.on_key_up(1050.0 + i * 100)
        self.assertEqual(ctrl.transcribed_count, 5)

    def test_extreme_hold_duration_hold_only(self):
        """BVA: Long 60,000ms key hold in HoldOnly maintains recording until release."""
        ctrl = RecordingModeController(RecordingMode.HOLD_ONLY)
        ctrl.on_key_down(1000.0)
        self.assertEqual(ctrl.state, RecordingState.RECORDING)
        s = ctrl.on_key_up(61000.0)
        self.assertEqual(s, RecordingState.TRANSCRIBING)

    def test_vad_exact_energy_threshold_boundary(self):
        """BVA: Exactly 0.05 energy threshold activates recording; 0.0499 does not."""
        ctrl = RecordingModeController(RecordingMode.CONTINUOUS_HANDS_FREE)
        s1 = ctrl.on_vad_speech_detected(0.0499, threshold=0.05)
        self.assertEqual(s1, RecordingState.IDLE)

        s2 = ctrl.on_vad_speech_detected(0.05, threshold=0.05)
        self.assertEqual(s2, RecordingState.RECORDING)
