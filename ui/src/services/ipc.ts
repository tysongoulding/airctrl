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
  startRecording: (args?: any) => invokeCommand<{ status: string }>("startRecording", args),
  stopRecording: (args?: any) => invokeCommand<{ status: string; text: string }>("stopRecording", args),
  triggerAction: (action: string, params?: any) =>
    invokeCommand<{ success: boolean; action: any }>("triggerAction", { action, ...params }),
  toggleRadialDial: (args?: any) => invokeCommand<{ dial_visible: boolean }>("toggleRadialDial", args),
  toggleSttOverlay: (args?: any) => invokeCommand<{ stt_overlay_visible: boolean }>("toggleSttOverlay", args),
  getSettings: () => invokeCommand("getSettings"),
  updateSettings: (settings: any) => invokeCommand("updateSettings", settings),
  getMeetingStatus: () => invokeCommand("getMeetingStatus"),
  triggerCameraKillSwitch: () =>
    invokeCommand<{ success: boolean; status: string; camera_handle_closed: boolean }>("triggerCameraKillSwitch"),
  getCatalogedApps: () => invokeCommand<{ apps: import("../types/config").ApplicationItem[] }>("getCatalogedApps"),
  refreshCatalogedApps: () => invokeCommand<{ apps: import("../types/config").ApplicationItem[] }>("refreshCatalogedApps"),
  launchApp: (path: string) => invokeCommand<{ success: boolean; pid: number }>("launchApp", { path }),
};
