"""
test_t4_real_world_r3_scenarios.py - Tier 4 Real-World Workload Scenarios (Features 33–61)

Comprehensive End-to-End User Journeys covering:
14. Handy Parity Complete User Journey (Model discovery, Parakeet V3, dictionary substitution, reliable paste, history indexing)
15. 100% Hands-Free Voice Coding & Navigation Journey (Launch app auto-centered, resize 1280x720, snap left, UI field nav, scroll)
16. Meetily Live Meeting Assistant Full Lifecycle (Dual mic+loopback mixing, voice ducking, live transcription, Ollama summary, PDF export)
17. Continuous Virtual Camera & MediaPipe Hand Tracking While On Live Meeting (Zero-lock fanout, gesture execution during meeting)
18. Virtual Camera Independent Privacy Mute with Uninterrupted Local Hand Tracking (Meeting muted with privacy slate, local gestures active)
19. Audio File Import & Enhance Workflow (.m4a import, Whisper Medium transcription, Claude technical decisions summary, DOCX export)
20. Hands-Free Window Snapping Multi-Step Flow (Sequential 9-way snapping: left, right, top, bottom, center, maximize, restore)
21. Smart Recording Mode Dynamic Switching & VAD Bursts (SmartDualMode -> HoldOnly -> ToggleOnly -> ContinuousHandsFree)
22. Overlay HUD Repositioning Across Display Modes (Bottom -> Top -> Center -> None zero focus stealing)
23. Local Meeting Archive Search and Synchronized Audio Seek (Full-text search, transcript chunk match, audio offset seek)
24. Headless IPC Bridge End-to-End Automation Pipeline (CI-driven control of STT, Actions, Meetings, Camera without display server)
25. Multi-Provider AI Summarization Cross-Verification (Ollama, Claude, Groq, OpenRouter, CustomOpenAI)
26. Two-Handed Gesture Interaction Journey (Left hand radial dial coordinate + Right hand pinch click)
27. High-Volume Meeting Audio Burst with Anti-Clipping Normalizer (Extreme audio overload strictly bounded in [-1.0, 1.0])
28. Unified 100% Hands-Free Developer Workday (Speech-to-text + Speech-to-action + Virtual Camera Splitter + Meetily AI Assistant)

Total Tests: 15 tests
"""

import os
import tempfile
import unittest
from tests.e2e.e2e_harness import (
    AdvancedGestureHeuristics,
    AirCtrlCliParser,
    AiSummaryService,
    DictionaryService,
    DocumentExporter,
    DualAudioMixer,
    ExportFormat,
    HandsFreeCommandParser,
    HeadlessIpcBridgeR3,
    Landmark3D,
    LiveMeetingTranscriber,
    LiveTranscriptChunk,
    MeetingArchiveEntry,
    MeetingArchiveStore,
    MeetingWatcher,
    ModelDiscoveryService,
    MultiHandLandmarks,
    OverlayGeometryManager,
    OverlayPosition,
    RecordingMode,
    RecordingModeController,
    ReliablePasteManager,
    SnapPosition,
    SpatialRadialDialMapper,
    SummaryProvider,
    SummaryTemplate,
    TranscriptHistoryStore,
    TranscriptItem,
    VirtualCameraSplitter,
    WhisperModelVariant,
    WindowGeometryEngine,
    create_mock_hand_landmarks,
)


