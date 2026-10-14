/**
 * Mock Tauri IPC Bridge implementing all commands for headless CI and browser preview.
 * Strictly complies with tests/e2e/tier1_category_partition/test_t1_f24_tauri_ipc_bridge.py.
 */
export class MockTauriIpcBridge {
  private dictionary: Record<string, string> = {
    "air control": "airctrl",
    "handy app": "Handy",
    "meetily app": "Meetily",
  };
  private postProcessing = true;
  private debugDrawer = false;
  private recordingMeeting = false;
  private privacyMuted = false;
  private muteBoth = false;
  private trackingMode: "SingleHand" | "DualHand" = "DualHand";

  private handlers: Record<string, (args: any) => any> = {
    startRecording: () => ({ status: "RecordingStarted" }),
    stopRecording: () => ({ status: "TranscriptionReady", text: "hello" }),
    triggerAction: (args: any) => ({ success: true, action: args?.action ?? "WindowSnap" }),
    toggleRadialDial: () => ({ dial_visible: true }),
    toggleSttOverlay: () => ({ stt_overlay_visible: true }),
    toggleCommandPalette: () => ({ command_palette_visible: true }),
    getSettings: () => ({
      hotkey_stt: "Ctrl+Spacebar",
      hotkey_dial: "Ctrl+Alt",
      push_to_talk: true,
      camera_auto_kill: false,
      compliance_audio_enabled: true,
      radial_dial_sectors: 6,
      lan_port: 8080,
      whisper_model: "parakeet-unified-en-0.6b-Q8_0.gguf",
      theme: "dark",
      hud_opacity: 0.95,
    }),
    updateSettings: (args: any) => ({
      success: true,
      settings: {
        hotkey_stt: args?.hotkey_stt ?? "Ctrl+Spacebar",
        hotkey_dial: args?.hotkey_dial ?? "Ctrl+Alt",
        camera_auto_kill: args?.camera_auto_kill ?? false,
        compliance_audio_enabled: args?.compliance_audio_enabled ?? true,
        radial_dial_sectors: args?.radial_dial_sectors ?? 6,
      },
    }),
    getMeetingStatus: () => ({
      status: this.recordingMeeting ? "Active" : "Inactive",
      meeting: this.recordingMeeting ? { app: "Zoom", title: "Architecture Sync" } : null,
      camera_state: this.privacyMuted ? "PrivacySlate" : "Active",
    }),
    triggerCameraKillSwitch: () => ({
      success: true,
      status: "CameraKilled",
      camera_handle_closed: true,
    }),
    getCatalogedApps: () => ({
      apps: [
        { id: "app-cursor", name: "Cursor", path: "cursor.exe", icon: "cpu", category: "Developer Tools" },
        { id: "app-code", name: "Visual Studio Code", path: "code.exe", icon: "layout", category: "Developer Tools" },
        { id: "app-chrome", name: "Google Chrome", path: "chrome.exe", icon: "window", category: "Web Browsers" },
        { id: "app-terminal", name: "Windows Terminal", path: "wt.exe", icon: "terminal", category: "System Utilities" },
        { id: "app-notepad", name: "Notepad", path: "notepad.exe", icon: "list", category: "Productivity" },
        { id: "app-explorer", name: "File Explorer", path: "explorer.exe", icon: "layout", category: "System Utilities" },
      ],
    }),
    refreshCatalogedApps: () => ({
      apps: [
        { id: "app-cursor", name: "Cursor", path: "cursor.exe", icon: "cpu", category: "Developer Tools" },
        { id: "app-code", name: "Visual Studio Code", path: "code.exe", icon: "layout", category: "Developer Tools" },
        { id: "app-chrome", name: "Google Chrome", path: "chrome.exe", icon: "window", category: "Web Browsers" },
        { id: "app-terminal", name: "Windows Terminal", path: "wt.exe", icon: "terminal", category: "System Utilities" },
        { id: "app-notepad", name: "Notepad", path: "notepad.exe", icon: "list", category: "Productivity" },
        { id: "app-explorer", name: "File Explorer", path: "explorer.exe", icon: "layout", category: "System Utilities" },
      ],
    }),
    launchApp: (args: any) => ({
      success: true,
      pid: 1234,
      target: args?.path ?? "app.exe",
    }),
    injectText: (args: any) => ({
      success: true,
      status: "Success",
      method: "ClipboardPaste",
      text_length: (args?.text ?? "").length,
    }),
    inject_text: (args: any) => ({
      success: true,
      status: "Success",
      method: "ClipboardPaste",
      text_length: (args?.text ?? "").length,
    }),

    // R1: Handy Parity
    listModels: () => ({
      models: [
        { id: "parakeet-v3", name: "Parakeet Unified EN 0.6B (Q8_0)", filename: "parakeet-unified-en-0.6b-Q8_0.gguf", format: "GGUF", size_mb: 731, downloaded: true, active: true },
        { id: "whisper-small", name: "Whisper Small", filename: "ggml-small.bin", format: "GGML", size_mb: 487, downloaded: true, active: false },
        { id: "whisper-medium", name: "Whisper Medium (Q4_1)", filename: "whisper-medium-q4_1.bin", format: "GGML", size_mb: 492, downloaded: true, active: false },
        { id: "whisper-turbo", name: "Whisper Large v3 Turbo", filename: "ggml-large-v3-turbo.bin", format: "GGML", size_mb: 1600, downloaded: false, active: false },
        { id: "whisper-large", name: "Whisper Large v3 (Q5_0)", filename: "ggml-large-v3-q5_0.bin", format: "GGML", size_mb: 1100, downloaded: false, active: false },
      ],
    }),
    selectModel: (args: any) => ({ success: true, selected_model: args?.model_id ?? "parakeet-v3" }),
    setRecordingMode: (args: any) => ({ success: true, mode: args?.mode ?? "SmartDualMode" }),
    getRecordingMode: () => ({ mode: "SmartDualMode" }),
    setOverlayPosition: (args: any) => ({ success: true, position: args?.position ?? "Bottom" }),
    getOverlayPosition: () => ({ position: "Bottom" }),
    updateDictionary: (args: any) => {
      if (args?.entries) this.dictionary = { ...args.entries };
      return { success: true, entries: this.dictionary };
    },
    getDictionary: () => ({ entries: this.dictionary }),
    togglePostProcessing: (args: any) => {
      this.postProcessing = args?.enabled !== undefined ? Boolean(args.enabled) : !this.postProcessing;
      return { success: true, post_processing_enabled: this.postProcessing };
    },
    searchTranscripts: () => ({
      entries: [
        { id: "tr-1", text: "Open Cursor and snap window left", timestamp_ms: Date.now() - 120000, model: "parakeet-v3" },
        { id: "tr-2", text: "Summarize action items from the Q4 architecture review", timestamp_ms: Date.now() - 60000, model: "parakeet-v3" },
      ],
    }),
    clearTranscripts: () => ({ success: true, cleared: true }),
    toggleDebugDrawer: () => {
      this.debugDrawer = !this.debugDrawer;
      return { debug_drawer_visible: this.debugDrawer };
    },
    getDebugDrawerStatus: () => ({ debug_drawer_visible: this.debugDrawer }),

    // R2: Speech-to-Action
    executeVoiceCommand: (args: any) => ({
      success: true,
      command: args?.command ?? "",
      intent: "Executed",
      centered: true,
    }),
    snapWindow: (args: any) => ({ success: true, direction: args?.direction ?? "Center" }),
    resizeWindow: (args: any) => ({ success: true, resize: args }),
    cycleWindow: (args: any) => ({ success: true, direction: args?.direction ?? "next" }),
    navigateUi: (args: any) => ({ success: true, action: args?.action ?? "next_field" }),

    // R3: Meetily
    getMixerStatus: () => ({
      active: this.recordingMeeting,
      mic_gain: 1.0,
      system_gain: 0.25,
      ducking_engaged: true,
      ducking_floor: 0.25,
      limiter_active: true,
      peak_dbfs: -6.2,
    }),
    startMeetingRecording: (args: any) => {
      this.recordingMeeting = true;
      return {
        success: true,
        meeting_id: "mtg-live-01",
        title: args?.title ?? "Live Meeting Session",
        status: "Recording",
      };
    },
    stopMeetingRecording: () => {
      this.recordingMeeting = false;
      return {
        success: true,
        meeting_id: "mtg-live-01",
        status: "Stopped",
        segments_recorded: 4,
      };
    },
    importAndEnhanceAudio: (args: any) => ({
      success: true,
      file_path: args?.file_path ?? "meeting_recording.wav",
      model: args?.model ?? "parakeet-v3",
      language: args?.language ?? "en",
      transcript: "[00:00:02] Host: Welcome to the hands-free AirCtrl and Meetily architecture sync.\n[00:00:09] Participant: Virtual Camera Splitter is streaming to Zoom while MediaPipe tracks both hands at 60 FPS.",
    }),
    generateSummary: (args: any) => ({
      success: true,
      provider: args?.provider ?? "ollama",
      template: args?.template ?? "Executive Summary",
      summary: `## ${args?.template ?? "Executive Summary"} (${(args?.provider ?? "Ollama").toUpperCase()})\n- **Key Outcome**: Unified Handy offline STT, Meetily dual-audio assistant, Speech-to-Action window/focus control, and Google MediaPipe dual-hand tracking.\n- **Action Items**:\n  1. Keep "AirCtrl Virtual Camera" active for concurrent Zoom/Teams video + 60 FPS MediaPipe hand tracking.\n  2. Use voice commands ("hey airctrl snap left", "resize window 80%", "next window") for 100% hands-free navigation.`,
    }),
    exportSummary: (args: any) => ({
      success: true,
      format: args?.format ?? "markdown",
      output_path: `exports/meeting_minutes.${args?.format === "docx" ? "docx" : args?.format === "pdf" ? "pdf" : "md"}`,
      bytes_written: 2048,
    }),
    getMeetingArchive: () => ({
      meetings: [
        {
          id: "mtg-01",
          title: "Product Architecture & Hands-Free UX Sync",
          created_at: new Date().toISOString(),
          duration_secs: 1840,
          provider: "Ollama (llama3.2)",
          summary_preview: "Aligned on continuous AirCtrl Virtual Camera splitter and 1:1 Handy + Meetily parity.",
        },
      ],
    }),
    seekAudioOffset: (args: any) => ({
      success: true,
      meeting_id: args?.meeting_id ?? "mtg-01",
      byte_offset: 44 + Math.floor(((args?.timestamp_ms ?? 0) / 1000) * 16000 * 2),
    }),

    // R4: MediaPipe & Virtual Camera Splitter
    getGestureEngineStatus: () => ({
      running: true,
      mode: this.trackingMode,
      active_hands: 2,
      fps: 60.0,
      landmarks_per_hand: 21,
      pinch_engage_threshold: 0.05,
      pinch_release_threshold: 0.07,
    }),
    setGestureTrackingMode: (args: any) => {
      if (args?.mode === "SingleHand" || args?.mode === "DualHand") {
        this.trackingMode = args.mode;
      }
      return { success: true, mode: this.trackingMode };
    },
    getVirtualCameraStatus: () => ({
      device_name: "AirCtrl Virtual Camera",
      running: true,
      consumers: ["MediaPipeGestureEngine", "OutboundMeetingStream"],
      consumer_count: 2,
      privacy_muted: this.privacyMuted,
      mute_both: this.muteBoth,
      privacy_mode: !this.privacyMuted ? "ActiveDualStream" : this.muteBoth ? "DualMute" : "PrivacySlate",
      fps: 60,
      resolution: "1280x720",
    }),
    setVirtualCameraPrivacy: (args: any) => {
      this.privacyMuted = Boolean(args?.privacy_muted);
      this.muteBoth = Boolean(args?.mute_both);
      return {
        success: true,
        device_name: "AirCtrl Virtual Camera",
        privacy_muted: this.privacyMuted,
        mute_both: this.muteBoth,
        gestures_active: !this.muteBoth,
        outbound_meeting_video: this.privacyMuted ? "PrivacySlate" : "LiveCamera",
      };
    },
    getMultiHandGestureStatus: () => ({
      hands_detected: 2,
      left_hand: { present: true, gesture: "OpenPalm", pinch_distance: 0.09 },
      right_hand: { present: true, gesture: "PointingRay", pinch_distance: 0.04 },
    }),
  };

  public invoke<T = any>(command: string, args?: any): Promise<T> {
    if (!command || !(command in this.handlers)) {
      return Promise.reject(new Error(`Tauri command not found: ${command}`));
    }
    return Promise.resolve(this.handlers[command](args || {}));
  }
}

export const mockBridge = new MockTauriIpcBridge();
