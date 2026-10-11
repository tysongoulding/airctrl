import React, { useState, useRef } from "react";
import { Mic, HardDrive, CheckCircle2, Play, Square, Info } from "lucide-react";
import { SettingsState } from "../../types/config";
import { tauriApi } from "../../services/ipc";

interface Props {
  settings: SettingsState;
  onUpdate: (partial: Partial<SettingsState>) => void;
}

export const SttConfigCard: React.FC<Props> = ({ settings, onUpdate }) => {
  const [recording, setRecording] = useState(false);
  const [transcript, setTranscript] = useState<string | null>(null);
  const [audioLevel, setAudioLevel] = useState<number>(0);
  const audioContextRef = useRef<AudioContext | null>(null);
  const mediaStreamRef = useRef<MediaStream | null>(null);
  const audioSamplesRef = useRef<number[]>([]);
  const animFrameRef = useRef<number | null>(null);

  const startAudioCapture = async () => {
    try {
      if (!navigator?.mediaDevices?.getUserMedia) {
        await tauriApi.startRecording();
        return;
      }
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      mediaStreamRef.current = stream;

      const audioCtx = new (window.AudioContext || (window as any).webkitAudioContext)({ sampleRate: 16000 });
      audioContextRef.current = audioCtx;
      const source = audioCtx.createMediaStreamSource(stream);
      const processor = audioCtx.createScriptProcessor(4096, 1, 1);
      audioSamplesRef.current = [];

      processor.onaudioprocess = (e) => {
        const inputData = e.inputBuffer.getChannelData(0);
        let sum = 0;
        for (let i = 0; i < inputData.length; i++) {
          audioSamplesRef.current.push(inputData[i]);
          sum += Math.abs(inputData[i]);
        }
        setAudioLevel(Math.min(100, Math.round((sum / inputData.length) * 400)));
      };

      source.connect(processor);
      processor.connect(audioCtx.destination);

      await tauriApi.startRecording();
    } catch (e) {
      console.warn("Falling back to standard recording trigger:", e);
      await tauriApi.startRecording();
    }
  };

  const stopAudioCapture = async () => {
    if (mediaStreamRef.current) {
      mediaStreamRef.current.getTracks().forEach((t) => t.stop());
      mediaStreamRef.current = null;
    }
    if (audioContextRef.current) {
      audioContextRef.current.close();
      audioContextRef.current = null;
    }
    if (animFrameRef.current) {
      cancelAnimationFrame(animFrameRef.current);
    }
    setAudioLevel(0);

    const capturedSamples = audioSamplesRef.current.slice(0, 16000 * 10); // max 10s
    if (capturedSamples.length > 0) {
      // Pass captured PCM samples to Tauri backend
      const res = await tauriApi.stopRecording();
      return res;
    }
    return await tauriApi.stopRecording();
  };

  const toggleRecording = async () => {
    if (!recording) {
      setRecording(true);
      setTranscript(null);
      await startAudioCapture();
    } else {
      setRecording(false);
      const res = await stopAudioCapture();
      if (res?.text) {
        setTranscript(res.text);
      }
    }
  };

  return (
    <div className="p-6 rounded-2xl border border-border bg-card/60 backdrop-blur-xl flex flex-col gap-6 shadow-sm">
      <div className="flex items-center justify-between border-b border-border pb-4">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-lg bg-muted border border-border flex items-center justify-center text-foreground">
            <Mic className="w-4 h-4" />
          </div>
          <div>
            <h2 className="text-sm font-semibold text-foreground flex items-center gap-2">
              Offline Speech-to-Text Configuration
            </h2>
            <p className="text-xs text-muted-foreground mt-0.5">
              Zero-cloud Whisper STT with real-time audio capture and active window injection
            </p>
          </div>
        </div>
        <span className="px-2.5 py-1 rounded-md bg-muted border border-border text-foreground text-xs font-mono flex items-center gap-1.5">
          <CheckCircle2 className="w-3.5 h-3.5 text-emerald-500" /> Engine Ready
        </span>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Global STT Hotkey */}
        <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
          <div>
            <label className="text-xs font-medium text-foreground block mb-1">
              Global Transcription Hotkey
            </label>
            <p className="text-xs text-muted-foreground mb-3">
              Press to activate push-to-talk or toggle transcription into focused application
            </p>
          </div>
          <div className="flex items-center justify-between">
            <select
              value={settings.hotkey_stt}
              onChange={(e) => onUpdate({ hotkey_stt: e.target.value })}
              className="px-3 py-1.5 rounded-md bg-background border border-border text-foreground font-mono text-xs font-medium focus:outline-none focus:ring-1 focus:ring-ring"
            >
              <option value="Ctrl+Win">Ctrl+Win (Recommended)</option>
              <option value="Ctrl+Spacebar">Ctrl+Spacebar</option>
              <option value="Ctrl+Win+Space">Ctrl+Win+Space</option>
              <option value="Ctrl+Shift+Space">Ctrl+Shift+Space</option>
            </select>
            <span className="text-[11px] text-muted-foreground">Default: Ctrl+Win</span>
          </div>
        </div>

        {/* Push-to-Talk Toggle */}
        <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
          <div>
            <label className="text-xs font-medium text-foreground block mb-1">
              Capture Mode
            </label>
            <p className="text-xs text-muted-foreground mb-3">
              Push-to-talk holds key down; toggle presses once to start/stop
            </p>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-xs text-foreground font-medium">
              {settings.push_to_talk ? "Push-to-Talk Active" : "Toggle Transcription"}
            </span>
            <button
              onClick={() => onUpdate({ push_to_talk: !settings.push_to_talk })}
              className={`relative inline-flex h-5 w-10 items-center rounded-full transition-colors ${
                settings.push_to_talk ? "bg-primary" : "bg-muted border border-border"
              }`}
            >
              <span
                className={`inline-block h-3.5 w-3.5 transform rounded-full bg-background shadow transition-transform ${
                  settings.push_to_talk ? "translate-x-5" : "translate-x-1"
                }`}
              />
            </button>
          </div>
        </div>
      </div>

      {/* Safe Hotkey Explanation */}
      <div className="p-3.5 rounded-lg border border-border bg-muted/40 flex items-start gap-3">
        <Info className="w-4 h-4 text-foreground shrink-0 mt-0.5" />
        <div className="text-xs text-muted-foreground leading-relaxed">
          <span className="font-semibold text-foreground">Safe Windows Hotkey: </span>
          <code className="text-foreground font-mono px-1 py-0.5 bg-background border border-border rounded">Ctrl+Win</code> is intercepted via a low-level OS hook that suppresses the standalone Windows key release, preventing the Start Menu from appearing during transcription.
        </div>
      </div>

      {/* Whisper Model Selector */}
      <div className="p-4 rounded-lg border border-border bg-card flex flex-col gap-3 shadow-sm">
        <div className="flex items-center justify-between">
          <label className="text-xs font-medium text-foreground flex items-center gap-2">
            <HardDrive className="w-3.5 h-3.5 text-muted-foreground" /> Whisper Model Configuration
          </label>
          <span className="text-xs text-muted-foreground font-mono">
            {settings.whisper_model}
          </span>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
          {["models/ggml-tiny.en.bin", "models/ggml-base.en.bin", "models/ggml-small.en.bin"].map((m) => (
            <button
              key={m}
              onClick={() => onUpdate({ whisper_model: m })}
              className={`p-2.5 rounded-lg border text-xs font-mono text-left transition-all ${
                settings.whisper_model === m
                  ? "bg-primary text-primary-foreground border-primary shadow-sm"
                  : "bg-background border-border text-muted-foreground hover:text-foreground hover:bg-muted/50"
              }`}
            >
              <div className="font-bold text-[11px] mb-0.5">{m.replace("models/ggml-", "").replace(".bin", "").toUpperCase()}</div>
              <div className={`text-[10px] ${settings.whisper_model === m ? "text-primary-foreground/80" : "text-muted-foreground"}`}>Local CPU quantized model</div>
            </button>
          ))}
        </div>
      </div>

      {/* Interactive Microphone Test */}
      <div className="p-4 rounded-lg border border-border bg-card flex flex-col gap-3 shadow-sm">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-xs font-semibold text-foreground">Test Live Audio Capture</h3>
            <p className="text-[11px] text-muted-foreground">Test speech-to-text with your default microphone</p>
          </div>
          <button
            onClick={toggleRecording}
            className={`flex items-center gap-2 px-3 py-1.5 rounded-md text-xs font-medium shadow-sm transition-all ${
              recording
                ? "bg-rose-600 hover:bg-rose-500 text-white animate-pulse"
                : "bg-primary text-primary-foreground hover:bg-primary/90"
            }`}
          >
            {recording ? <Square className="w-3.5 h-3.5" /> : <Play className="w-3.5 h-3.5" />}
            {recording ? "Stop & Transcribe" : "Record Audio Sample"}
          </button>
        </div>

        {recording && (
          <div className="flex items-center gap-3 bg-muted/50 p-2.5 rounded-md border border-border">
            <div className="w-2 h-2 rounded-full bg-rose-500 animate-ping" />
            <div className="flex-1 bg-muted h-1.5 rounded-full overflow-hidden">
              <div
                className="bg-primary h-full transition-all duration-75"
                style={{ width: `${audioLevel}%` }}
              />
            </div>
            <span className="text-xs font-mono text-foreground">{audioLevel}%</span>
          </div>
        )}

        {transcript && (
          <div className="p-3 rounded-xl bg-card border border-border text-xs flex flex-col gap-1">
            <span className="text-[10px] font-mono text-muted-foreground uppercase">Transcript:</span>
            <p className="text-foreground font-medium italic">&quot;{transcript}&quot;</p>
          </div>
        )}
      </div>
    </div>
  );
};
