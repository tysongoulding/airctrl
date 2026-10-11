export interface AudioEnergyMetrics {
  rms: number; // 0.0 to 1.0
  dbfs: number; // -60.0 to 0.0 dB
  vad_speech_detected: boolean;
  stt_state: "Idle" | "Listening" | "Transcribing" | "Injecting" | "Error";
}

export interface GestureMetrics {
  fps: number; // e.g. 30 to 60 FPS
  hand_detected: boolean;
  handedness: "Left" | "Right" | "None";
  recognized_gesture: "Pointing" | "Pinch" | "OpenPalm" | "SwipeLeft" | "SwipeRight" | "Neutral";
  active_sector: number | null; // 0..5 or null if in deadzone
  normalized_radius: number;
}

export interface MeetingMetrics {
  status: "Inactive" | "Detected" | "Active" | "Ended";
  platform: "Zoom" | "Teams" | "Meet" | "Webex" | "Discord" | null;
  pid: number | null;
  camera_state: "Active" | "KilledByMeeting" | "PausedByUser";
  compliance_audio_injected: boolean;
}

export interface LanMetrics {
  server_ip: string;
  port: number;
  is_online: boolean;
  binary_name: string;
  binary_sha256: string | null;
  artifact_available: boolean;
  uptime_secs: number;
}
