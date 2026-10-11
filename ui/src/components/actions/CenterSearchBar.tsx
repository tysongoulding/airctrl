import React, { useState, useEffect, useRef, useCallback } from "react";
import {
  Search,
  Mic,
  Square,
  X,
  Check,
  Maximize2,
  Minimize2,
  FolderOpen,
  Terminal,
  Cpu,
  Layout,
  Globe,
  Loader2,
  Sparkles,
  Command,
} from "lucide-react";
import { tauriApi } from "../../services/ipc";
import { ApplicationItem } from "../../types/config";

export type ActionParam = "open" | "close" | "center";

interface Props {
  onClose?: () => void;
  standalone?: boolean;
}

export const CenterSearchBar: React.FC<Props> = ({ onClose, standalone = true }) => {
  const [query, setQuery] = useState("");
  const [selectedParam, setSelectedParam] = useState<ActionParam>("open");
  const [catalogApps, setCatalogApps] = useState<ApplicationItem[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [isRecording, setIsRecording] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [waveHeights, setWaveHeights] = useState<number[]>(() => Array(16).fill(4));
  const [statusToast, setStatusToast] = useState<{ success: boolean; message: string } | null>(null);

  const inputRef = useRef<HTMLInputElement>(null);
  const audioCtxRef = useRef<AudioContext | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const mediaStreamRef = useRef<MediaStream | null>(null);
  const animFrameRef = useRef<number | null>(null);

  // Fetch cataloged apps on mount
  useEffect(() => {
    let isMounted = true;
    tauriApi.getCatalogedApps().then((res) => {
      if (isMounted && res?.apps) {
        setCatalogApps(res.apps);
      }
    }).catch((err) => console.warn("Failed to load cataloged apps:", err));

    if (inputRef.current) {
      inputRef.current.focus();
    }
    return () => {
      isMounted = false;
    };
  }, []);

  // Parse parameter prefix if user types "open slack", "close chrome", "center terminal"
  const parsePrefix = (text: string): { param: ActionParam; cleanQuery: string } => {
    const trimmed = text.trim();
    const lower = trimmed.toLowerCase();
    if (lower.startsWith("open ") || lower.startsWith("launch ") || lower.startsWith("run ")) {
      return { param: "open", cleanQuery: trimmed.replace(/^(open|launch|run)\s+/i, "") };
    }
    if (lower.startsWith("close ") || lower.startsWith("quit ") || lower.startsWith("exit ") || lower.startsWith("kill ")) {
      return { param: "close", cleanQuery: trimmed.replace(/^(close|quit|exit|kill)\s+/i, "") };
    }
    if (lower.startsWith("center ") || lower.startsWith("centre ")) {
      return { param: "center", cleanQuery: trimmed.replace(/^(center|centre)\s+/i, "") };
    }
    return { param: selectedParam, cleanQuery: trimmed };
  };

  const { param: activeParam, cleanQuery } = parsePrefix(query);

  // Filter application list
  const filteredApps = catalogApps.filter((app) => {
    if (!cleanQuery) return true;
    const q = cleanQuery.toLowerCase();
    return (
      app.name.toLowerCase().includes(q) ||
      app.path.toLowerCase().includes(q) ||
      (app.category && app.category.toLowerCase().includes(q))
    );
  });

  // Keep selected index within bounds
  useEffect(() => {
    if (selectedIndex >= filteredApps.length) {
      setSelectedIndex(Math.max(0, filteredApps.length - 1));
    }
  }, [filteredApps.length, selectedIndex]);

  // Audio Equalizer for Voice Command Dictation
  const updateWaveform = useCallback(() => {
    if (isRecording && analyserRef.current) {
      const dataArray = new Uint8Array(analyserRef.current.frequencyBinCount);
      analyserRef.current.getByteFrequencyData(dataArray);
      const count = 16;
      const step = Math.floor(dataArray.length / count);
      const heights: number[] = [];
      for (let i = 0; i < count; i++) {
        let sum = 0;
        for (let j = 0; j < step; j++) {
          sum += dataArray[i * step + j] || 0;
        }
        const avg = sum / step;
        const h = Math.max(4, Math.min(22, Math.round((avg / 255) * 22)));
        heights.push(h);
      }
      setWaveHeights(heights);
    } else if (isTranscribing) {
      const now = performance.now() * 0.008;
      const heights = Array.from({ length: 16 }, (_, i) => {
        const val = Math.sin(now + i * 0.4);
        return Math.max(4, Math.round(((val + 1) * 0.5) * 18) + 4);
      });
      setWaveHeights(heights);
    } else {
      setWaveHeights(Array(16).fill(4));
    }
    animFrameRef.current = requestAnimationFrame(updateWaveform);
  }, [isRecording, isTranscribing]);

  useEffect(() => {
    animFrameRef.current = requestAnimationFrame(updateWaveform);
    return () => {
      if (animFrameRef.current) cancelAnimationFrame(animFrameRef.current);
    };
  }, [updateWaveform]);

  // Execute parameter-first command
  const executeCommand = async (param: ActionParam, targetApp?: ApplicationItem | string) => {
    const appName = typeof targetApp === "string" ? targetApp : targetApp?.name || cleanQuery || "current window";
    const fullCommand = `${param} ${appName}`.trim();

    try {
      const res = await tauriApi.triggerAction(fullCommand);
      let feedback = "";
      if (param === "open") {
        feedback = `Opened & centered ${appName}`;
      } else if (param === "close") {
        feedback = `Closed ${appName}`;
      } else if (param === "center") {
        feedback = `Centered ${appName}`;
      }

      setStatusToast({
        success: res?.success ?? true,
        message: feedback,
      });

      setTimeout(() => {
        if (standalone && onClose) {
          onClose();
        } else {
          setStatusToast(null);
        }
      }, 1500);
    } catch (err: any) {
      setStatusToast({
        success: false,
        message: `Command failed: ${err?.message || err}`,
      });
      setTimeout(() => setStatusToast(null), 4000);
    }
  };

  // Voice recording toggle
  const toggleRecording = async () => {
    if (isRecording) {
      // Stop recording and transcribe
      setIsRecording(false);
      setIsTranscribing(true);

      if (mediaStreamRef.current) {
        mediaStreamRef.current.getTracks().forEach((t) => t.stop());
        mediaStreamRef.current = null;
      }
      if (audioCtxRef.current) {
        audioCtxRef.current.close();
        audioCtxRef.current = null;
      }

      try {
        const res = await tauriApi.stopRecording();
        setIsTranscribing(false);
        if (res && res.text) {
          const spoken = res.text.trim();
          setQuery(spoken);
          const parsed = parsePrefix(spoken);
          await executeCommand(parsed.param, parsed.cleanQuery);
        }
      } catch (err) {
        setIsTranscribing(false);
        console.warn("STT stop notice:", err);
      }
    } else {
      // Start recording
      setIsRecording(true);
      setStatusToast(null);

      try {
        if (navigator?.mediaDevices?.getUserMedia) {
          const stream = await navigator.mediaDevices.getUserMedia({
            audio: { channelCount: 1, sampleRate: 16000 },
          });
          mediaStreamRef.current = stream;

          const audioCtx = new (window.AudioContext || (window as any).webkitAudioContext)({
            sampleRate: 16000,
          });
          audioCtxRef.current = audioCtx;

          const source = audioCtx.createMediaStreamSource(stream);
          const analyser = audioCtx.createAnalyser();
          analyser.fftSize = 64;
          source.connect(analyser);
          analyserRef.current = analyser;
        }
        await tauriApi.startRecording();
      } catch (err) {
        console.warn("Audio recording warning:", err);
      }
    }
  };

  // Keyboard navigation
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      if (onClose) onClose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((prev) => Math.min(filteredApps.length - 1, prev + 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((prev) => Math.max(0, prev - 1));
    } else if (e.key === "Tab") {
      e.preventDefault();
      const params: ActionParam[] = ["open", "close", "center"];
      const nextIndex = (params.indexOf(selectedParam) + 1) % params.length;
      setSelectedParam(params[nextIndex]);
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (filteredApps.length > 0 && selectedIndex < filteredApps.length) {
        executeCommand(activeParam, filteredApps[selectedIndex]);
      } else if (cleanQuery) {
        executeCommand(activeParam, cleanQuery);
      }
    }
  };

  const getAppIcon = (iconName?: string) => {
    switch (iconName) {
      case "terminal":
        return <Terminal className="w-4 h-4 text-[#d87943]" />;
      case "cpu":
        return <Cpu className="w-4 h-4 text-emerald-400" />;
      case "layout":
        return <Layout className="w-4 h-4 text-sky-400" />;
      case "window":
        return <Globe className="w-4 h-4 text-amber-400" />;
      default:
        return <FolderOpen className="w-4 h-4 text-muted-foreground" />;
    }
  };

  return (
    <div
      className={`select-none font-sans transition-all duration-300 ${
        standalone
          ? "fixed inset-0 flex items-center justify-center pointer-events-none p-4 z-50"
          : "w-full"
      }`}
    >
      <div
        className={`pointer-events-auto rounded-2xl bg-[#121113]/95 border border-[#d87943]/40 shadow-2xl shadow-[#d87943]/15 backdrop-blur-2xl flex flex-col overflow-hidden transition-all duration-300 ${
          standalone ? "w-[640px] max-w-[94vw]" : "w-full"
        }`}
      >
        {/* Top Search Bar & Voice Input */}
        <div className="flex items-center gap-3 p-3.5 border-b border-border/60 bg-[#161518]/90">
          <Search className="w-5 h-5 text-muted-foreground shrink-0 ml-1" />

          {/* Active Parameter Chip Badge */}
          <div className="flex items-center gap-1.5 shrink-0">
            <span
              className={`px-2 py-0.5 rounded text-[11px] font-mono font-semibold uppercase tracking-wider transition-colors ${
                activeParam === "open"
                  ? "bg-[#d87943]/20 text-[#d87943] border border-[#d87943]/40"
                  : activeParam === "close"
                  ? "bg-rose-500/20 text-rose-400 border border-rose-500/40"
                  : "bg-[#527575]/25 text-[#527575] border border-[#527575]/50"
              }`}
            >
              {activeParam === "open" ? "Open & Center" : activeParam}
            </span>
          </div>

          {/* Main Search Input */}
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Type or speak (e.g. open slack, close chrome, center code)..."
            className="flex-1 bg-transparent border-none text-foreground text-sm font-sans placeholder:text-muted-foreground/60 focus:outline-none focus:ring-0 min-w-0"
          />

          {/* Real-time Voice Audio Visualizer */}
          {(isRecording || isTranscribing) && (
            <div className="flex items-center gap-0.5 px-2 py-1 bg-[#18171a] rounded-lg border border-border shrink-0 h-7">
              {waveHeights.map((h, i) => (
                <span
                  key={i}
                  style={{ height: `${h}px` }}
                  className={`w-0.5 rounded-full transition-all duration-75 ${
                    isRecording ? (i % 2 === 0 ? "bg-[#d87943]" : "bg-[#527575]") : "bg-[#d87943] opacity-80"
                  }`}
                />
              ))}
            </div>
          )}

          {/* Microphone Voice Button */}
          <button
            onClick={toggleRecording}
            className={`p-2 rounded-xl flex items-center justify-center transition-all shrink-0 ${
              isRecording
                ? "bg-[#d87943] text-white animate-pulse shadow-md shadow-[#d87943]/50"
                : isTranscribing
                ? "bg-[#527575] text-white"
                : "bg-[#201e22] text-[#d87943] hover:bg-[#d87943] hover:text-white border border-[#d87943]/30"
            }`}
            title={isRecording ? "Stop listening" : "Speak voice command"}
          >
            {isRecording ? (
              <Square className="w-4 h-4 fill-current" />
            ) : isTranscribing ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <Mic className="w-4 h-4" />
            )}
          </button>

          {/* Close Button */}
          {onClose && (
            <button
              onClick={onClose}
              className="p-1.5 rounded-lg text-muted-foreground hover:text-foreground hover:bg-muted/60 transition-all shrink-0"
              title="Close (Esc)"
            >
              <X className="w-4 h-4" />
            </button>
          )}
        </div>

        {/* Action Parameter Selector Chips */}
        <div className="flex items-center justify-between px-4 py-2 border-b border-border/40 bg-[#141315] text-xs">
          <div className="flex items-center gap-1.5">
            <span className="text-[11px] text-muted-foreground mr-1">Parameter:</span>
            <button
              onClick={() => setSelectedParam("open")}
              className={`px-2.5 py-1 rounded-lg font-mono text-[11px] transition-all flex items-center gap-1.5 ${
                activeParam === "open"
                  ? "bg-[#d87943] text-white font-semibold shadow-sm"
                  : "bg-[#1f1d21] text-muted-foreground hover:text-foreground border border-border/50"
              }`}
            >
              <Maximize2 className="w-3 h-3" /> [open] Open & Center
            </button>
            <button
              onClick={() => setSelectedParam("close")}
              className={`px-2.5 py-1 rounded-lg font-mono text-[11px] transition-all flex items-center gap-1.5 ${
                activeParam === "close"
                  ? "bg-rose-600 text-white font-semibold shadow-sm"
                  : "bg-[#1f1d21] text-muted-foreground hover:text-foreground border border-border/50"
              }`}
            >
              <Minimize2 className="w-3 h-3" /> [close] Close App
            </button>
            <button
              onClick={() => setSelectedParam("center")}
              className={`px-2.5 py-1 rounded-lg font-mono text-[11px] transition-all flex items-center gap-1.5 ${
                activeParam === "center"
                  ? "bg-[#527575] text-white font-semibold shadow-sm"
                  : "bg-[#1f1d21] text-muted-foreground hover:text-foreground border border-border/50"
              }`}
            >
              <Layout className="w-3 h-3" /> [center] Center Window
            </button>
          </div>

          <span className="text-[10px] text-muted-foreground font-mono">
            Tab to cycle • Enter to run
          </span>
        </div>

        {/* Status Toast Banner */}
        {statusToast && (
          <div
            className={`px-4 py-2 text-xs font-mono flex items-center justify-between transition-all ${
              statusToast.success
                ? "bg-emerald-500/15 border-b border-emerald-500/30 text-emerald-400"
                : "bg-rose-500/15 border-b border-rose-500/30 text-rose-400"
            }`}
          >
            <span className="flex items-center gap-1.5">
              {statusToast.success ? <Check className="w-3.5 h-3.5" /> : <X className="w-3.5 h-3.5" />}
              {statusToast.message}
            </span>
            <span className="text-[10px] opacity-75">OS Hook Executed</span>
          </div>
        )}

        {/* Filtered Application Results List */}
        <div className="max-h-[260px] overflow-y-auto p-2 flex flex-col gap-1">
          {filteredApps.length === 0 ? (
            <div className="py-8 text-center text-xs text-muted-foreground flex flex-col items-center gap-2">
              <Command className="w-6 h-6 opacity-30 text-[#d87943]" />
              <span>No applications matched &quot;{cleanQuery}&quot;</span>
              <span className="text-[10px] font-mono">
                Press Enter to run generic command: &quot;{activeParam} {cleanQuery}&quot;
              </span>
            </div>
          ) : (
            filteredApps.map((app, idx) => {
              const isSelected = idx === selectedIndex;
              return (
                <div
                  key={app.id || idx}
                  onClick={() => {
                    setSelectedIndex(idx);
                    executeCommand(activeParam, app);
                  }}
                  className={`flex items-center justify-between px-3 py-2 rounded-xl cursor-pointer transition-all ${
                    isSelected
                      ? "bg-[#252229] border border-[#d87943]/50 text-foreground shadow-sm"
                      : "hover:bg-[#1a181c] text-foreground/80 border border-transparent"
                  }`}
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <div className="w-8 h-8 rounded-lg bg-[#18171a] border border-border/60 flex items-center justify-center shrink-0">
                      {getAppIcon(app.icon)}
                    </div>
                    <div className="flex flex-col min-w-0">
                      <span className="text-xs font-semibold text-foreground truncate flex items-center gap-2">
                        {app.name}
                        {app.running && (
                          <span className="px-1.5 py-0.2 rounded-full bg-emerald-500/10 text-emerald-400 text-[9px] font-mono border border-emerald-500/20">
                            Running
                          </span>
                        )}
                      </span>
                      <span className="text-[10px] text-muted-foreground truncate font-mono">
                        {app.category || app.path}
                      </span>
                    </div>
                  </div>

                  {/* Contextual Action Button */}
                  <div className="flex items-center gap-1.5 shrink-0">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        executeCommand("open", app);
                      }}
                      className="px-2 py-1 rounded-md bg-[#18171a] hover:bg-[#d87943] hover:text-white border border-border text-[10px] font-mono text-muted-foreground transition-all"
                      title="Open & Center window"
                    >
                      Open
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        executeCommand("center", app);
                      }}
                      className="px-2 py-1 rounded-md bg-[#18171a] hover:bg-[#527575] hover:text-white border border-border text-[10px] font-mono text-muted-foreground transition-all"
                      title="Center window"
                    >
                      Center
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        executeCommand("close", app);
                      }}
                      className="px-2 py-1 rounded-md bg-[#18171a] hover:bg-rose-600 hover:text-white border border-border text-[10px] font-mono text-muted-foreground transition-all"
                      title="Close application"
                    >
                      Close
                    </button>
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* Footer Shortcut Bar */}
        <div className="px-4 py-2 border-t border-border/40 bg-[#121113] flex items-center justify-between text-[11px] text-muted-foreground font-mono">
          <div className="flex items-center gap-3">
            <span className="flex items-center gap-1">
              <kbd className="px-1.5 py-0.5 rounded bg-[#1f1d21] border border-border text-[9px]">↑↓</kbd> Navigate
            </span>
            <span className="flex items-center gap-1">
              <kbd className="px-1.5 py-0.5 rounded bg-[#1f1d21] border border-border text-[9px]">↵</kbd> Run
            </span>
            <span className="flex items-center gap-1">
              <kbd className="px-1.5 py-0.5 rounded bg-[#1f1d21] border border-border text-[9px]">Tab</kbd> Action
            </span>
          </div>
          <span className="flex items-center gap-1 text-[#d87943]">
            <Sparkles className="w-3 h-3" /> Qrafthive Voice Command Palette
          </span>
        </div>
      </div>
    </div>
  );
};
