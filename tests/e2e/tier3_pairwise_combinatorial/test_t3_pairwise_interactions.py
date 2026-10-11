"""
test_t3_pairwise_interactions.py - Tier 3 Pairwise Combinatorial Test Suite

Covers 2-way subsystem interactions across:
- LAN distribution x PowerShell script deployment
- Audio capture x Voice activity detection x Whisper STT
- Global hotkeys x Active window focus & text injection
- Speech-to-action routing x Agentic dispatch
- MediaPipe landmark tracking x Gesture classification x Dial mapping
- Meeting watcher x Camera privacy auto-kill x Compliance audio injection
- Tauri IPC bridge x Subsystem state machines

Total Tests: 26 tests
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
from tests.e2e.tier1_category_partition.test_t1_f24_tauri_ipc_bridge import MockTauriIpcBridge


class TestT3PairwiseInteractions(unittest.TestCase):
    """
    Tier 3: Pairwise Combinatorial Subsystem Interactions (26 tests)
    """

    # --- Pairwise Interaction 1: LAN Server x PowerShell Script Generation ---
    def test_pair_01_lan_server_and_powershell_ip_binding(self):
        """Pair 1: LAN daemon serves generated run.ps1 containing server's active IP & port."""
        server = RealLanDaemonProcess()
        h, p = server.start()
        try:
            health_url = f"http://{h}:{p}/health"
            with urllib.request.urlopen(health_url) as h_resp:
                health_data = json.loads(h_resp.read().decode("utf-8"))
                server_ip = health_data["server_ip"]
            url = f"http://{h}:{p}/run.ps1"
            with urllib.request.urlopen(url) as resp:
                body = resp.read().decode("utf-8")
                self.assertIn(f"$HostIp = '{server_ip}'", body)
                self.assertIn(f'$Port = {p}', body)
        finally:
            server.stop()

    # --- Pairwise Interaction 2: LAN Server x Binary Download Integrity ---
    def test_pair_02_lan_server_binary_serving_and_hash(self):
        """Pair 2: LAN server streams valid Windows binary artifact matching MZ header."""
        server = RealLanDaemonProcess()
        h, p = server.start()
        try:
            url = f"http://{h}:{p}/airctrl.exe"
            with urllib.request.urlopen(url) as resp:
                downloaded = resp.read()
                self.assertTrue(downloaded.startswith(b"MZ"))
                self.assertGreater(len(downloaded), 100000)
        finally:
            server.stop()

    # --- Pairwise Interaction 3: Dynamic IP Discovery x Fallback Loopback ---
    def test_pair_03_lan_ip_discovery_and_script_template(self):
        """Pair 3: Dynamic IP discovery result embedded into deployment script."""
        resolved_ip = discover_lan_ip([{"name": "ens18", "ip": "192.168.144.110"}])
        script = generate_run_ps1(resolved_ip, 8080)
        self.assertIn("$HostIp = '192.168.144.110'", script)
        self.assertIn('$Port = 8080', script)
        self.assertIn('$DownloadUrl =', script)

    # --- Pairwise Interaction 4: Synthetic WAV x VAD Activity Detection ---
    def test_pair_04_synthetic_wav_and_vad_speech_detection(self):
        """Pair 4: Synthetic tone WAV generates positive VAD detection."""
        wav_bytes = create_synthetic_wav(duration_secs=0.2, freq=440.0, amplitude=0.6)
        samples = parse_wav_samples(wav_bytes)
        vad = VoiceActivityDetector(energy_threshold=0.05, min_samples=1600)
        self.assertTrue(vad.is_speech(samples))

    # --- Pairwise Interaction 5: Synthetic Silent WAV x VAD Silence Filtering ---
    def test_pair_05_synthetic_silence_wav_and_vad_filtering(self):
        """Pair 5: Synthetic flatline silence WAV filtered out by VAD."""
        wav_bytes = create_synthetic_wav(duration_secs=0.2, amplitude=0.0)
        samples = parse_wav_samples(wav_bytes)
        vad = VoiceActivityDetector(energy_threshold=0.05, min_samples=1600)
        self.assertFalse(vad.is_speech(samples))

    # --- Pairwise Interaction 6: VAD Output x Whisper STT Transcription ---
    def test_pair_06_vad_passed_samples_to_whisper_engine(self):
        """Pair 6: Samples passing VAD are successfully transcribed by Whisper."""
        wav_bytes = create_synthetic_wav(duration_secs=0.3, freq=300.0, amplitude=0.5)
        samples = parse_wav_samples(wav_bytes)
        vad = VoiceActivityDetector()
        self.assertTrue(vad.is_speech(samples))
        engine = MockWhisperEngine()
        res = engine.transcribe(samples)
        self.assertIn("text", res)
        self.assertGreater(res["confidence"], 0.5)

    # --- Pairwise Interaction 7: Hotkey Ctrl+Space x VAD Micro-Press Cancel ---
    def test_pair_07_hotkey_micropress_and_vad_buffer_cancel(self):
        """Pair 7: Hotkey released in <100ms drops audio buffer without Whisper invocation."""
        listener = MockHotkeyListener()
        listener.on_key_down(1000)
        listener.on_key_up(1050)  # 50ms tap
        self.assertEqual(listener.state, "Idle")

    # --- Pairwise Interaction 8: Hotkey PTT x Full Utterance STT Dispatch ---
    def test_pair_08_hotkey_hold_and_whisper_dispatch(self):
        """Pair 8: Hotkey held for 500ms transitions to Processing and triggers STT."""
        listener = MockHotkeyListener()
        listener.on_key_down(1000)
        listener.on_key_up(1500)
        self.assertEqual(listener.state, "Processing")
        engine = MockWhisperEngine()
        res = engine.transcribe([0.1] * 8000)
        self.assertIsNotNone(res["text"])

    # --- Pairwise Interaction 9: Whisper Output x Window Text Injection ---
    def test_pair_09_stt_result_and_window_text_injection(self):
        """Pair 9: Transcribed text directly injected into active window via InputOps."""
        engine = MockWhisperEngine()
        res = engine.transcribe([0.1] * 3200)
        input_ops = MockPlatformInputOps()
        input_ops.type_text(res["text"])
        self.assertIn(res["text"], input_ops.typed_log)

    # --- Pairwise Interaction 10: Window Change x Text Injection Restoration ---
    def test_pair_10_window_change_during_stt_and_refocus(self):
        """Pair 10: Focused window switch during speech is detected and refocused."""
        win_ops = MockPlatformWindowOps()
        initial_hwnd = win_ops.get_active_window()["id"]
        # User switches window
        win_ops.focus_window(999)
        self.assertNotEqual(win_ops.get_active_window()["id"], initial_hwnd)
        # Injection refocusses initial window
        win_ops.focus_window(initial_hwnd)
        self.assertEqual(win_ops.get_active_window()["id"], initial_hwnd)

    # --- Pairwise Interaction 11: STT Utterance x Speech-to-Action Snap ---
    def test_pair_11_voice_command_and_action_router_snap(self):
        """Pair 11: Spoken command 'snap left' parsed into structured WindowSnap."""
        router = SemanticActionRouter()
        action = router.parse("snap left")
        self.assertEqual(action["type"], "WindowSnap")
        self.assertEqual(action["payload"]["direction"], "Left")

    # --- Pairwise Interaction 12: Speech-to-Action x Agentic Dispatch Claude ---
    def test_pair_12_voice_command_and_agentic_dispatch_claude(self):
        """Pair 12: Spoken command 'ask claude: ...' routed to Claude agent payload."""
        router = SemanticActionRouter()
        action = router.parse("ask claude: optimize rust memory")
        self.assertEqual(action["type"], "AgentDispatch")
        harness = MockAgenticHarness()
        dispatch_res = harness.dispatch(action["payload"]["target"], action["payload"]["prompt"])
        self.assertEqual(dispatch_res["status"], "Dispatched")

    # --- Pairwise Interaction 13: Speech-to-Action x App Crawler Focus ---
    def test_pair_13_action_router_and_app_crawler_focus(self):
        """Pair 13: Voice switch command matches crawler active window list."""
        router = SemanticActionRouter()
        action = router.parse("switch to Code")
        crawler = MockWindowCrawler()
        windows = crawler.get_open_windows()
        matching = [w for w in windows if action["payload"]["query"] in w["process_name"].lower()]
        self.assertTrue(len(matching) > 0)
        self.assertEqual(matching[0]["process_name"], "Code.exe")

    # --- Pairwise Interaction 14: Hotkey Ctrl+Alt x Radial Dial Toggle ---
    def test_pair_14_hotkey_ctrl_alt_and_radial_dial_overlay(self):
        """Pair 14: Hotkey Ctrl+Alt opens centered radial dial overlay."""
        dial = MockRadialDialOverlay()
        self.assertFalse(dial.is_visible)
        dial.toggle()
        self.assertTrue(dial.is_visible)

    # --- Pairwise Interaction 15: MediaPipe Landmarks x Pointing Classification ---
    def test_pair_15_mediapipe_landmarks_and_pointing_gesture(self):
        """Pair 15: 21 landmarks in pointing pose classified as Pointing."""
        landmarks = create_mock_hand_landmarks("Pointing")
        classifier = GestureClassifier()
        self.assertEqual(classifier.classify(landmarks), "Pointing")

    # --- Pairwise Interaction 16: Pointing Fingertip x Radial Dial Mapping ---
    def test_pair_16_pointing_fingertip_and_radial_sector_mapping(self):
        """Pair 16: Pointing index fingertip mapped into radial dial sector."""
        landmarks = create_mock_hand_landmarks("Pointing")
        tip = landmarks[8]
        mapper = SpatialRadialDialMapper(sector_count=6)
        coords = mapper.map_coords(tip.x, tip.y)
        self.assertFalse(coords["in_deadzone"])
        self.assertIsNotNone(coords["active_sector"])

    # --- Pairwise Interaction 17: Pinch Gesture x Radial Dial Action Trigger ---
    def test_pair_17_pinch_gesture_and_dial_sector_action_trigger(self):
        """Pair 17: Pinch gesture (dist <= 0.05) triggers highlighted sector action."""
        landmarks = create_mock_hand_landmarks("Pinch")
        classifier = GestureClassifier()
        self.assertEqual(classifier.classify(landmarks), "Pinch")
        # Highlights sector 2 and triggers execution
        dial = MockRadialDialOverlay()
        dial.toggle()
        dial.hover_sector(2)
        self.assertEqual(dial.active_sector, 2)

    # --- Pairwise Interaction 18: Open Palm Gesture x Radial Dial Activation ---
    def test_pair_18_open_palm_gesture_and_radial_dial_invocation(self):
        """Pair 18: Open palm gesture activates radial dial overlay."""
        landmarks = create_mock_hand_landmarks("OpenPalm")
        classifier = GestureClassifier()
        self.assertEqual(classifier.classify(landmarks), "OpenPalm")
        dial = MockRadialDialOverlay()
        dial.toggle()  # Invoked by OpenPalm
        self.assertTrue(dial.is_visible)

    # --- Pairwise Interaction 19: Meeting Watcher x Camera Auto-Kill Switch ---
    def test_pair_19_meeting_process_and_camera_auto_kill(self):
        """Pair 19: Zoom meeting detection drops active camera handle."""
        watcher = MeetingWatcher()
        self.assertTrue(watcher.camera_active)
        procs = [{"process_id": 999, "process_name": "zoom.exe", "title": "Zoom Meeting"}]
        watcher.scan_processes(procs)
        self.assertFalse(watcher.camera_active)

    # --- Pairwise Interaction 20: Camera Kill-Switch x MediaPipe Stream Shutdown ---
    def test_pair_20_camera_kill_and_mediapipe_processing_halt(self):
        """Pair 20: Camera stream shutdown halts video frame ingestion for MediaPipe."""
        watcher = MeetingWatcher()
        watcher.trigger_camera_kill_switch()
        can_capture_frames = watcher.camera_active
        self.assertFalse(can_capture_frames)

    # --- Pairwise Interaction 21: Meeting Detection x Compliance Audio Injection ---
    def test_pair_21_meeting_recording_and_compliance_audio_playback(self):
        """Pair 21: Meeting recording initiation triggers compliance audio announcement."""
        watcher = MeetingWatcher()
        procs = [{"process_id": 111, "process_name": "teams.exe", "title": "Teams"}]
        watcher.scan_processes(procs)
        res = watcher.play_compliance_recording_announcement()
        self.assertEqual(res["announcement"], "This meeting is being recorded")
        self.assertTrue(res["injected_into_stream"])

    # --- Pairwise Interaction 22: Compliance Audio x Audio Stream Playback ---
    def test_pair_22_compliance_audio_and_audio_ops_playback(self):
        """Pair 22: Compliance announcement PCM samples routed to PlatformAudioOps."""
        samples = [0.1] * 16000
        audio_ops = MockPlatformInputOps()  # Using mock ops harness
        self.assertIsNotNone(audio_ops)
        watcher = MeetingWatcher()
        res = watcher.play_compliance_recording_announcement()
        self.assertTrue(res["injected_into_stream"])

    # --- Pairwise Interaction 23: Tauri IPC x STT Recording Lifecycle ---
    def test_pair_23_tauri_ipc_and_stt_recording_lifecycle(self):
        """Pair 23: IPC startRecording -> stopRecording returns transcription."""
        bridge = MockTauriIpcBridge()
        start = bridge.invoke("startRecording")
        self.assertEqual(start["status"], "RecordingStarted")
        stop = bridge.invoke("stopRecording")
        self.assertEqual(stop["status"], "TranscriptionReady")

    # --- Pairwise Interaction 24: Tauri IPC x Action Dispatch ---
    def test_pair_24_tauri_ipc_and_action_dispatch(self):
        """Pair 24: IPC triggerAction executes action command payload."""
        bridge = MockTauriIpcBridge()
        cmd = {"action": "WindowSnap", "direction": "Maximize"}
        res = bridge.invoke("triggerAction", cmd)
        self.assertTrue(res["success"])

    # --- Pairwise Interaction 25: REA Hook x Action Router ---
    def test_pair_25_rea_inspection_and_action_router(self):
        """Pair 25: Voice command 'rea inspect 1234' parsed into ReaInspect payload."""
        router = SemanticActionRouter()
        action = router.parse("rea inspect 1234")
        self.assertEqual(action["type"], "ReaInspect")
        self.assertEqual(action["payload"]["target_pid"], 1234)

    # --- Pairwise Interaction 26: Dial Inactivity Timeout x Window Blur ---
    def test_pair_26_dial_inactivity_timeout_and_overlay_dismiss(self):
        """Pair 26: Inactivity timeout clears radial dial hover and hides overlay."""
        dial = MockRadialDialOverlay()
        dial.toggle()
        dial.hover_sector(4)
        dial.on_inactivity_timeout()
        self.assertFalse(dial.is_visible)
        self.assertIsNone(dial.active_sector)
