import { mockBridge } from "./mockBridge";

// Detect if running inside real Tauri desktop webview
const isTauri = typeof window !== "undefined" && ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);

function toSnakeCase(str: string): string {
  return str.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`);
}

export async function invokeCommand<T = any>(cmd: string, args?: Record<string, any>): Promise<T> {
  if (isTauri) {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const snakeCmd = toSnakeCase(cmd);
      try {
        return await invoke<T>(snakeCmd, args);
      } catch {
        return await invoke<T>(cmd, args);
      }
    } catch (err) {
      console.warn(`[IPC Fallback] Failed to invoke '${cmd}' natively, routing to mock bridge:`, err);
      return mockBridge.invoke<T>(cmd, args);
    }
  }
  // Browser preview or headless CI mode
  return mockBridge.invoke<T>(cmd, args);
}

export const tauriApi = {
  // Core & Recording
  startRecording: (args?: any) => invokeCommand<{ status: string }>("startRecording", args),
  stopRecording: (args?: any) => invokeCommand<{ status: string; text: string }>("stopRecording", args),
  triggerAction: (action: string, params?: any) =>
    invokeCommand<{ success: boolean; action: any }>("triggerAction", { action, ...params }),
  toggleRadialDial: (args?: any) => invokeCommand<{ dial_visible: boolean }>("toggleRadialDial", args),
  toggleSttOverlay: (args?: any) => invokeCommand<{ stt_overlay_visible: boolean }>("toggleSttOverlay", args),
  toggleCommandPalette: (args?: any) => invokeCommand<{ command_palette_visible: boolean }>("toggleCommandPalette", args),
  getSettings: () => invokeCommand("getSettings"),
  updateSettings: (settings: any) => invokeCommand("updateSettings", settings),
  getMeetingStatus: () => invokeCommand("getMeetingStatus"),
  triggerCameraKillSwitch: () =>
    invokeCommand<{ success: boolean; status: string; camera_handle_closed: boolean }>("triggerCameraKillSwitch"),
  getCatalogedApps: () => invokeCommand<{ apps: import("../types/config").ApplicationItem[] }>("getCatalogedApps"),
  refreshCatalogedApps: () => invokeCommand<{ apps: import("../types/config").ApplicationItem[] }>("refreshCatalogedApps"),
  launchApp: (path: string) => invokeCommand<{ success: boolean; pid: number }>("launchApp", { path }),
  injectText: (text: string) => invokeCommand<{ success: boolean; text_length: number }>("injectText", { text }),

  // R1: Handy Parity Commands
  listModels: () => invokeCommand("listModels"),
  selectModel: (model_id: string) => invokeCommand("selectModel", { model_id }),
  setRecordingMode: (mode: string) => invokeCommand("setRecordingMode", { mode }),
  getRecordingMode: () => invokeCommand("getRecordingMode"),
  setOverlayPosition: (position: string) => invokeCommand("setOverlayPosition", { position }),
  getOverlayPosition: () => invokeCommand("getOverlayPosition"),
  updateDictionary: (entries: Record<string, string>) => invokeCommand("updateDictionary", { entries }),
  getDictionary: () => invokeCommand("getDictionary"),
  togglePostProcessing: (enabled?: boolean) => invokeCommand("togglePostProcessing", enabled !== undefined ? { enabled } : {}),
  searchTranscripts: (query: string) => invokeCommand("searchTranscripts", { query }),
  clearTranscripts: () => invokeCommand("clearTranscripts"),
  toggleDebugDrawer: () => invokeCommand("toggleDebugDrawer"),
  getDebugDrawerStatus: () => invokeCommand("getDebugDrawerStatus"),

  // R2: Speech-to-Action 100% Hands-Free Commands
  executeVoiceCommand: (command: string) => invokeCommand("executeVoiceCommand", { command }),
  snapWindow: (direction: string) => invokeCommand("snapWindow", { direction }),
  resizeWindow: (args: { mode: string; value?: string; width?: number; height?: number; percent?: number }) =>
    invokeCommand("resizeWindow", args),
  cycleWindow: (direction: "next" | "previous") => invokeCommand("cycleWindow", { direction }),
  navigateUi: (action: string, direction?: string, amount?: number) =>
    invokeCommand("navigateUi", { action, direction, amount }),

  // R3: Meetily 1:1 Parity Commands
  getMixerStatus: () => invokeCommand("getMixerStatus"),
  startMeetingRecording: (title?: string, platform?: string) =>
    invokeCommand("startMeetingRecording", { title, platform }),
  stopMeetingRecording: () => invokeCommand("stopMeetingRecording"),
  importAndEnhanceAudio: (file_path: string, model?: string, language?: string) =>
    invokeCommand("importAndEnhanceAudio", { file_path, model, language }),
  generateSummary: (args: { provider: string; model?: string; template?: string; endpoint?: string; transcript?: string }) =>
    invokeCommand("generateSummary", args),
  exportSummary: (args: { format: "markdown" | "pdf" | "docx"; meeting_id?: string; title?: string; content?: string }) =>
    invokeCommand("exportSummary", args),
  getMeetingArchive: (query?: string) => invokeCommand("getMeetingArchive", query ? { query } : {}),
  seekAudioOffset: (meeting_id: string, timestamp_ms: number) =>
    invokeCommand("seekAudioOffset", { meeting_id, timestamp_ms }),

  // R4: Google MediaPipe & Persistent Virtual Camera Splitter Commands
  getGestureEngineStatus: () => invokeCommand("getGestureEngineStatus"),
  setGestureTrackingMode: (mode: "SingleHand" | "DualHand", sensitivity?: number, cooldown_ms?: number) =>
    invokeCommand("setGestureTrackingMode", { mode, sensitivity, cooldown_ms }),
  getVirtualCameraStatus: () => invokeCommand("getVirtualCameraStatus"),
  setVirtualCameraPrivacy: (privacy_muted: boolean, mute_both?: boolean) =>
    invokeCommand("setVirtualCameraPrivacy", { privacy_muted, mute_both: mute_both ?? false }),
  getMultiHandGestureStatus: () => invokeCommand("getMultiHandGestureStatus"),
};
