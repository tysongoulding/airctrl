/**
 * Mock Tauri IPC Bridge implementing all commands for headless CI and browser preview.
 * Strictly complies with tests/e2e/tier1_category_partition/test_t1_f24_tauri_ipc_bridge.py.
 */
export class MockTauriIpcBridge {
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
      camera_auto_kill: true,
      compliance_audio_enabled: true,
      radial_dial_sectors: 6,
      lan_port: 8080,
      whisper_model: "models/ggml-tiny.en.bin",
      theme: "dark",
      hud_opacity: 0.95,
    }),
    updateSettings: (args: any) => ({
      success: true,
      settings: {
        hotkey_stt: args?.hotkey_stt ?? "Ctrl+Spacebar",
        hotkey_dial: args?.hotkey_dial ?? "Ctrl+Alt",
        camera_auto_kill: args?.camera_auto_kill ?? true,
        compliance_audio_enabled: args?.compliance_audio_enabled ?? true,
        radial_dial_sectors: args?.radial_dial_sectors ?? 6,
      },
    }),
    getMeetingStatus: () => ({
      status: "Inactive",
      meeting: null,
      camera_state: "Active",
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
  };

  public invoke<T = any>(command: string, args?: any): Promise<T> {
    if (!command || !(command in this.handlers)) {
      return Promise.reject(new Error(`Tauri command not found: ${command}`));
    }
    return Promise.resolve(this.handlers[command](args || {}));
  }
}

export const mockBridge = new MockTauriIpcBridge();
