import unittest
from tests.e2e.e2e_harness import HeadlessIpcBridgeR3


class TestT2F60HeadlessIpcBridgeSurface(unittest.TestCase):
    """
    Feature 60: Headless IPC Bridge Command Surface (Boundary Value Analysis)
    """

    def setUp(self):
        self.bridge = HeadlessIpcBridgeR3()

    def test_empty_payload_dispatch_handling(self):
        """BVA: Dispatching commands with None or empty payload uses safe defaults."""
        res = self.bridge.dispatch("listModels", None)
        self.assertEqual(res["status"], "success")

        res_mixer = self.bridge.dispatch("getMixerStatus", {})
        self.assertEqual(res_mixer["status"], "success")

    def test_invalid_recording_mode_string_handling(self):
        """BVA: Passing invalid recording mode string raises ValueError without crashing process."""
        with self.assertRaises(ValueError):
            self.bridge.dispatch("setRecordingMode", {"mode": "InvalidModeStringXYZ"})

    def test_large_dictionary_update_payload_1000_rules(self):
        """BVA: Bulk updating 1,000 custom dictionary rules in a single IPC call."""
        rules = {f"trigger_{i}": f"replacement_{i}" for i in range(1000)}
        res = self.bridge.dispatch("updateDictionary", {"rules": rules})
        self.assertEqual(res["status"], "success")
        self.assertEqual(res["total_rules"], 1000)

    def test_rapid_sequential_cross_subsystem_dispatch(self):
        """BVA: Dispatches commands across STT, Actions, Meetings, Gestures in rapid sequence."""
        cmds = [
            ("selectModel", {"model": "whisper-medium.en"}),
            ("snapWindow", {"position": "Left"}),
            ("startMeetingRecording", {}),
            ("setVirtualCameraPrivacy", {"privacy_muted": True}),
        ]
        for cmd, payload in cmds:
            res = self.bridge.dispatch(cmd, payload)
            self.assertEqual(res["status"], "success")

    def test_unsupported_export_format_ipc_error(self):
        """BVA: Unsupported export format string via IPC raises ValueError."""
        with self.assertRaises(ValueError):
            self.bridge.dispatch("exportSummary", {"content": "text", "format": "INVALID_FMT"})
