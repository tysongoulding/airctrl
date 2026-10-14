import unittest
from tests.e2e.e2e_harness import HeadlessIpcBridgeR3


class TestT1F60HeadlessIpcBridgeSurface(unittest.TestCase):
    """
    Feature 60: Headless IPC Bridge Command Surface (Equivalence Partitions)
    """

    def setUp(self):
        self.bridge = HeadlessIpcBridgeR3()

    def test_ipc_stt_commands_dispatch(self):
        """Partition: Dispatches STT commands (listModels, selectModel, setRecordingMode, setOverlayPosition)."""
        res_list = self.bridge.dispatch("listModels")
        self.assertEqual(res_list["status"], "success")
        self.assertIn("whisper-turbo.en", res_list["models"])

        res_sel = self.bridge.dispatch("selectModel", {"model": "parakeet-unified-en-0.6b-q8_0"})
        self.assertEqual(res_sel["selected_model"], "parakeet-unified-en-0.6b-q8_0")

        res_mode = self.bridge.dispatch("setRecordingMode", {"mode": "HoldOnly"})
        self.assertEqual(res_mode["mode"], "HoldOnly")

        res_pos = self.bridge.dispatch("setOverlayPosition", {"position": "Top"})
        self.assertTrue(res_pos["bounds"]["visible"])

    def test_ipc_action_commands_dispatch(self):
        """Partition: Dispatches Action commands (executeVoiceCommand, snapWindow, resizeWindow)."""
        res_cmd = self.bridge.dispatch("executeVoiceCommand", {"utterance": "snap left"})
        self.assertEqual(res_cmd["action"], "SnapWindow")

        res_snap = self.bridge.dispatch("snapWindow", {"position": "Right"})
        self.assertEqual(res_snap["bounds"]["x"], 960)

        res_resize = self.bridge.dispatch("resizeWindow", {"dimension": "larger"})
        self.assertEqual(res_resize["status"], "success")

    def test_ipc_meeting_commands_dispatch(self):
        """Partition: Dispatches Meeting commands (getMixerStatus, startMeetingRecording, generateSummary, exportSummary)."""
        res_mixer = self.bridge.dispatch("getMixerStatus")
        self.assertEqual(res_mixer["status"], "success")
        self.assertEqual(res_mixer["sample_rate"], 16000)

        res_rec = self.bridge.dispatch("startMeetingRecording")
        self.assertTrue(res_rec["recording_started"])

        res_sum = self.bridge.dispatch("generateSummary", {"transcript": "Meeting discussion.", "template": "ActionItems"})
        self.assertIn("Action Items", res_sum["summary"]["summary_markdown"])

        res_exp = self.bridge.dispatch("exportSummary", {"content": "# Test", "format": "Markdown"})
        self.assertEqual(res_exp["format"], "Markdown")

    def test_ipc_gestures_camera_commands_dispatch(self):
        """Partition: Dispatches Virtual Camera & Gesture commands."""
        res_status = self.bridge.dispatch("getVirtualCameraStatus")
        self.assertEqual(res_status["device_name"], "AirCtrl Virtual Camera")

        res_priv = self.bridge.dispatch("setVirtualCameraPrivacy", {"privacy_muted": True})
        self.assertTrue(res_priv["privacy_muted"])

    def test_ipc_unknown_command_error(self):
        """Partition: Unknown IPC command returns structured error status."""
        res = self.bridge.dispatch("unknownCommandXYZ")
        self.assertEqual(res["status"], "error")
        self.assertIn("unknownCommandXYZ", res["error"])