class TestT4RealWorldR3Scenarios(unittest.TestCase):
    """
    Tier 4: 15 Real-World Workload Scenarios for Features 33–61
    """

    def test_scenario_14_handy_parity_model_selection_and_paste_journey(self):
        """Scenario 14: Complete Handy workflow - model discovery, Parakeet V3, dictionary, reliable paste, history."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            model_file = os.path.join(tmp_dir, "parakeet-unified-en-0.6b-q8_0.gguf")
            with open(model_file, "wb") as f:
                f.write(b"\x00" * 4096)

            # 1. Discover models
            discovered = ModelDiscoveryService.discover_models([tmp_dir])
            self.assertEqual(len(discovered), 1)
            self.assertEqual(discovered[0].variant, WhisperModelVariant.PARAKEET_V3)

            # 2. Recording mode controller
            rec_ctrl = RecordingModeController(RecordingMode.SMART_DUAL_MODE)
            rec_ctrl.on_key_down(1000.0)
            rec_ctrl.on_key_up(1400.0)  # PTT release
            self.assertEqual(rec_ctrl.transcribed_count, 1)

            # 3. Vocabulary replacement
            dict_svc = DictionaryService({"air control": "airctrl", "tauri to": "Tauri 2.0"})
            dict_svc.play_cue("recording_stop")
            text = dict_svc.replace_words("testing air control with tauri to")
            self.assertEqual(text, "Testing airctrl with Tauri 2.0")

            # 4. Reliable paste with clipboard restoration
            paste_mgr = ReliablePasteManager(delay_before_ms=50, delay_after_ms=50, reliable_paste=True)
            paste_mgr.clipboard_content = "original_user_snippet"
            event = paste_mgr.paste_text(text)
            self.assertTrue(event["restored_clipboard"])
            self.assertEqual(paste_mgr.clipboard_content, "original_user_snippet")

            # 5. History store indexing
            history = TranscriptHistoryStore()
            history.store(TranscriptItem("item_14", 1400.0, text, "testing air control with tauri to", 400, "parakeet-v3", 0.99))
            results = history.search("Tauri 2.0")
            self.assertEqual(len(results), 1)

    def test_scenario_15_hands_free_voice_coding_and_docking_journey(self):
        """Scenario 15: Voice-driven app launch, sizing to 1280x720, docking left, form navigation and scrolling."""
        parser = HandsFreeCommandParser()
        engine = WindowGeometryEngine(1920, 1080)

        # 1. Launch editor auto-centered
        cmd_open = parser.parse("open code")
        self.assertEqual(cmd_open.action_type, "LaunchApp")
        self.assertTrue(cmd_open.payload["auto_center"])

        # 2. Resize to 1280x720
        cmd_resize = parser.parse("resize 1280 by 720")
        b1 = engine.compute_resize({"x": 100, "y": 100, "width": 800, "height": 600}, cmd_resize)
        self.assertEqual(b1["width"], 1280)
        self.assertEqual(b1["height"], 720)

        # 3. Snap left
        cmd_snap = parser.parse("snap left")
        b2 = engine.compute_snap(SnapPosition(cmd_snap.payload["position"]))
        self.assertEqual(b2["x"], 0)
        self.assertEqual(b2["width"], 960)

        # 4. Form navigation & scroll
        cmd_field = parser.parse("next field")
        self.assertEqual(cmd_field.payload["element"], "NextField")

        cmd_enter = parser.parse("press enter")
        self.assertEqual(cmd_enter.payload["element"], "Enter")

        cmd_scroll = parser.parse("scroll down 10")
        self.assertEqual(cmd_scroll.payload["amount"], 10)

    def test_scenario_16_meetily_live_meeting_assistant_full_lifecycle(self):
        """Scenario 16: Live meeting assistant lifecycle with ducking, transcription, summary and PDF export."""
        mixer = DualAudioMixer(sample_rate=16000, ducking_gain=0.25)
        transcriber = LiveMeetingTranscriber()

        # 1. Dual stream mixing with speech ducking
        mic_speech = [0.8] * 1600  # 100ms of active host speech
        loopback_audio = [0.5] * 1600
        mixed = mixer.mix(mic_speech, loopback_audio)
        self.assertEqual(len(mixed), 1600)
        self.assertLess(mixer.current_duck_gain, 0.50)

        # 2. Live transcript chunk attribution
        c1 = transcriber.feed_audio(mic_energy=0.8, loopback_energy=0.1, duration_ms=2000, transcribed_text="Architecture status update")
        self.assertEqual(c1.speaker, "Host")

        c2 = transcriber.feed_audio(mic_energy=0.05, loopback_energy=0.7, duration_ms=2000, transcribed_text="Reviewing requirements document")
        self.assertEqual(c2.speaker, "Participant")

        # 3. Generate summary via local Ollama
        full_transcript = f"{c1.speaker}: {c1.text}\n{c2.speaker}: {c2.text}"
        summarizer = AiSummaryService(SummaryProvider.OLLAMA)
        summary = summarizer.generate_summary(full_transcript, SummaryTemplate.EXECUTIVE_SUMMARY)
        self.assertIn("Executive Summary", summary["summary_markdown"])

        # 4. Export to PDF
        pdf_doc = DocumentExporter.export(summary["summary_markdown"], ExportFormat.PDF)
        self.assertTrue(pdf_doc.startswith(b"%PDF-1.4"))
        self.assertTrue(pdf_doc.endswith(b"%%EOF\n"))

    def test_scenario_17_virtual_camera_and_mediapipe_spatial_control_during_meeting(self):
        """Scenario 17: Virtual camera streams video while MediaPipe hand gestures execute concurrently."""
        splitter = VirtualCameraSplitter()
        heuristics = AdvancedGestureHeuristics()

        # Continuous streaming to meeting app (Consumer 2) and gesture engine (Consumer 1)
        for frame in range(10):
            res = splitter.feed_camera_frame(frame)
            self.assertEqual(res["consumer_2_meeting"], f"outbound_frame_{frame}")
            self.assertEqual(res["consumer_1_mediapipe"], f"raw_frame_{frame}")

        # Gesture pointing ray computed while meeting active
        ray = heuristics.compute_pointing_ray(Landmark3D(0.5, 0.5, 0.0), Landmark3D(0.5, 0.1, 0.0))
        self.assertEqual(ray["dy"], -1.0)

        # Pinch click executed
        is_pinch = heuristics.evaluate_pinch(Landmark3D(0.5, 0.5, 0.0), Landmark3D(0.53, 0.5, 0.0))
        self.assertTrue(is_pinch)

    def test_scenario_18_virtual_camera_privacy_mute_with_uninterrupted_hand_gestures(self):
        """Scenario 18: Privacy mute active replaces meeting video while local gesture tracking continues."""
        splitter = VirtualCameraSplitter()
        splitter.set_privacy_mute(True)

        res = splitter.feed_camera_frame(42)
        # Meeting feed receives privacy slate
        self.assertEqual(res["consumer_2_meeting"], "privacy_slate_frame")
        # Local gesture engine still receives raw live camera frame
        self.assertEqual(res["consumer_1_mediapipe"], "raw_frame_42")

        # Hands-free command continues working
        parser = HandsFreeCommandParser()
        cmd = parser.parse("snap center")
        self.assertEqual(cmd.payload["position"], "Center")

    def test_scenario_19_audio_file_import_enhance_and_docx_export_journey(self):
        """Scenario 19: Audio file import, re-transcription, Claude technical summary, and DOCX export."""
        from tests.e2e.tier1_category_partition.test_t1_f51_audio_import_enhance import MockAudioImportService
        service = MockAudioImportService()
        imported = service.import_and_transcribe("/audio/architecture_sync.m4a", WhisperModelVariant.MEDIUM_EN)
        self.assertEqual(imported["format"], "m4a")

        summarizer = AiSummaryService(SummaryProvider.CLAUDE)
        summary = summarizer.generate_summary(imported["transcript"], SummaryTemplate.TECHNICAL_DECISIONS)
        self.assertIn("Technical Decisions", summary["summary_markdown"])

        docx = DocumentExporter.export(summary["summary_markdown"], ExportFormat.DOCX)
        self.assertEqual(docx[:4], b"PK\x03\x04")

    def test_scenario_20_hands_free_window_snapping_multi_step_flow(self):
        """Scenario 20: Sequential 9-way snapping workflow across all dock positions."""
        parser = HandsFreeCommandParser()
        engine = WindowGeometryEngine(1920, 1080)

        sequence = ["snap left", "snap right", "snap top", "snap bottom", "snap center", "maximize", "restore"]
        for utterance in sequence:
            cmd = parser.parse(utterance)
            self.assertEqual(cmd.action_type, "SnapWindow")
            pos = SnapPosition(cmd.payload["position"])
            bounds = engine.compute_snap(pos)
            self.assertTrue(0 <= bounds["width"] <= 1920)
            self.assertTrue(0 <= bounds["height"] <= 1080)

    def test_scenario_21_smart_recording_mode_switching_and_vad_burst(self):
        """Scenario 21: Dynamically switches recording modes and verifies VAD chunking."""
        ctrl = RecordingModeController()

        # 1. SmartDualMode
        ctrl.mode = RecordingMode.SMART_DUAL_MODE
        ctrl.on_key_down(1000.0)
        self.assertEqual(ctrl.on_key_up(1300.0), "Transcribing")

        # 2. HoldOnly
        ctrl.mode = RecordingMode.HOLD_ONLY
        ctrl.on_key_down(2000.0)
        self.assertEqual(ctrl.on_key_up(2050.0), "Transcribing")

        # 3. ToggleOnly
        ctrl.mode = RecordingMode.TOGGLE_ONLY
        ctrl.on_key_down(3000.0)
        self.assertEqual(ctrl.state, "Recording")
        ctrl.on_key_down(3500.0)
        self.assertEqual(ctrl.state, "Transcribing")

        # 4. ContinuousHandsFree VAD
        ctrl.mode = RecordingMode.CONTINUOUS_HANDS_FREE
        ctrl.on_vad_speech_detected(0.12)
        self.assertEqual(ctrl.state, "Recording")
        ctrl.on_vad_speech_detected(0.01)
        self.assertEqual(ctrl.state, "Transcribing")

    def test_scenario_22_overlay_hud_repositioning_across_display_modes(self):
        """Scenario 22: Overlay HUD positions tested across Bottom, Top, Center, and None."""
        for pos in [OverlayPosition.BOTTOM, OverlayPosition.TOP, OverlayPosition.CENTER, OverlayPosition.NONE]:
            bounds = OverlayGeometryManager.calculate_bounds(pos, 1920, 1080)
            if pos == OverlayPosition.NONE:
                self.assertFalse(bounds["visible"])
                self.assertFalse(bounds["focus_stealing"])
            else:
                self.assertTrue(bounds["visible"])
                self.assertFalse(bounds["focus_stealing"])

    def test_scenario_23_local_meeting_archive_search_and_synchronized_audio_seek(self):
        """Scenario 23: Meeting archived, full-text searched, and audio offset retrieved for playback."""
        store = MeetingArchiveStore()
        chunk = LiveTranscriptChunk("c_23", "Host", "Crucial budget decision at 12000ms", 12000, 15000, 0.98)
        entry = MeetingArchiveEntry("m_23", "Budget Review", "2026-10-14", 600, [chunk], "# Budget Summary", "/audio/m23.wav", ["Host"])
        store.save_meeting(entry)

        res = store.search("budget decision")
        self.assertEqual(len(res), 1)

        offset = store.seek_audio_offset("m_23", "c_23")
        self.assertEqual(offset, 12000)

    def test_scenario_24_headless_ipc_bridge_end_to_end_automation_pipeline(self):
        """Scenario 24: CI automation drives complete lifecycle via headless pure-Rust IPC bridge."""
        bridge = HeadlessIpcBridgeR3()

        # Step 1: Select Model
        r1 = bridge.dispatch("selectModel", {"model": "parakeet-unified-en-0.6b-q8_0"})
        self.assertEqual(r1["selected_model"], "parakeet-unified-en-0.6b-q8_0")

        # Step 2: Set Mode & Overlay
        r2 = bridge.dispatch("setRecordingMode", {"mode": "ContinuousHandsFree"})
        self.assertEqual(r2["mode"], "ContinuousHandsFree")

        r3 = bridge.dispatch("setOverlayPosition", {"position": "None"})
        self.assertFalse(r3["bounds"]["visible"])

        # Step 3: Execute Action
        r4 = bridge.dispatch("executeVoiceCommand", {"utterance": "snap center"})
        self.assertEqual(r4["action"], "SnapWindow")

        # Step 4: Summary & Export
        r5 = bridge.dispatch("generateSummary", {"transcript": "All features verified", "template": "ExecutiveSummary"})
        self.assertIn("Executive Summary", r5["summary"]["summary_markdown"])

        # Step 5: Camera Privacy
        r6 = bridge.dispatch("setVirtualCameraPrivacy", {"privacy_muted": True})
        self.assertTrue(r6["privacy_muted"])

    def test_scenario_25_multiprovider_ai_summarization_cross_verification(self):
        """Scenario 25: Same transcript verified across Ollama, Claude, Groq, OpenRouter, CustomOpenAI."""
        transcript = "Cross-provider benchmark transcript."
        for prov in [SummaryProvider.OLLAMA, SummaryProvider.CLAUDE, SummaryProvider.GROQ,
                     SummaryProvider.OPENROUTER, SummaryProvider.CUSTOM_OPENAI]:
            svc = AiSummaryService(prov)
            res = svc.generate_summary(transcript, SummaryTemplate.EXECUTIVE_SUMMARY)
            self.assertEqual(res["provider"], prov.value)
            self.assertIn("# Executive Summary", res["summary_markdown"])

    def test_scenario_26_two_handed_gesture_interaction_journey(self):
        """Scenario 26: Left hand sets dial angle while right hand performs pinch click."""
        dial = SpatialRadialDialMapper(sector_count=6)
        mapped = dial.map_coords(x=0.50, y=0.85)  # pointing down -> sector 1 or 2
        self.assertFalse(mapped["in_deadzone"])

        heuristics = AdvancedGestureHeuristics()
        is_pinch = heuristics.evaluate_pinch(Landmark3D(0.5, 0.5, 0.0), Landmark3D(0.52, 0.5, 0.0))
        self.assertTrue(is_pinch)

    def test_scenario_27_high_volume_meeting_audio_burst_with_anti_clipping(self):
        """Scenario 27: Overload audio spikes processed through tanh limiter bounded in [-1.0, 1.0]."""
        mixer = DualAudioMixer()
        overload_mic = [20.0, -25.0, 50.0, -100.0] * 25
        overload_lp = [15.0, -30.0, 40.0, -80.0] * 25

        mixed = mixer.mix(overload_mic, overload_lp)
        self.assertEqual(len(mixed), 100)
        for s in mixed:
            self.assertTrue(-1.0 <= s <= 1.0)

    def test_scenario_28_unified_100_percent_hands_free_developer_workday(self):
        """Scenario 28: Full workday journey - STT dictation, window tiling, camera gestures, meeting assistant."""
        # 1. Voice launch & snap
        parser = HandsFreeCommandParser()
        engine = WindowGeometryEngine(1920, 1080)
        cmd1 = parser.parse("hey airctrl open code")
        self.assertEqual(cmd1.action_type, "LaunchApp")
        cmd2 = parser.parse("hey airctrl snap left")
        b = engine.compute_snap(SnapPosition(cmd2.payload["position"]))
        self.assertEqual(b["x"], 0)

        # 2. Dictate code with reliable paste
        dict_svc = DictionaryService({"todo": "TODO:"})
        text = dict_svc.replace_words("todo implement error handler")
        paste = ReliablePasteManager(reliable_paste=True)
        paste.clipboard_content = "existing_code"
        ev = paste.paste_text(text)
        self.assertEqual(ev["pasted_text"], "TODO: implement error handler")
        self.assertEqual(paste.clipboard_content, "existing_code")

        # 3. Virtual camera & meeting assistant
        splitter = VirtualCameraSplitter()
        frame = splitter.feed_camera_frame(1)
        self.assertIsNotNone(frame["consumer_2_meeting"])

        mixer = DualAudioMixer()
        transcriber = LiveMeetingTranscriber()
        mixed_audio = mixer.mix([0.5] * 100, [0.3] * 100)
        chunk = transcriber.feed_audio(0.5, 0.1, duration_ms=1000, transcribed_text="Workday wrap-up")
        self.assertEqual(chunk.speaker, "Host")
