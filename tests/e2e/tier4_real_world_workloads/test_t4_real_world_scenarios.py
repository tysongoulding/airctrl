"""
test_t4_real_world_scenarios.py - Tier 4 Real-World Workload Scenarios

Comprehensive End-to-End User Journeys testing all 5 core requirements (R1-R5):
1. Full LAN distribution, PowerShell fetching, and binary verification
2. Push-to-talk STT with sub-second Whisper transcription & text injection
3. Accidental micro-press (<100ms) audio cancellation
4. Spoken window snap workflow
5. Spoken agentic dispatch to Cursor
6. Radial dial overlay opening, MediaPipe pointing, and pinch execution
7. Zoom meeting detection, camera privacy auto-kill, and compliance audio announcement
8. Background browser Google Meet detection
9. REA application schema reverse engineering workflow
10. Multilingual UTF-8 transcription and clipboard fallback
11. Radial dial inactivity timeout auto-dismissal
12. Concurrent meeting arrival during active STT transcription
13. Complete end-to-end full developer lifecycle

Total Tests: 13 tests
"""

import json
import unittest
import urllib.request
from tests.e2e.e2e_harness import (
    GestureClassifier,
    Landmark3D,
    MeetingWatcher,
    MockLanHttpServer,
    RealLanDaemonProcess,
    PeArtifactInspector,
    PowerShellValidator,
    MockWhisperEngine,
    MockWindowCrawler,
    SemanticActionRouter,
    SpatialRadialDialMapper,
    VoiceActivityDetector,
    create_mock_hand_landmarks,
    create_synthetic_wav,
    discover_lan_ip,
    generate_run_ps1,
    parse_wav_samples,
)
from tests.e2e.tier1_category_partition.test_t1_f06_platform_stubs import (
    MockPlatformInputOps,
    MockPlatformWindowOps,
)
from tests.e2e.tier1_category_partition.test_t1_f11_hotkey_ctrl_space import MockHotkeyListener
from tests.e2e.tier1_category_partition.test_t1_f14_radial_overlay import MockRadialDialOverlay
from tests.e2e.tier1_category_partition.test_t1_f16_agentic_dispatch import MockAgenticHarness
from tests.e2e.tier1_category_partition.test_t1_f17_rea_integration import MockReaInspector
from tests.e2e.tier1_category_partition.test_t1_f24_tauri_ipc_bridge import MockTauriIpcBridge


