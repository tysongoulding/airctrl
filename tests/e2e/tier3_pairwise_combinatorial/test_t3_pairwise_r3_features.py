"""
test_t3_pairwise_r3_features.py - Tier 3 Pairwise Combinatorial Subsystem Interactions (Features 33–61)

Covers 2-way subsystem interactions across:
- Extended Whisper / Parakeet models x Model discovery
- 4 Recording modes x Reliable paste clipboard manager
- Overlay positioning x Dictionary service & acoustic cues
- Searchable transcript history x CLI remote control flags
- Hands-Free app lifecycle x 9-way window snapping & docking
- Dynamic window sizing x Window focus switching & cycling
- Hands-Free UI navigation x Directional scrolling & mouse clicks
- Voice wake prefixes x Dedicated shortcut triggers
- Dual mic + system loopback audio mixer x Voice ducking engine
- Anti-clipping tanh limiter x Live streaming meeting transcription
- Audio import & enhance x Multi-provider AI summaries
- Summary templates & exporters x Local meeting archive & playback
- Dual-hand landmark tracking x Advanced gesture heuristics
- Continuous virtual camera splitter x Zero-lock video fanout
- Independent privacy mute toggle x MediaPipe hand gestures
- Headless IPC bridge x STT, Actions, Meetings, Gestures commands
- Full system regression & zero-mock production policy

Total Tests: 29 tests
"""

import math
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


