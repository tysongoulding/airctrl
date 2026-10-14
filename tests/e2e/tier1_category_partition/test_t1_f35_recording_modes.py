import unittest
from tests.e2e.e2e_harness import RecordingMode, RecordingModeController, RecordingState


class TestT1F35RecordingModes(unittest.TestCase):
    """
    Feature 35: 4-Variant Recording Modes (Equivalence Partitions)
    """

    def test_smart_dual_mode_tap_toggle(self):
        """Partition: Tap (< 250ms) in SmartDualMode enters latched Recording state."""
        ctrl = RecordingModeController(RecordingMode.SMART_DUAL_MODE)
        ctrl.on_key_down(1000.0)
        state = ctrl.on_key_up(1100.0)  # 100ms duration
        self.assertEqual(state, RecordingState.RECORDING)

        # Second tap transcribes and finishes
        ctrl.on_key_down(2000.0)
        self.assertEqual(ctrl.state, RecordingState.TRANSCRIBING)

    def test_smart_dual_mode_hold_ptt(self):
        """Partition: Hold (>= 250ms) in SmartDualMode transcribes on release."""
        ctrl = RecordingModeController(RecordingMode.SMART_DUAL_MODE)
        ctrl.on_key_down(1000.0)
        state = ctrl.on_key_up(1350.0)  # 350ms duration
        self.assertEqual(state, RecordingState.TRANSCRIBING)
        self.assertEqual(ctrl.transcribed_count, 1)

    def test_hold_only_mode(self):
        """Partition: HoldOnly records only while key is down, finishes on release."""
        ctrl = RecordingModeController(RecordingMode.HOLD_ONLY)
        self.assertEqual(ctrl.on_key_down(1000.0), RecordingState.RECORDING)
        self.assertEqual(ctrl.on_key_up(1050.0), RecordingState.TRANSCRIBING)

    def test_toggle_only_mode(self):
        """Partition: ToggleOnly toggles on press regardless of hold time."""
        ctrl = RecordingModeController(RecordingMode.TOGGLE_ONLY)
        ctrl.on_key_down(1000.0)
        self.assertEqual(ctrl.state, RecordingState.RECORDING)
        ctrl.on_key_up(1500.0)  # Hold duration ignored
        self.assertEqual(ctrl.state, RecordingState.RECORDING)

        # Next press toggles off
        ctrl.on_key_down(2000.0)
        self.assertEqual(ctrl.state, RecordingState.TRANSCRIBING)

    def test_continuous_hands_free_vad_mode(self):
        """Partition: ContinuousHandsFree triggers recording via VAD energy without keypresses."""
        ctrl = RecordingModeController(RecordingMode.CONTINUOUS_HANDS_FREE)
        # Above threshold -> recording
        ctrl.on_vad_speech_detected(0.12, threshold=0.05)
        self.assertEqual(ctrl.state, RecordingState.RECORDING)

        # Below threshold -> speech ended, transcribing
        ctrl.on_vad_speech_detected(0.01, threshold=0.05)
        self.assertEqual(ctrl.state, RecordingState.TRANSCRIBING)