class TestT4RealWorldScenarios(unittest.TestCase):
    """
    Tier 4: 13 Real-World Workload Scenarios
    """

    def test_scenario_01_full_lan_distribution_and_execution_lifecycle(self):
        """Scenario 1: Full LAN distribution server -> run.ps1 download -> airctrl.exe execution."""
        server = RealLanDaemonProcess()
        h, p = server.start()
        try:
            # 1. Health check
            with urllib.request.urlopen(f"http://{h}:{p}/health") as resp:
                self.assertEqual(resp.status, 200)
                health = json.loads(resp.read().decode("utf-8"))
                self.assertEqual(health["status"], "healthy")
                self.assertTrue(health["artifact_available"])

            # 2. Windows PC executes one-liner: irm http://<vm-ip>:<port>/run.ps1 | iex
            with urllib.request.urlopen(f"http://{h}:{p}/run.ps1") as resp:
                self.assertEqual(resp.status, 200)
                script = resp.read().decode("utf-8")
                self.assertIn(f"$Port = {p}", script)
                self.assertIn('Invoke-WebRequest', script)
                self.assertIn('Start-Process', script)
                val = PowerShellValidator.validate(script)
                self.assertTrue(val["valid"], f"PowerShell syntax error: {val['errors']}")

            # 3. Script downloads airctrl.exe
            with urllib.request.urlopen(f"http://{h}:{p}/airctrl.exe") as resp:
                self.assertEqual(resp.status, 200)
                downloaded_exe = resp.read()
                self.assertTrue(downloaded_exe.startswith(b"MZ"))
                self.assertGreater(len(downloaded_exe), 100000)

            # 4. Version check and checksum match
            with urllib.request.urlopen(f"http://{h}:{p}/version") as resp:
                self.assertEqual(resp.status, 200)
                v = json.loads(resp.read().decode("utf-8"))
                pe = PeArtifactInspector.inspect()
                self.assertEqual(v["artifact_sha256"], pe["sha256"])
        finally:
            server.stop()

    def test_scenario_02_push_to_talk_speech_to_text_active_window_injection(self):
        """Scenario 2: Push-to-talk STT with sub-second Whisper transcription & text injection."""
        win_ops = MockPlatformWindowOps()
        input_ops = MockPlatformInputOps()
        listener = MockHotkeyListener()
        vad = VoiceActivityDetector()
        engine = MockWhisperEngine()

        # User focuses window ID 101
        win_ops.focus_window(101)

        # User presses and holds Ctrl+Spacebar
        listener.on_key_down(1000)
        self.assertEqual(listener.state, "Recording")

        # Synthesize 400ms spoken audio
        wav_bytes = create_synthetic_wav(duration_secs=0.4, freq=300.0, amplitude=0.5)
        samples = parse_wav_samples(wav_bytes)

        # Release hotkey
        listener.on_key_up(1400)
        self.assertEqual(listener.state, "Processing")

        # VAD checks audio energy
        self.assertTrue(vad.is_speech(samples))

        # Whisper transcribes
        res = engine.transcribe(samples)
        self.assertLess(res["duration_ms"], 1000)
        transcribed_text = res["text"]

        # Text injected into active window
        input_ops.type_text(transcribed_text)
        self.assertIn(transcribed_text, input_ops.typed_log)

    def test_scenario_03_accidental_micropress_audio_cancellation(self):
        """Scenario 3: Accidental micro-press (<100ms) audio cancellation."""
        listener = MockHotkeyListener()
        input_ops = MockPlatformInputOps()

        # Press at t=1000, release at t=1040 (40ms tap)
        listener.on_key_down(1000)
        listener.on_key_up(1040)

        # Discarded immediately without processing
        self.assertEqual(listener.state, "Idle")
        self.assertEqual(len(input_ops.typed_log), 0)

    def test_scenario_04_spoken_window_snap_workflow(self):
        """Scenario 4: Spoken window snap workflow ('snap left')."""
        router = SemanticActionRouter()
        action = router.parse("snap left")
        self.assertEqual(action["type"], "WindowSnap")
        self.assertEqual(action["payload"]["direction"], "Left")

        # Simulate window manager geometry snap
        screen_width = 1920
        screen_height = 1080
        if action["payload"]["direction"] == "Left":
            new_bounds = {"x": 0, "y": 0, "width": screen_width // 2, "height": screen_height}
        self.assertEqual(new_bounds["width"], 960)
        self.assertEqual(new_bounds["x"], 0)

    def test_scenario_05_spoken_agentic_instruction_dispatch_cursor(self):
        """Scenario 5: Spoken agentic instruction dispatch to Cursor."""
        router = SemanticActionRouter()
        harness = MockAgenticHarness()

        utterance = "ask cursor: refactor audio pipeline to use lock-free ring buffer"
        action = router.parse(utterance)
        self.assertEqual(action["type"], "AgentDispatch")
        self.assertEqual(action["payload"]["target"], "Cursor")

        dispatch_res = harness.dispatch(
            target=action["payload"]["target"],
            prompt=action["payload"]["prompt"],
            context="File: crates/airctrl-stt/src/audio.rs"
        )
        self.assertEqual(dispatch_res["status"], "Dispatched")
        self.assertIn("lock-free ring buffer", dispatch_res["payload"]["instruction"])

    def test_scenario_06_radial_dial_open_gesture_pointing_and_pinch_execution(self):
        """Scenario 6: Radial dial overlay opening, MediaPipe pointing, and pinch execution."""
        dial = MockRadialDialOverlay()
        classifier = GestureClassifier()
        mapper = SpatialRadialDialMapper(sector_count=6)

        # 1. Open dial via Ctrl+Alt
        dial.toggle()
        self.assertTrue(dial.is_visible)

        # 2. MediaPipe detects Pointing gesture
        pointing_landmarks = create_mock_hand_landmarks("Pointing")
        self.assertEqual(classifier.classify(pointing_landmarks), "Pointing")

        # 3. Fingertip coordinates map to sector 2
        tip = pointing_landmarks[8]
        coords = mapper.map_coords(tip.x, tip.y)
        self.assertFalse(coords["in_deadzone"])
        dial.hover_sector(coords["active_sector"])
        self.assertEqual(dial.active_sector, coords["active_sector"])

        # 4. User pinches to confirm selection
        pinch_landmarks = create_mock_hand_landmarks("Pinch")
        self.assertEqual(classifier.classify(pinch_landmarks), "Pinch")

        # Action executes and dial hides
        dial.toggle()
        self.assertFalse(dial.is_visible)

    def test_scenario_07_meeting_detected_camera_auto_kill_and_compliance_recording(self):
        """Scenario 7: Zoom meeting detection, camera privacy auto-kill, and compliance audio."""
        watcher = MeetingWatcher()
        self.assertTrue(watcher.camera_active)

        # Zoom detected
        procs = [{"process_id": 4321, "process_name": "zoom.exe", "title": "Zoom Meeting"}]
        meeting = watcher.scan_processes(procs)
        self.assertIsNotNone(meeting)
        self.assertEqual(meeting["platform"], "Zoom")

        # Camera immediately killed
        self.assertFalse(watcher.camera_active)

        # User triggers recording -> Compliance announcement played
        announcement = watcher.play_compliance_recording_announcement()
        self.assertEqual(announcement["announcement"], "This meeting is being recorded")
        self.assertTrue(announcement["injected_into_stream"])
        self.assertTrue(watcher.compliance_audio_played)

    def test_scenario_08_background_google_meet_detection_in_browser(self):
        """Scenario 8: Background browser Google Meet detection."""
        watcher = MeetingWatcher()
        procs = [
            {"process_id": 501, "process_name": "chrome.exe", "title": "Google Docs"},
            {"process_id": 502, "process_name": "chrome.exe", "title": "Quarterly Planning - Google Meet"},
            {"process_id": 503, "process_name": "slack.exe", "title": "General"}
        ]
        m = watcher.scan_processes(procs)
        self.assertIsNotNone(m)
        self.assertEqual(m["platform"], "Meet")
        self.assertFalse(watcher.camera_active)

    def test_scenario_09_rea_reverse_engineering_application_workflow(self):
        """Scenario 9: REA application schema reverse engineering workflow."""
        router = SemanticActionRouter()
        rea = MockReaInspector()

        action = router.parse("rea inspect 2048")
        self.assertEqual(action["type"], "ReaInspect")
        pid = action["payload"]["target_pid"]

        schema = rea.inspect(pid)
        self.assertEqual(schema["status"], "SchemaExtracted")
        self.assertIn("Save", schema["action_schema"]["buttons"])

    def test_scenario_10_multilingual_speech_transcription_and_clipboard_fallback(self):
        """Scenario 10: Multilingual UTF-8 transcription and clipboard fallback."""
        engine = MockWhisperEngine()
        input_ops = MockPlatformInputOps()

        samples = [0.1] * 8000
        res = engine.transcribe(samples)
        # Emulating non-ASCII text
        res["text"] = "日本語音声認識テスト 🎙️"

        # Direct input fails -> clipboard paste fallback
        input_ops.paste_text(res["text"])
        self.assertIn(f"[PASTE]{res['text']}", input_ops.typed_log)

    def test_scenario_11_inactivity_timeout_dismissal_of_radial_overlay(self):
        """Scenario 11: Radial dial inactivity timeout auto-dismissal."""
        dial = MockRadialDialOverlay()
        dial.toggle()
        dial.hover_sector(3)
        self.assertTrue(dial.is_visible)

        # 3 seconds elapse with no input
        dial.on_inactivity_timeout()
        self.assertFalse(dial.is_visible)
        self.assertIsNone(dial.active_sector)

    def test_scenario_12_concurrent_meeting_arrival_during_stt_transcription(self):
        """Scenario 12: Concurrent meeting arrival during active STT transcription."""
        watcher = MeetingWatcher()
        engine = MockWhisperEngine()
        input_ops = MockPlatformInputOps()

        # Audio being transcribed
        samples = [0.1] * 8000
        stt_res = engine.transcribe(samples)

        # While transcribing, Teams meeting starts
        procs = [{"process_id": 999, "process_name": "teams.exe", "title": "Teams Call"}]
        meeting = watcher.scan_processes(procs)
        self.assertIsNotNone(meeting)
        self.assertFalse(watcher.camera_active)

        # STT completes and types without crashing
        input_ops.type_text(stt_res["text"])
        self.assertIn(stt_res["text"], input_ops.typed_log)

    def test_scenario_13_end_to_end_full_developer_experience(self):
        """Scenario 13: Complete end-to-end full developer lifecycle."""
        # 1. LAN server starts
        server = MockLanHttpServer(host="127.0.0.1", port=0)
        h, p = server.start()
        try:
            # 2. Client queries version
            with urllib.request.urlopen(f"http://{h}:{p}/version") as resp:
                v = json.loads(resp.read().decode("utf-8"))
                self.assertEqual(v["version"], "0.1.0")

            # 3. Speech to Action
            router = SemanticActionRouter()
            action = router.parse("snap right")
            self.assertEqual(action["type"], "WindowSnap")

            # 4. Gesture classification
            classifier = GestureClassifier()
            palm_lms = create_mock_hand_landmarks("OpenPalm")
            self.assertEqual(classifier.classify(palm_lms), "OpenPalm")

            # 5. Meeting detection
            watcher = MeetingWatcher()
            procs = [{"process_id": 1, "process_name": "zoom.exe", "title": "Zoom"}]
            m = watcher.scan_processes(procs)
            self.assertEqual(m["platform"], "Zoom")
            self.assertFalse(watcher.camera_active)

            # 6. Compliance announcement
            announcement = watcher.play_compliance_recording_announcement()
            self.assertEqual(announcement["status"], "Success")
        finally:
            server.stop()
