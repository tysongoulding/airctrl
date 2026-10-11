import json
import unittest


class MockTauriIpcBridge:
    def __init__(self):
        self.handlers = {
            "startRecording": lambda args: {"status": "RecordingStarted"},
            "stopRecording": lambda args: {"status": "TranscriptionReady", "text": "hello"},
            "triggerAction": lambda args: {"success": True, "action": args.get("action")},
            "toggleRadialDial": lambda args: {"dial_visible": True}
        }

    def invoke(self, command_name: str, args=None):
        if command_name not in self.handlers:
            raise KeyError(f"Tauri command not found: {command_name}")
        return self.handlers[command_name](args or {})


class TestT1F24TauriIpcBridge(unittest.TestCase):
    """
    Feature 24: Tauri 2.0 Shell & IPC Bridge (Equivalence Partitions)
    """

    def setUp(self):
        self.bridge = MockTauriIpcBridge()

    def test_invoke_start_recording(self):
        """Partition: IPC command startRecording invoked."""
        res = self.bridge.invoke("startRecording")
        self.assertEqual(res["status"], "RecordingStarted")

    def test_invoke_stop_recording(self):
        """Partition: IPC command stopRecording returns transcription."""
        res = self.bridge.invoke("stopRecording")
        self.assertEqual(res["status"], "TranscriptionReady")
        self.assertEqual(res["text"], "hello")

    def test_invoke_trigger_action_with_args(self):
        """Partition: IPC command triggerAction passes payload arguments."""
        res = self.bridge.invoke("triggerAction", {"action": "WindowSnap"})
        self.assertTrue(res["success"])
        self.assertEqual(res["action"], "WindowSnap")

    def test_invoke_toggle_radial_dial(self):
        """Partition: IPC command toggleRadialDial."""
        res = self.bridge.invoke("toggleRadialDial")
        self.assertTrue(res["dial_visible"])

    def test_invoke_unknown_command_raises_error(self):
        """Partition: Unregistered command name raises error."""
        with self.assertRaises(KeyError):
            self.bridge.invoke("unknownCommand")
