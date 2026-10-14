import React, { useState, useEffect, useRef, useCallback } from "react";
import { Mic, Square, Check, Copy, X, Loader2, Volume2, VolumeX } from "lucide-react";
import { tauriApi } from "../../services/ipc";
import { audioFeedback } from "../../utils/audioFeedback";

export type SttOverlayState = "idle" | "recording" | "transcribing" | "done" | "error";

interface Props {
  onClose?: () => void;
  standalone?: boolean;
  position?: "bottom" | "top" | "center" | "none";
  soundEnabled?: boolean;
}

export const SttOverlay: React.FC<Props> = ({
  onClose,
  standalone = true,
  position = "bottom",
  soundEnabled = true,
}) => {
  const [state, setState] = useState<SttOverlayState>("idle");
  const [transcript, setTranscript] = useState<string>("");
  const [copied, setCopied] = useState<boolean>(false);
  const [elapsedSecs, setElapsedSecs] = useState<number>(0);
  const [dbfsLevel, setDbfsLevel] = useState<number>(-45);
  const [waveHeights, setWaveHeights] = useState<number[]>(() => Array(16).fill(4));
  const [soundOn, setSoundOn] = useState<boolean>(soundEnabled);

  const audioCtxRef = useRef<AudioContext | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const mediaStreamRef = useRef<MediaStream | null>(null);
  const samplesRef = useRef<number[]>([]);
  const animFrameRef = useRef<number | null>(null);
  const timerRef = useRef<any>(null);
  const autoHideTimerRef = useRef<any>(null);
  const keyDownTimeRef = useRef<number>(0);

  // Sync sound setting
  useEffect(() => {
    audioFeedback.setEnabled(soundOn);
  }, [soundOn]);

  // Audio Waveform Visualization Loop
  const updateWaveform = useCallback(() => {
    if (state === "recording" && analyserRef.current) {
      const dataArray = new Uint8Array(analyserRef.current.frequencyBinCount);
      analyserRef.current.getByteFrequencyData(dataArray);

      // Extract 16 frequency bands across vocal spectrum
      const barCount = 16;
      const step = Math.floor(dataArray.length / barCount);
      const newHeights: number[] = [];
      let totalEnergy = 0;

      for (let i = 0; i < barCount; i++) {
        let sum = 0;
        for (let j = 0; j < step; j++) {
          sum += dataArray[i * step + j] || 0;
        }
        const avg = sum / step;
        totalEnergy += avg;
        // Scale to 4px min and 24px max height
        const h = Math.max(4, Math.min(24, Math.round((avg / 255) * 24)));
        newHeights.push(h);
      }

      setWaveHeights(newHeights);

      const normEnergy = totalEnergy / (barCount * 255);
      const approxDbfs = normEnergy > 0.001 ? Math.round(20 * Math.log10(normEnergy)) : -60;
      setDbfsLevel(Math.max(-60, Math.min(0, approxDbfs)));
    } else if (state === "transcribing") {
      // Shimmer traveling wave animation
      const now = performance.now() * 0.008;
      const animated = Array.from({ length: 16 }, (_, i) => {
        const val = Math.sin(now + i * 0.4);
        return Math.max(4, Math.round(((val + 1) * 0.5) * 18) + 4);
      });
      setWaveHeights(animated);
    } else {
      setWaveHeights(Array(16).fill(4));
      setDbfsLevel(-60);
    }

    animFrameRef.current = requestAnimationFrame(updateWaveform);
  }, [state]);

  useEffect(() => {
    animFrameRef.current = requestAnimationFrame(updateWaveform);
    return () => {
      if (animFrameRef.current) cancelAnimationFrame(animFrameRef.current);
    };
  }, [updateWaveform]);

  // Recording Timer
  useEffect(() => {
    if (state === "recording") {
      setElapsedSecs(0);
      timerRef.current = setInterval(() => {
        setElapsedSecs((prev) => prev + 1);
      }, 1000);
    } else {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    }
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [state]);

  const recognitionRef = useRef<any>(null);
  const liveTranscriptRef = useRef<string>("");

  // Start Audio Stream, Live Speech Recognition & IPC Recording
  const startRecording = async () => {
    try {
      if (autoHideTimerRef.current) {
        clearTimeout(autoHideTimerRef.current);
        autoHideTimerRef.current = null;
      }

      setState("recording");
      setTranscript("");
      liveTranscriptRef.current = "";
      setCopied(false);

      // Play rising start sound cue (Handy acoustic feedback)
      audioFeedback.playStartCue();

      // Start native live speech recognition (WebView2 / Chromium SpeechRecognition)
      const SpeechRec = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
      if (SpeechRec) {
        try {
          const rec = new SpeechRec();
          rec.continuous = true;
          rec.interimResults = true;
          rec.lang = "en-US";
          rec.onresult = (event: any) => {
            let fullText = "";
            for (let i = 0; i < event.results.length; i++) {
              fullText += event.results[i][0].transcript;
            }
            const cleaned = fullText.trim();
            if (cleaned) {
              liveTranscriptRef.current = cleaned;
              setTranscript(cleaned);
            }
          };
          rec.onerror = () => {};
          rec.start();
          recognitionRef.current = rec;
        } catch {
          // ignore if unavailable
        }
      }

      if (navigator?.mediaDevices?.getUserMedia) {
        const stream = await navigator.mediaDevices.getUserMedia({
          audio: {
            channelCount: 1,
            sampleRate: 16000,
            echoCancellation: true,
            noiseSuppression: true,
          },
        });
        mediaStreamRef.current = stream;

        const AudioCtx = window.AudioContext || (window as any).webkitAudioContext;
        const audioCtx = new AudioCtx({ sampleRate: 16000 });
        audioCtxRef.current = audioCtx;

        const source = audioCtx.createMediaStreamSource(stream);
        const analyser = audioCtx.createAnalyser();
        analyser.fftSize = 64;
        source.connect(analyser);
        analyserRef.current = analyser;

        const processor = audioCtx.createScriptProcessor(4096, 1, 1);
        samplesRef.current = [];
        processor.onaudioprocess = (e) => {
          const input = e.inputBuffer.getChannelData(0);
          for (let i = 0; i < input.length; i++) {
            samplesRef.current.push(input[i]);
          }
        };
        source.connect(processor);
        processor.connect(audioCtx.destination);
      }

      await tauriApi.startRecording();
    } catch (err: any) {
      console.warn("Audio capture initialization notice:", err);
      await tauriApi.startRecording();
    }
  };

  // Stop Recording, Transcribe, and Auto-Paste / Execute Voice Action
  const stopRecording = async () => {
    setState("transcribing");

    if (recognitionRef.current) {
      try {
        recognitionRef.current.stop();
      } catch {
        // ignore
      }
      recognitionRef.current = null;
    }

    if (mediaStreamRef.current) {
      mediaStreamRef.current.getTracks().forEach((t) => t.stop());
      mediaStreamRef.current = null;
    }
    if (audioCtxRef.current) {
      audioCtxRef.current.close().catch(() => {});
      audioCtxRef.current = null;
    }

    try {
      // Wait briefly for final speech recognition event if any
      await new Promise((r) => setTimeout(r, 120));

      const captured = samplesRef.current.slice(0, 16000 * 15);
      const spokenText = liveTranscriptRef.current.trim();

      let res;
      if (spokenText || captured.length > 0) {
        res = await tauriApi.stopRecording({
          samples: captured.slice(0, 3200),
          transcript: spokenText || undefined,
          auto_inject: true,
        });
      } else {
        res = await tauriApi.stopRecording({ auto_inject: true });
      }

      const text = spokenText || (res && res.text ? res.text : "");
      setTranscript(text);
      setState("done");

      if (text) {
        // Auto-copy to system clipboard
        if (navigator.clipboard) {
          navigator.clipboard.writeText(text).catch(() => {});
        }

        // If spoken text matches a desktop voice command, also dispatch it!
        const lower = text.toLowerCase();
        if (
          lower.startsWith("open ") ||
          lower.startsWith("close ") ||
          lower.startsWith("center ") ||
          lower.startsWith("snap ") ||
          lower.startsWith("maximize") ||
          lower.startsWith("minimize") ||
          lower.startsWith("resize ") ||
          lower.startsWith("make window") ||
          lower.startsWith("next window") ||
          lower.startsWith("previous window") ||
          lower.startsWith("focus ") ||
          lower.startsWith("switch to ")
        ) {
          await tauriApi.triggerAction(text).catch(() => {});
        } else {
          // Native text injection into the focused application window
          await tauriApi.injectText(text).catch(() => {});
        }
      }

      // Play completion confirmation chime
      audioFeedback.playCompleteCue();

      // Notify transcript history listeners
      window.dispatchEvent(new CustomEvent("airctrl-transcript-added", { detail: { text } }));

      // Handy Parity: Auto-dismiss pill overlay after 1.2s delay
      if (standalone) {
        autoHideTimerRef.current = setTimeout(async () => {
          if (onClose) {
            onClose();
          } else {
            await tauriApi.toggleSttOverlay().catch(() => {});
          }
        }, 1200);
      }
    } catch (err: any) {
      console.error("Transcription error:", err);
      const fallback = liveTranscriptRef.current || "No speech detected — speak clearly into mic";
      setTranscript(fallback);
      setState("done");
      audioFeedback.playCompleteCue();

      if (standalone) {
        autoHideTimerRef.current = setTimeout(async () => {
          if (onClose) {
            onClose();
          } else {
            await tauriApi.toggleSttOverlay().catch(() => {});
          }
        }, 1200);
      }
    }
  };

  // Cancel Recording immediately without transcribing or pasting
  const cancelRecording = async () => {
    if (autoHideTimerRef.current) {
      clearTimeout(autoHideTimerRef.current);
      autoHideTimerRef.current = null;
    }

    if (recognitionRef.current) {
      try {
        recognitionRef.current.stop();
      } catch {
        // ignore
      }
      recognitionRef.current = null;
    }

    if (mediaStreamRef.current) {
      mediaStreamRef.current.getTracks().forEach((t) => t.stop());
      mediaStreamRef.current = null;
    }
    if (audioCtxRef.current) {
      audioCtxRef.current.close().catch(() => {});
      audioCtxRef.current = null;
    }

    samplesRef.current = [];
    liveTranscriptRef.current = "";
    setState("idle");
    setTranscript("");

    // Play cancel blip
    audioFeedback.playCancelCue();

    if (standalone) {
      if (onClose) {
        onClose();
      } else {
        await tauriApi.toggleSttOverlay().catch(() => {});
      }
    }
  };

  // Smart Dual-Mode Keyboard Listener
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Escape cancels anytime
      if (e.key === "Escape") {
        e.preventDefault();
        cancelRecording();
        return;
      }

      // Hotkey detection: Space or Ctrl+Space
      const isHotkey = e.code === "Space" || (e.ctrlKey && e.code === "Space");
      if (isHotkey && !e.repeat) {
        if (state === "idle") {
          e.preventDefault();
          keyDownTimeRef.current = Date.now();
          startRecording();
        } else if (state === "recording") {
          // Toggle off if already listening
          e.preventDefault();
          stopRecording();
        }
      }
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      const isHotkey = e.code === "Space" || (e.ctrlKey && e.code === "Space");
      if (isHotkey && state === "recording") {
        const heldDuration = Date.now() - keyDownTimeRef.current;
        if (heldDuration >= 250) {
          // Push-to-Talk completed: user held key down and released!
          e.preventDefault();
          stopRecording();
        }
        // If < 250ms, it was a quick tap -> stay listening in Toggle mode!
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("keyup", handleKeyUp);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("keyup", handleKeyUp);
      if (autoHideTimerRef.current) clearTimeout(autoHideTimerRef.current);
    };
  }, [state]);

  const handleCopy = () => {
    if (transcript) {
      navigator.clipboard.writeText(transcript);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const formatTimer = (secs: number) => {
    const m = Math.floor(secs / 60).toString().padStart(2, "0");
    const s = (secs % 60).toString().padStart(2, "0");
    return `${m}:${s}`;
  };

  // Container placement: Bottom Center (Handy default) vs Top Center
  const containerAlignment = position === "top" ? "items-start pt-3" : "items-end pb-3";

  return (
    <div
      className={`select-none font-sans transition-all duration-300 ${
        standalone
          ? `fixed inset-0 flex justify-center ${containerAlignment} pointer-events-none p-3 z-50`
          : "w-full"
      }`}
    >
      {/* Floating Handy Pill Container */}
      <div
        className={`pointer-events-auto rounded-full bg-[#121113]/95 border border-[#d87943]/40 shadow-2xl shadow-[#d87943]/20 backdrop-blur-2xl flex items-center justify-between gap-3 px-4 py-2 transition-all duration-300 ${
          standalone
            ? "w-auto min-w-[340px] max-w-[440px] h-[52px]"
            : "w-full max-w-xl mx-auto h-[54px]"
        }`}
      >
        {/* Left Section: State Indicator & Trigger */}
        <div className="flex items-center gap-2.5 min-w-0">
          <button
            onClick={() => {
              if (state === "recording") stopRecording();
              else if (state === "idle" || state === "done") startRecording();
            }}
            className={`w-7 h-7 rounded-full flex items-center justify-center transition-all duration-300 shrink-0 ${
              state === "recording"
                ? "bg-[#d87943] text-white animate-pulse shadow-md shadow-[#d87943]/50"
                : state === "transcribing"
                ? "bg-[#527575] text-white shadow-md shadow-[#527575]/40"
                : state === "done"
                ? "bg-emerald-600 text-white shadow-md shadow-emerald-600/30"
                : "bg-[#201e22] text-[#d87943] hover:bg-[#d87943] hover:text-white border border-[#d87943]/30"
            }`}
            title={state === "recording" ? "Stop dictation" : "Start dictation (Ctrl+Space)"}
          >
            {state === "recording" ? (
              <Square className="w-3 h-3 fill-current" />
            ) : state === "transcribing" ? (
              <Loader2 className="w-3.5 h-3.5 animate-spin" />
            ) : state === "done" ? (
              <Check className="w-3.5 h-3.5 stroke-[3]" />
            ) : (
              <Mic className="w-3.5 h-3.5" />
            )}
          </button>

          {/* State Text & Visual Badge */}
          <div className="flex flex-col min-w-0">
            <span className="text-xs font-semibold text-foreground flex items-center gap-1.5 truncate">
              {state === "recording" ? (
                <>
                  <span className="w-2 h-2 rounded-full bg-[#d87943] animate-ping shrink-0" />
                  <span className="text-[#d87943] font-medium">Listening...</span>
                </>
              ) : state === "transcribing" ? (
                <span className="text-[#527575] font-medium">Transcribing...</span>
              ) : state === "done" ? (
                <span className="text-emerald-400 font-medium flex items-center gap-1">
                  Pasted ✓
                </span>
              ) : (
                <span className="text-muted-foreground font-medium">Dictate</span>
              )}
            </span>
            <span className="text-[10px] text-muted-foreground/80 font-mono truncate">
              {state === "recording"
                ? transcript
                  ? `"${transcript.slice(-28)}"`
                  : "Speak into microphone..."
                : state === "transcribing"
                ? "Processing Speech..."
                : state === "done"
                ? transcript
                  ? `"${transcript.slice(0, 26)}${transcript.length > 26 ? "…" : ""}"`
                  : "Injected into app"
                : "Ctrl+Spacebar"}
            </span>
          </div>
        </div>

        {/* Center Section: Handy Equalizer Audio Waveform */}
        <div className="flex items-center gap-0.5 px-2.5 py-1 bg-[#18171a] rounded-full border border-border/40 shrink-0 h-7">
          {waveHeights.map((h, i) => (
            <span
              key={i}
              style={{ height: `${h}px` }}
              className={`w-1 rounded-full transition-all duration-75 ${
                state === "recording"
                  ? i % 2 === 0
                    ? "bg-[#d87943]"
                    : "bg-[#527575]"
                  : state === "transcribing"
                  ? "bg-[#d87943] opacity-80"
                  : state === "done"
                  ? "bg-emerald-500/80"
                  : "bg-muted-foreground/30"
              }`}
            />
          ))}
        </div>

        {/* Right Section: Timer, Sound Toggle & Dismiss */}
        <div className="flex items-center gap-1.5 shrink-0">
          {state === "recording" && (
            <div className="flex items-center gap-1 bg-[#18171a] px-2 py-0.5 rounded-full border border-border/50 text-[10px] font-mono">
              <span className="text-foreground font-semibold">{formatTimer(elapsedSecs)}</span>
              <span className="text-muted-foreground/50">|</span>
              <span className="text-[#d87943] text-[9px]">{dbfsLevel}dB</span>
            </div>
          )}

          {/* Sound cue mute toggle */}
          <button
            onClick={() => setSoundOn(!soundOn)}
            className="p-1 rounded-full text-muted-foreground hover:text-foreground hover:bg-[#201e22] transition-colors"
            title={soundOn ? "Mute audio cues" : "Unmute audio cues"}
          >
            {soundOn ? <Volume2 className="w-3 h-3 text-[#d87943]" /> : <VolumeX className="w-3 h-3" />}
          </button>

          {state === "done" && (
            <button
              onClick={handleCopy}
              className="p-1 rounded-full text-muted-foreground hover:text-emerald-400 hover:bg-[#201e22] transition-colors"
              title="Copy to clipboard"
            >
              {copied ? <Check className="w-3 h-3 text-emerald-400" /> : <Copy className="w-3 h-3" />}
            </button>
          )}

          {/* Dismiss button */}
          <button
            onClick={cancelRecording}
            className="p-1 rounded-full text-muted-foreground hover:text-foreground hover:bg-[#201e22] transition-colors"
            title="Dismiss / Cancel (Esc)"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </div>
  );
};