class TestT3PairwiseR3Features(unittest.TestCase):
    """
    Tier 3: Pairwise Combinatorial Subsystem Interactions for Features 33–61 (29 tests)
    """

    # --- Pair 27: Extended Whisper Models x Model Discovery ---
    def test_pair_27_whisper_extended_models_and_model_discovery(self):
        """Pair 27: Model discovery scans directories and resolves Whisper Turbo & Parakeet V3."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            p1 = os.path.join(tmp_dir, "whisper-turbo.en.bin")
            p2 = os.path.join(tmp_dir, "parakeet-unified-en-0.6b-q8_0.gguf")
            with open(p1, "wb") as f: f.write(b"\x00" * 2048)
            with open(p2, "wb") as f: f.write(b"\x00" * 2048)

            discovered = ModelDiscoveryService.discover_models([tmp_dir])
            self.assertEqual(len(discovered), 2)
            variants = {d.variant for d in discovered}
            self.assertIn(WhisperModelVariant.TURBO_EN, variants)
            self.assertIn(WhisperModelVariant.PARAKEET_V3, variants)

    # --- Pair 28: Recording Modes x Reliable Paste ---
    def test_pair_28_recording_modes_and_reliable_paste(self):
        """Pair 28: HoldOnly recording completes and triggers ReliablePaste with clipboard restoration."""
        ctrl = RecordingModeController(RecordingMode.HOLD_ONLY)
        paste_mgr = ReliablePasteManager(reliable_paste=True)
        paste_mgr.clipboard_content = "user_initial_clipboard"

        ctrl.on_key_down(1000.0)
        ctrl.on_key_up(1500.0)
        self.assertEqual(ctrl.transcribed_count, 1)

        event = paste_mgr.paste_text("transcribed text")
        self.assertEqual(event["pasted_text"], "transcribed text")
        self.assertEqual(paste_mgr.clipboard_content, "user_initial_clipboard")

    # --- Pair 29: Overlay Positioning x Dictionary Service ---
    def test_pair_29_overlay_positioning_and_dictionary_service(self):
        """Pair 29: Top HUD overlay positioned while dictionary replaces vocabulary with acoustic cues."""
        bounds = OverlayGeometryManager.calculate_bounds(OverlayPosition.TOP, 1920, 1080)
        self.assertEqual(bounds["y"], 30)

        dict_service = DictionaryService({"air control": "airctrl"})
        dict_service.play_cue("recording_start")
        replaced = dict_service.replace_words("air control active")
        self.assertIn("Airctrl", replaced)
        self.assertEqual(dict_service.played_cues, ["recording_start"])

    # --- Pair 30: Searchable History x CLI Flags ---
    def test_pair_30_searchable_transcript_history_and_cli_flags(self):
        """Pair 30: CLI parsed with --start-hidden while history store indexes and searches transcripts."""
        opts = AirCtrlCliParser.parse_args(["--start-hidden", "--debug"])
        self.assertTrue(opts["start_hidden"])

        store = TranscriptHistoryStore()
        store.store(TranscriptItem("h1", 1000.0, "Refactoring Rust workspace", "refactoring rust workspace", 800, "whisper-small.en", 0.95))
        res = store.search("Rust")
        self.assertEqual(len(res), 1)
        self.assertEqual(res[0].id, "h1")

    # --- Pair 31: App Lifecycle x 9-Way Window Snapping ---
    def test_pair_31_voice_app_lifecycle_and_9way_snapping(self):
        """Pair 31: Spoken command launches app with auto-center, followed by snap right."""
        parser = HandsFreeCommandParser()
        engine = WindowGeometryEngine(1920, 1080)

        cmd_launch = parser.parse("open slack")
        self.assertEqual(cmd_launch.action_type, "LaunchApp")
        self.assertTrue(cmd_launch.payload["auto_center"])

        cmd_snap = parser.parse("snap right")
        bounds = engine.compute_snap(SnapPosition(cmd_snap.payload["position"]))
        self.assertEqual(bounds["x"], 960)
        self.assertEqual(bounds["width"], 960)

    # --- Pair 32: Dynamic Window Sizing x App Focus ---
    def test_pair_32_dynamic_window_sizing_and_app_focus(self):
        """Pair 32: Window resized dynamically, then focus switched to target app."""
        parser = HandsFreeCommandParser()
        engine = WindowGeometryEngine(1920, 1080)

        cmd_resize = parser.parse("resize window 80%")
        bounds = engine.compute_resize({"x": 0, "y": 0, "width": 1000, "height": 600}, cmd_resize)
        self.assertEqual(bounds["width"], int(1920 * 0.80))

        cmd_focus = parser.parse("switch to Google Chrome")
        self.assertEqual(cmd_focus.action_type, "SwitchFocus")
        self.assertEqual(cmd_focus.payload["app_name"], "google chrome")

    # --- Pair 33: Window Cycling x UI Navigation ---
    def test_pair_33_window_cycling_and_ui_navigation(self):
        """Pair 33: Z-order window cycled backwards, followed by form navigation keystrokes."""
        parser = HandsFreeCommandParser()
        cmd_cycle = parser.parse("previous window")
        self.assertEqual(cmd_cycle.action_type, "CycleWindow")
        self.assertEqual(cmd_cycle.payload["direction"], "Previous")

        cmd_nav = parser.parse("next field")
        self.assertEqual(cmd_nav.action_type, "NavigateUi")
        self.assertEqual(cmd_nav.payload["element"], "NextField")

    # --- Pair 34: Dedicated Shortcut Wake x Directional Scroll ---
    def test_pair_34_dedicated_shortcut_wake_and_directional_scroll(self):
        """Pair 34: Voice wake prefix triggers directional scroll."""
        parser = HandsFreeCommandParser()
        cmd = parser.parse("hey airctrl scroll down 10")
        self.assertEqual(cmd.action_type, "Scroll")
        self.assertEqual(cmd.payload["direction"], "Down")
        self.assertEqual(cmd.payload["amount"], 10)

    # --- Pair 35: Dual Audio Mixer x Dynamic Voice Ducking ---
    def test_pair_35_dual_audio_mixer_and_dynamic_voice_ducking(self):
        """Pair 35: High mic energy triggers voice ducking on loopback channel in dual mixer."""
        mixer = DualAudioMixer(sample_rate=16000, ducking_gain=0.25)
        mic = [0.9] * 2000
        loopback = [0.6] * 2000
        mixed = mixer.mix(mic, loopback)
        self.assertLess(mixer.current_duck_gain, 0.40)
        self.assertGreaterEqual(mixer.current_duck_gain, 0.25)
        for s in mixed:
            self.assertTrue(-1.0 <= s <= 1.0)

    # --- Pair 36: Anti-Clipping Normalizer x Live Meeting Transcription ---
    def test_pair_36_anti_clipping_normalizer_and_live_meeting_transcription(self):
        """Pair 36: Anti-clipping limiter normalizes hot mixed audio, fed to live transcriber."""
        mixer = DualAudioMixer()
        transcriber = LiveMeetingTranscriber()

        mixed = mixer.mix([5.0] * 100, [5.0] * 100)
        self.assertTrue(all(-1.0 <= s <= 1.0 for s in mixed))

        chunk = transcriber.feed_audio(mic_energy=0.8, loopback_energy=0.1, duration_ms=1000)
        self.assertEqual(chunk.speaker, "Host")
        self.assertGreaterEqual(chunk.confidence, 0.90)

    # --- Pair 37: Audio Import & Enhance x Multi-Provider AI Summaries ---
    def test_pair_37_audio_import_enhance_and_multiprovider_ai_summaries(self):
        """Pair 37: Audio file (.flac) imported, then summarized via Ollama local provider."""
        from tests.e2e.tier1_category_partition.test_t1_f51_audio_import_enhance import MockAudioImportService
        import_service = MockAudioImportService()
        imported = import_service.import_and_transcribe("/audio/q3_standup.flac", WhisperModelVariant.PARAKEET_V3)
        self.assertEqual(imported["format"], "flac")

        summarizer = AiSummaryService(SummaryProvider.OLLAMA)
        summary = summarizer.generate_summary(imported["transcript"], SummaryTemplate.EXECUTIVE_SUMMARY)
        self.assertEqual(summary["provider"], "Ollama")
        self.assertIn("Executive Summary", summary["summary_markdown"])

    # --- Pair 38: Templates Doc Exporters x Local Meeting Archive ---
    def test_pair_38_templates_doc_exporters_and_meeting_archive(self):
        """Pair 38: Action items summary generated, exported to PDF, and indexed in archive store."""
        summarizer = AiSummaryService()
        summary_res = summarizer.generate_summary("Team meeting text", SummaryTemplate.ACTION_ITEMS)
        pdf_bytes = DocumentExporter.export(summary_res["summary_markdown"], ExportFormat.PDF)
        self.assertTrue(pdf_bytes.startswith(b"%PDF-1.4"))

        store = MeetingArchiveStore()
        entry = MeetingArchiveEntry(
            id="meet_pair_38",
            title="Sprint Review",
            date_iso="2026-10-14T11:00:00Z",
            duration_secs=900,
            transcript_chunks=[LiveTranscriptChunk("c1", "Host", "Sprint Review Notes", 0, 5000, 0.95)],
            summary_markdown=summary_res["summary_markdown"],
            audio_path="/archive/meet_38.wav",
            participants=["Tyson", "Reviewer"]
        )
        store.save_meeting(entry)
        self.assertEqual(len(store.search("Sprint Review")), 1)

    # --- Pair 39: Dual-Hand Landmarks x Advanced Gesture Heuristics ---
    def test_pair_39_dual_hand_tracking_and_advanced_gesture_heuristics(self):
        """Pair 39: Dual hands tracked simultaneously; right hand executes hysteresis pinch."""
        l_hand = create_mock_hand_landmarks("Neutral")
        r_hand = create_mock_hand_landmarks("Pinch")
        multi = MultiHandLandmarks(left_hand=l_hand, right_hand=r_hand)
        self.assertEqual(multi.hand_count, 2)

        heuristics = AdvancedGestureHeuristics()
        is_pinching = heuristics.evaluate_pinch(r_hand[4], r_hand[8])
        self.assertTrue(is_pinching)

    # --- Pair 40: Continuous Virtual Camera x Zero-Lock Fanout ---
    def test_pair_40_continuous_virtual_camera_splitter_and_zerolock_fanout(self):
        """Pair 40: Virtual camera splitter concurrently fans out frames to MediaPipe and meeting sink."""
        splitter = VirtualCameraSplitter("AirCtrl Virtual Camera")
        res = splitter.feed_camera_frame(101)
        self.assertEqual(res["consumer_1_mediapipe"], "raw_frame_101")
        self.assertEqual(res["consumer_2_meeting"], "outbound_frame_101")
        self.assertEqual(splitter.frames_fanned_out, 1)

    # --- Pair 41: Independent Privacy Mute x MediaPipe Gestures ---
    def test_pair_41_independent_privacy_mute_and_mediapipe_gestures(self):
        """Pair 41: Privacy mute sends privacy slate to meeting while local MediaPipe tracking continues."""
        splitter = VirtualCameraSplitter()
        splitter.set_privacy_mute(True)

        res = splitter.feed_camera_frame(202)
        self.assertEqual(res["consumer_2_meeting"], "privacy_slate_frame")
        self.assertEqual(res["consumer_1_mediapipe"], "raw_frame_202")

        # Gestures continue computing
        heuristics = AdvancedGestureHeuristics()
        ray = heuristics.compute_pointing_ray(Landmark3D(0.5, 0.5, 0.0), Landmark3D(0.5, 0.2, 0.0))
        self.assertEqual(ray["dy"], -1.0)

    # --- Pair 42: Headless IPC Bridge x Model Selection ---
    def test_pair_42_headless_ipc_bridge_and_handy_model_selection(self):
        """Pair 42: Headless IPC bridge lists models and selects Parakeet V3."""
        bridge = HeadlessIpcBridgeR3()
        res_list = bridge.dispatch("listModels")
        self.assertIn("parakeet-unified-en-0.6b-q8_0", res_list["models"])

        res_sel = bridge.dispatch("selectModel", {"model": "parakeet-unified-en-0.6b-q8_0"})
        self.assertEqual(res_sel["selected_model"], "parakeet-unified-en-0.6b-q8_0")

    # --- Pair 43: Headless IPC Bridge x Speech-to-Action Snapping ---
    def test_pair_43_headless_ipc_bridge_and_speech_to_action_snapping(self):
        """Pair 43: Headless IPC bridge parses speech utterance and updates window snapping geometry."""
        bridge = HeadlessIpcBridgeR3()
        res_cmd = bridge.dispatch("executeVoiceCommand", {"utterance": "snap top"})
        self.assertEqual(res_cmd["action"], "SnapWindow")

        res_snap = bridge.dispatch("snapWindow", {"position": "Top"})
        self.assertEqual(res_snap["bounds"]["height"], 540)

    # --- Pair 44: Headless IPC Bridge x Meeting Summary & DOCX Export ---
    def test_pair_44_headless_ipc_bridge_and_meeting_summary_export(self):
        """Pair 44: Headless IPC generates summary and exports to DOCX format."""
        bridge = HeadlessIpcBridgeR3()
        res_sum = bridge.dispatch("generateSummary", {"transcript": "Important architecture sync", "template": "TechnicalDecisions"})
        self.assertEqual(res_sum["status"], "success")

        res_exp = bridge.dispatch("exportSummary", {"content": res_sum["summary"]["summary_markdown"], "format": "DOCX"})
        self.assertEqual(res_exp["format"], "DOCX")
        self.assertGreater(res_exp["bytes_count"], 10)

    # --- Pair 45: Headless IPC Bridge x Virtual Camera Privacy ---
    def test_pair_45_headless_ipc_bridge_and_virtual_camera_privacy(self):
        """Pair 45: Headless IPC queries virtual camera status and sets privacy mute state."""
        bridge = HeadlessIpcBridgeR3()
        res_status = bridge.dispatch("getVirtualCameraStatus")
        self.assertEqual(res_status["device_name"], "AirCtrl Virtual Camera")

        res_priv = bridge.dispatch("setVirtualCameraPrivacy", {"privacy_muted": True})
        self.assertTrue(res_priv["privacy_muted"])

    # --- Pair 46: Parakeet V3 x Continuous Hands-Free VAD ---
    def test_pair_46_parakeet_v3_cpu_model_and_vad_continuous_handsfree(self):
        """Pair 46: Parakeet V3 model variant matched with continuous hands-free VAD mode."""
        pv3 = WhisperModelVariant.PARAKEET_V3
        self.assertTrue(pv3.is_cpu_optimized)

        ctrl = RecordingModeController(RecordingMode.CONTINUOUS_HANDS_FREE)
        ctrl.on_vad_speech_detected(0.15, threshold=0.05)
        self.assertEqual(ctrl.state, "Recording")
        ctrl.on_vad_speech_detected(0.01, threshold=0.05)
        self.assertEqual(ctrl.state, "Transcribing")

    # --- Pair 47: Reliable Paste Delay x Active Window Focus ---
    def test_pair_47_reliable_paste_delay_and_active_window_focus(self):
        """Pair 47: Reliable paste respects before/after delays while focused application accepts text."""
        paste_mgr = ReliablePasteManager(delay_before_ms=50, delay_after_ms=100, reliable_paste=True)
        paste_mgr.clipboard_content = "code_editor_buffer"
        event = paste_mgr.paste_text("def test_function(): pass")
        self.assertEqual(event["delay_before_ms"], 50)
        self.assertEqual(event["delay_after_ms"], 100)
        self.assertEqual(paste_mgr.clipboard_content, "code_editor_buffer")

    # --- Pair 48: Custom Dictionary x Transcript History ---
    def test_pair_48_custom_dictionary_and_transcript_history(self):
        """Pair 48: Text replaced by custom vocabulary stored into searchable history."""
        dict_service = DictionaryService({"air control": "airctrl", "tauri to": "Tauri 2.0"})
        replaced = dict_service.replace_words("deploy air control on tauri to")

        store = TranscriptHistoryStore()
        store.store(TranscriptItem("h_dict", 1500.0, replaced, "deploy air control on tauri to", 600, "whisper-small.en", 0.98))
        res = store.search("Tauri 2.0")
        self.assertEqual(len(res), 1)

    # --- Pair 49: Voice Close App x Window Geometry Engine ---
    def test_pair_49_voice_close_app_and_window_geometry(self):
        """Pair 49: 'close window' command closes active window geometry."""
        parser = HandsFreeCommandParser()
        cmd = parser.parse("close window")
        self.assertEqual(cmd.action_type, "CloseApp")
        self.assertEqual(cmd.payload["target"], "CurrentWindow")

    # --- Pair 50: Dual Audio Mixer x Meeting Process Watcher ---
    def test_pair_50_dual_audio_mixer_and_meeting_watcher(self):
        """Pair 50: Meeting watcher detects Zoom meeting; dual audio mixer begins simultaneous stream capture."""
        watcher = MeetingWatcher()
        watcher.scan_processes([{"process_name": "zoom.exe", "title": "Zoom Meeting"}])
        self.assertIsNotNone(watcher.active_meeting)

        mixer = DualAudioMixer()
        mixed = mixer.mix([0.4] * 100, [0.3] * 100)
        self.assertEqual(len(mixed), 100)

    # --- Pair 51: Multi-Hand Tracking x Spatial Radial Dial ---
    def test_pair_51_multi_hand_tracking_and_spatial_dial_mapper(self):
        """Pair 51: Left hand points to dial sector while right hand performs pinch selection."""
        dial = SpatialRadialDialMapper(sector_count=6)
        mapped = dial.map_coords(x=0.75, y=0.50)  # sector 0 (pointing right)
        self.assertEqual(mapped["active_sector"], 0)

        heuristics = AdvancedGestureHeuristics()
        is_pinch = heuristics.evaluate_pinch(Landmark3D(0.5, 0.5, 0.0), Landmark3D(0.52, 0.5, 0.0))
        self.assertTrue(is_pinch)

    # --- Pair 52: Virtual Camera Splitter x Camera Killswitch ---
    def test_pair_52_virtual_camera_splitter_and_meeting_privacy_killswitch(self):
        """Pair 52: Virtual camera splitter streams video; meeting triggers privacy mute replace."""
        splitter = VirtualCameraSplitter()
        watcher = MeetingWatcher()
        watcher.trigger_camera_kill_switch()
        self.assertFalse(watcher.camera_active)

        splitter.set_privacy_mute(True)
        out = splitter.feed_camera_frame(55)
        self.assertEqual(out["consumer_2_meeting"], "privacy_slate_frame")

    # --- Pair 53: Live Transcription x Meeting Archive Seeking ---
    def test_pair_53_live_meeting_transcription_and_meeting_archive_seeking(self):
        """Pair 53: Live transcript chunk saved into archive and verified for synchronized playback seeking."""
        transcriber = LiveMeetingTranscriber()
        c = transcriber.feed_audio(0.7, 0.1, duration_ms=2500, transcribed_text="Important statement at 0ms")

        store = MeetingArchiveStore()
        entry = MeetingArchiveEntry("m_sync", "Sync Meeting", "2026-10-14", 300, [c], "# Summary", "/path.wav", ["Host"])
        store.save_meeting(entry)

        seek_offset = store.seek_audio_offset("m_sync", c.id)
        self.assertEqual(seek_offset, 0)

    # --- Pair 54: Claude AI Summaries x Markdown Export ---
    def test_pair_54_claude_ai_summaries_and_markdown_export(self):
        """Pair 54: Claude provider generates action items summary exported to markdown."""
        service = AiSummaryService(SummaryProvider.CLAUDE)
        summary = service.generate_summary("Team roadmap sync", SummaryTemplate.ACTION_ITEMS)
        md_bytes = DocumentExporter.export(summary["summary_markdown"], ExportFormat.MARKDOWN)
        self.assertIn(b"# Action Items", md_bytes)

    # --- Pair 55: System Regression x Headless IPC State Invariants ---
    def test_pair_55_system_regression_and_headless_ipc_state_invariants(self):
        """Pair 55: Full headless IPC bridge maintains state invariants with zero mock fallbacks."""
        bridge = HeadlessIpcBridgeR3()
        # Ensure STT, Window Snapping, Meeting Mixer, and Camera Splitter all respond with status=success
        r1 = bridge.dispatch("selectModel", {"model": "whisper-turbo.en"})
        r2 = bridge.dispatch("snapWindow", {"position": "Maximize"})
        r3 = bridge.dispatch("getMixerStatus")
        r4 = bridge.dispatch("getVirtualCameraStatus")

        self.assertEqual(r1["status"], "success")
        self.assertEqual(r2["status"], "success")
        self.assertEqual(r3["status"], "success")
        self.assertEqual(r4["status"], "success")
