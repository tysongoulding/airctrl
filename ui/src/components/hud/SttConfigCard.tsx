import React, { useState, useEffect } from "react";
import {
  Mic,
  HardDrive,
  CheckCircle2,
  Info,
  Sparkles,
  Radio,
  Volume2,
  Check,
  BookOpen,
  History,
  Bug,
  Plus,
  Trash2,
  Search,
  RefreshCw,
} from "lucide-react";
import { SettingsState } from "../../types/config";
import { tauriApi } from "../../services/ipc";
import { SttOverlay } from "../stt/SttOverlay";

interface Props {
  settings: SettingsState;
  onUpdate: (partial: Partial<SettingsState>) => void;
}

const HANDY_MODELS = [
  {
    id: "parakeet-v3",
    path: "models/parakeet-unified-en-0.6b-Q8_0.gguf",
    label: "Parakeet V3 (0.6B GGUF)",
    desc: "Fastest CPU-optimized engine with auto language detection",
    badge: "Recommended",
  },
  {
    id: "whisper-small",
    path: "models/ggml-small.en.bin",
    label: "Whisper Small (.bin)",
    desc: "Balanced offline GGML model (~466 MB)",
    badge: "GGML",
  },
  {
    id: "whisper-medium",
    path: "models/ggml-medium.en.bin",
    label: "Whisper Medium (.bin)",
    desc: "High accuracy offline GGML model (~1.5 GB)",
    badge: "GGML",
  },
  {
    id: "whisper-turbo",
    path: "models/ggml-large-v3-turbo.bin",
    label: "Whisper Large V3 Turbo",
    desc: "Pruned decoder high-speed Large V3 (~1.6 GB)",
    badge: "Turbo",
  },
  {
    id: "whisper-large",
    path: "models/ggml-large-v3.bin",
    label: "Whisper Large V3 (.bin)",
    desc: "Maximum multilingual & technical accuracy (~3.1 GB)",
    badge: "Max Accuracy",
  },
  {
    id: "whisper-tiny",
    path: "models/ggml-tiny.en.bin",
    label: "Whisper Tiny (.bin)",
    desc: "Ultra-low memory footprint (~75 MB)",
    badge: "Lightweight",
  },
];

export const SttConfigCard: React.FC<Props> = ({ settings, onUpdate }) => {
  const currentMode = settings.stt_mode ?? (settings.push_to_talk ? "smart_dual" : "toggle");
  const overlayPos = settings.stt_overlay_position ?? "bottom";
  const audioCues = settings.stt_audio_cues ?? true;
  const autoPaste = settings.stt_auto_paste ?? true;
  const reliablePaste = settings.stt_reliable_paste ?? true;
  const pasteDelayBefore = settings.stt_paste_delay_before_ms ?? 50;
  const pasteDelayAfter = settings.stt_paste_delay_after_ms ?? 50;
  const postProcess = settings.stt_post_process ?? true;

  const [dictionary, setDictionary] = useState<Array<[string, string]>>([
    ["air control", "airctrl"],
    ["meet a lee", "Meetily"],
    ["media pipe", "MediaPipe"],
  ]);
  const [newSpoken, setNewSpoken] = useState("");
  const [newReplacement, setNewReplacement] = useState("");
  const [historyQuery, setHistoryQuery] = useState("");
  const [historyItems, setHistoryItems] = useState<any[]>([]);
  const [discoveredModels, setDiscoveredModels] = useState<any[]>([]);
  const [debugDrawerOpen, setDebugDrawerOpen] = useState(false);

  useEffect(() => {
    tauriApi.listModels().then((res: any) => {
      const models = Array.isArray(res) ? res : res?.models;
      if (Array.isArray(models) && models.length > 0) {
        setDiscoveredModels(models);
      }
    });
    const refreshHistory = () => {
      tauriApi.searchTranscripts("").then((res: any) => {
        const items = Array.isArray(res) ? res : res?.results;
        if (Array.isArray(items)) {
          setHistoryItems(items);
        }
      });
    };
    refreshHistory();
    window.addEventListener("airctrl-transcript-added", refreshHistory);
    return () => window.removeEventListener("airctrl-transcript-added", refreshHistory);
  }, []);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === "d") {
        e.preventDefault();
        setDebugDrawerOpen((prev) => !prev);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const handleModeChange = (mode: "smart_dual" | "push_to_talk" | "toggle" | "continuous") => {
    onUpdate({
      stt_mode: mode,
      push_to_talk: mode === "smart_dual" || mode === "push_to_talk",
    });
    tauriApi.setRecordingMode(mode);
  };

  const handleAddDictionaryEntry = async () => {
    if (!newSpoken.trim() || !newReplacement.trim()) return;
    const updated: Array<[string, string]> = [
      ...dictionary.filter(([k]) => k.toLowerCase() !== newSpoken.trim().toLowerCase()),
      [newSpoken.trim(), newReplacement.trim()],
    ];
    setDictionary(updated);
    setNewSpoken("");
    setNewReplacement("");
    await tauriApi.updateDictionary(Object.fromEntries(updated));
  };

  const handleRemoveDictionaryEntry = async (spoken: string) => {
    const updated = dictionary.filter(([k]) => k !== spoken);
    setDictionary(updated);
    await tauriApi.updateDictionary(Object.fromEntries(updated));
  };

  const handleSearchHistory = async (q: string) => {
    setHistoryQuery(q);
    const res: any = await tauriApi.searchTranscripts(q);
    const items = Array.isArray(res) ? res : res?.results;
    if (Array.isArray(items)) {
      setHistoryItems(items);
    }
  };

  return (
    <div className="p-6 rounded-2xl border border-border bg-card/60 backdrop-blur-xl flex flex-col gap-6 shadow-sm">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-border pb-4">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-lg bg-muted border border-border flex items-center justify-center text-foreground">
            <Mic className="w-4 h-4" />
          </div>
          <div>
            <h2 className="text-sm font-semibold text-foreground flex items-center gap-2">
              Speech-to-Text Configuration
              <span className="px-2 py-0.5 text-[10px] rounded bg-[#d87943]/10 border border-[#d87943]/20 text-[#d87943] font-mono font-medium">
                Handy 1:1 Parity
              </span>
            </h2>
            <p className="text-xs text-muted-foreground mt-0.5">
              Offline Whisper &amp; Parakeet V3 GGUF CPU engine with Smart Dual-Mode, Reliable Paste, Custom Dictionary &amp; History
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setDebugDrawerOpen((prev) => !prev)}
            title="Toggle Handy Debug Drawer (Ctrl+Shift+D)"
            className={`px-2.5 py-1 rounded-md border text-xs font-mono flex items-center gap-1.5 transition-all ${
              debugDrawerOpen
                ? "bg-[#d87943]/20 border-[#d87943] text-[#d87943]"
                : "bg-muted border-border text-muted-foreground hover:text-foreground"
            }`}
          >
            <Bug className="w-3.5 h-3.5" /> Debug (Ctrl+Shift+D)
          </button>
          <span className="px-2.5 py-1 rounded-md bg-muted border border-border text-foreground text-xs font-mono flex items-center gap-1.5">
            <CheckCircle2 className="w-3.5 h-3.5 text-emerald-500" /> Engine Ready
          </span>
        </div>
      </div>

      {/* Grid: Hotkey & 4 Capture Modes */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Global STT Hotkey */}
        <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
          <div>
            <label className="text-xs font-medium text-foreground block mb-1">
              Global Transcription Hotkey
            </label>
            <p className="text-xs text-muted-foreground mb-3">
              Press to activate push-to-talk, toggle, or continuous hands-free dictation
            </p>
          </div>
          <div className="flex items-center justify-between">
            <select
              value={settings.hotkey_stt}
              onChange={(e) => onUpdate({ hotkey_stt: e.target.value })}
              className="px-3 py-1.5 rounded-md bg-background border border-border text-foreground font-mono text-xs font-medium focus:outline-none focus:ring-1 focus:ring-ring"
            >
              <option value="Ctrl+Spacebar">Ctrl+Spacebar (Handy Default)</option>
              <option value="Ctrl+Win">Ctrl+Win (Safe Intercept)</option>
              <option value="Ctrl+Win+Space">Ctrl+Win+Space</option>
              <option value="Ctrl+Shift+Space">Ctrl+Shift+Space</option>
            </select>
            <span className="text-[11px] text-muted-foreground font-mono">Default: Ctrl+Spacebar</span>
          </div>
        </div>

        {/* Handy 4 Recording Modes */}
        <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
          <div>
            <label className="text-xs font-medium text-foreground block mb-1">
              Recording &amp; Trigger Mode (4 Modes)
            </label>
            <p className="text-xs text-muted-foreground mb-3">
              Smart Dual (&ge;250ms hold / &lt;250ms tap), Hold-Only, Toggle-Only, or Continuous Hands-Free VAD
            </p>
          </div>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-1.5">
            {[
              { id: "smart_dual" as const, label: "Smart Dual" },
              { id: "push_to_talk" as const, label: "Hold Only" },
              { id: "toggle" as const, label: "Toggle Only" },
              { id: "continuous" as const, label: "Hands-Free VAD" },
            ].map((m) => (
              <button
                key={m.id}
                onClick={() => handleModeChange(m.id)}
                className={`px-2 py-1.5 rounded-md text-[11px] font-medium border text-center transition-all ${
                  currentMode === m.id
                    ? "bg-[#d87943] text-white border-[#d87943] shadow-sm font-semibold"
                    : "bg-background border-border text-muted-foreground hover:text-foreground"
                }`}
              >
                {m.label}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Grid: 4 Overlay Positions, Acoustic Feedback, and Reliable Paste */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {/* 4-Way Overlay Position */}
        <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
          <div>
            <label className="text-xs font-medium text-foreground block mb-1">
              Overlay Position (4-Way)
            </label>
            <p className="text-xs text-muted-foreground mb-3">
              Floating pill placement (&quot;None&quot; runs 100% invisible)
            </p>
          </div>
          <div className="grid grid-cols-2 gap-1.5">
            {(["bottom", "top", "center", "none"] as const).map((pos) => (
              <button
                key={pos}
                onClick={() => {
                  onUpdate({ stt_overlay_position: pos });
                  tauriApi.setOverlayPosition(pos);
                }}
                className={`py-1.5 px-2 rounded-md text-xs font-medium border text-center capitalize transition-all ${
                  overlayPos === pos
                    ? "bg-[#527575] text-white border-[#527575] font-semibold"
                    : "bg-background border-border text-muted-foreground hover:text-foreground"
                }`}
              >
                {pos}
              </button>
            ))}
          </div>
        </div>

        {/* Audio Chimes & Post-Processing */}
        <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between gap-2 shadow-sm">
          <div className="flex items-center justify-between">
            <div>
              <label className="text-xs font-medium text-foreground flex items-center gap-1.5">
                <Volume2 className="w-3.5 h-3.5 text-[#d87943]" /> Acoustic Feedback
              </label>
              <p className="text-[11px] text-muted-foreground">Start/stop synthesized audio cues</p>
            </div>
            <button
              onClick={() => onUpdate({ stt_audio_cues: !audioCues })}
              className={`relative inline-flex h-5 w-10 items-center rounded-full transition-colors ${
                audioCues ? "bg-[#d87943]" : "bg-muted border border-border"
              }`}
            >
              <span
                className={`inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow transition-transform ${
                  audioCues ? "translate-x-5" : "translate-x-1"
                }`}
              />
            </button>
          </div>

          <div className="pt-2 border-t border-border/50 flex items-center justify-between">
            <div>
              <label className="text-xs font-medium text-foreground">Post-Processing &amp; Dict</label>
              <p className="text-[11px] text-muted-foreground">Auto-punctuation &amp; vocabulary</p>
            </div>
            <button
              onClick={() => {
                onUpdate({ stt_post_process: !postProcess });
                tauriApi.togglePostProcessing(!postProcess);
              }}
              className={`relative inline-flex h-5 w-10 items-center rounded-full transition-colors ${
                postProcess ? "bg-emerald-600" : "bg-muted border border-border"
              }`}
            >
              <span
                className={`inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow transition-transform ${
                  postProcess ? "translate-x-5" : "translate-x-1"
                }`}
              />
            </button>
          </div>
        </div>

        {/* Reliable Paste & Before/After Delays */}
        <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between gap-2 shadow-sm">
          <div className="flex items-center justify-between">
            <div>
              <label className="text-xs font-medium text-foreground flex items-center gap-1.5">
                <Check className="w-3.5 h-3.5 text-emerald-400" /> Reliable Paste &amp; Restore
              </label>
              <p className="text-[11px] text-muted-foreground">
                Preserves clipboard &amp; injects with custom delays
              </p>
            </div>
            <button
              onClick={() => {
                onUpdate({ stt_auto_paste: !autoPaste, stt_reliable_paste: !reliablePaste });
              }}
              className={`relative inline-flex h-5 w-10 items-center rounded-full transition-colors ${
                reliablePaste ? "bg-emerald-600" : "bg-muted border border-border"
              }`}
            >
              <span
                className={`inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow transition-transform ${
                  reliablePaste ? "translate-x-5" : "translate-x-1"
                }`}
              />
            </button>
          </div>

          <div className="pt-2 border-t border-border/50 grid grid-cols-2 gap-2">
            <div>
              <span className="text-[10px] text-muted-foreground block">Delay Before (ms)</span>
              <input
                type="number"
                min={0}
                max={1000}
                value={pasteDelayBefore}
                onChange={(e) => {
                  const val = Math.max(0, parseInt(e.target.value || "0", 10));
                  onUpdate({ stt_paste_delay_before_ms: val });
                }}
                className="w-full mt-0.5 px-2 py-1 rounded bg-background border border-border text-xs font-mono text-foreground"
              />
            </div>
            <div>
              <span className="text-[10px] text-muted-foreground block">Delay After (ms)</span>
              <input
                type="number"
                min={0}
                max={1000}
                value={pasteDelayAfter}
                onChange={(e) => {
                  const val = Math.max(0, parseInt(e.target.value || "0", 10));
                  onUpdate({ stt_paste_delay_after_ms: val });
                }}
                className="w-full mt-0.5 px-2 py-1 rounded bg-background border border-border text-xs font-mono text-foreground"
              />
            </div>
          </div>
        </div>
      </div>

      {/* Handy Model Catalog: Parakeet V3 GGUF + Whisper Small/Medium/Turbo/Large */}
      <div className="p-4 rounded-lg border border-border bg-card flex flex-col gap-3 shadow-sm">
        <div className="flex items-center justify-between">
          <label className="text-xs font-medium text-foreground flex items-center gap-2">
            <HardDrive className="w-3.5 h-3.5 text-[#d87943]" /> Handy Model Catalog (Whisper .bin &amp; Parakeet V3 .gguf)
          </label>
          <div className="flex items-center gap-2">
            <span className="text-xs text-muted-foreground font-mono">
              Active: {settings.whisper_model}
            </span>
            <button
              onClick={() =>
                tauriApi.listModels().then((res: any) => {
                  const models = Array.isArray(res) ? res : res?.models;
                  if (Array.isArray(models)) setDiscoveredModels(models);
                })
              }
              className="px-2 py-1 rounded bg-muted hover:bg-muted/80 border border-border text-[10px] font-mono text-foreground flex items-center gap-1"
            >
              <RefreshCw className="w-3 h-3" /> Scan Models
            </button>
          </div>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2.5">
          {HANDY_MODELS.map((m) => {
            const isSelected = settings.whisper_model === m.path;
            return (
              <button
                key={m.id}
                onClick={() => {
                  onUpdate({ whisper_model: m.path });
                  tauriApi.selectModel(m.id);
                }}
                className={`p-3 rounded-lg border text-left transition-all flex flex-col justify-between gap-1.5 ${
                  isSelected
                    ? "bg-primary text-primary-foreground border-primary shadow-sm"
                    : "bg-background border-border text-muted-foreground hover:text-foreground hover:bg-muted/50"
                }`}
              >
                <div className="flex items-center justify-between w-full">
                  <span className="font-bold text-xs">{m.label}</span>
                  <span
                    className={`text-[9px] font-mono px-1.5 py-0.5 rounded ${
                      isSelected
                        ? "bg-primary-foreground/20 text-primary-foreground"
                        : "bg-muted text-foreground"
                    }`}
                  >
                    {m.badge}
                  </span>
                </div>
                <div
                  className={`text-[10px] leading-snug ${
                    isSelected ? "text-primary-foreground/85" : "text-muted-foreground"
                  }`}
                >
                  {m.desc}
                </div>
              </button>
            );
          })}
        </div>
        {discoveredModels.length > 0 && (
          <div className="text-[11px] font-mono text-muted-foreground pt-1 border-t border-border/50">
            Discovered in local directories (&#123;app_data_dir&#125;/models/ &amp; ~/.cache/huggingface/hub):{" "}
            <span className="text-foreground font-semibold">{discoveredModels.length} models ready</span>
          </div>
        )}
      </div>

      {/* Custom Vocabulary Dictionary & Searchable Transcript History */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        {/* Custom Dictionary */}
        <div className="p-4 rounded-lg border border-border bg-card flex flex-col gap-3 shadow-sm">
          <div className="flex items-center justify-between">
            <label className="text-xs font-semibold text-foreground flex items-center gap-1.5">
              <BookOpen className="w-3.5 h-3.5 text-[#d87943]" /> Custom Vocabulary &amp; Dictionary
            </label>
            <span className="text-[10px] font-mono text-muted-foreground">
              {dictionary.length} Replacements
            </span>
          </div>
          <div className="flex items-center gap-2">
            <input
              type="text"
              value={newSpoken}
              onChange={(e) => setNewSpoken(e.target.value)}
              placeholder="Spoken phrase (e.g. air control)"
              className="flex-1 px-2.5 py-1.5 rounded bg-background border border-border text-xs font-mono text-foreground"
            />
            <input
              type="text"
              value={newReplacement}
              onChange={(e) => setNewReplacement(e.target.value)}
              placeholder="Replacement (e.g. airctrl)"
              className="flex-1 px-2.5 py-1.5 rounded bg-background border border-border text-xs font-mono text-foreground"
            />
            <button
              onClick={handleAddDictionaryEntry}
              className="px-2.5 py-1.5 rounded bg-[#d87943] text-white text-xs font-medium flex items-center gap-1"
            >
              <Plus className="w-3.5 h-3.5" /> Add
            </button>
          </div>
          <div className="max-h-32 overflow-y-auto flex flex-col gap-1.5 pr-1">
            {dictionary.map(([spoken, replacement]) => (
              <div
                key={spoken}
                className="flex items-center justify-between px-2.5 py-1.5 rounded bg-background border border-border text-xs font-mono"
              >
                <div>
                  <span className="text-muted-foreground">&quot;{spoken}&quot;</span>
                  <span className="mx-2 text-[#d87943]">&rarr;</span>
                  <span className="text-foreground font-semibold">&quot;{replacement}&quot;</span>
                </div>
                <button
                  onClick={() => handleRemoveDictionaryEntry(spoken)}
                  className="text-muted-foreground hover:text-rose-500 transition-colors"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </button>
              </div>
            ))}
          </div>
        </div>

        {/* Searchable Transcript History */}
        <div className="p-4 rounded-lg border border-border bg-card flex flex-col gap-3 shadow-sm">
          <div className="flex items-center justify-between">
            <label className="text-xs font-semibold text-foreground flex items-center gap-1.5">
              <History className="w-3.5 h-3.5 text-[#527575]" /> Searchable Transcript History
            </label>
            <span className="text-[10px] font-mono text-muted-foreground">
              Local Ring Archive
            </span>
          </div>
          <div className="relative">
            <Search className="w-3.5 h-3.5 text-muted-foreground absolute left-2.5 top-2" />
            <input
              type="text"
              value={historyQuery}
              onChange={(e) => handleSearchHistory(e.target.value)}
              placeholder="Search past dictations & voice commands..."
              className="w-full pl-8 pr-3 py-1.5 rounded bg-background border border-border text-xs font-mono text-foreground"
            />
          </div>
          <div className="max-h-32 overflow-y-auto flex flex-col gap-1.5 pr-1">
            {historyItems.length === 0 ? (
              <div className="text-xs text-muted-foreground font-mono py-3 text-center">
                No matching transcripts yet. Speak with Ctrl+Spacebar to populate history.
              </div>
            ) : (
              historyItems.map((item, idx) => (
                <div
                  key={item.id || idx}
                  className="px-2.5 py-1.5 rounded bg-background border border-border text-xs flex items-center justify-between gap-2"
                >
                  <span className="text-foreground truncate">{item.text || JSON.stringify(item)}</span>
                  <span className="text-[10px] font-mono text-muted-foreground shrink-0">
                    {item.model_used || "local"}
                  </span>
                </div>
              ))
            )}
          </div>
        </div>
      </div>

      {/* Ctrl+Shift+D Debug Drawer */}
      {debugDrawerOpen && (
        <div className="p-4 rounded-xl border border-[#d87943]/50 bg-[#141315] flex flex-col gap-2 font-mono text-xs">
          <div className="flex items-center justify-between text-[#d87943] font-semibold">
            <span>HANDY DIAGNOSTIC &amp; CLI FLAGS DRAWER (Ctrl+Shift+D)</span>
            <button onClick={() => setDebugDrawerOpen(false)} className="text-muted-foreground hover:text-white">
              Close
            </button>
          </div>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-[11px] text-muted-foreground">
            <div>CLI Flags: <code>--toggle-transcription --toggle-post-process --cancel --start-hidden --no-tray --debug</code></div>
            <div>Paste Pipeline: <code>ReliablePaste={String(reliablePaste)} (Before={pasteDelayBefore}ms, After={pasteDelayAfter}ms)</code></div>
            <div>Model Discovery: <code>&#123;app_data_dir&#125;/models/ &amp; ~/.cache/huggingface/hub</code></div>
            <div>Active Mode: <code>{currentMode} | Overlay={overlayPos}</code></div>
          </div>
        </div>
      )}

      {/* Safe Hotkey Explanation */}
      <div className="p-3.5 rounded-lg border border-border bg-muted/40 flex items-start gap-3">
        <Info className="w-4 h-4 text-foreground shrink-0 mt-0.5" />
        <div className="text-xs text-muted-foreground leading-relaxed">
          <span className="font-semibold text-foreground">Zero Focus-Stealing: </span>
          The floating pill overlay never takes keyboard focus away from your active editor, browser, or chat window.
          Press and hold <code className="text-foreground font-mono px-1 py-0.5 bg-background border border-border rounded">Ctrl+Spacebar</code> to speak; release to instantly paste into your cursor.
        </div>
      </div>

      {/* Handy-Style Voice Dictation Overlay Section */}
      <div className="p-6 rounded-2xl border border-[#d87943]/30 bg-gradient-to-b from-card/80 to-card/40 backdrop-blur-xl flex flex-col gap-4 shadow-lg shadow-[#d87943]/5">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-border/70 pb-4">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-xl bg-[#d87943] text-white flex items-center justify-center shadow-md shadow-[#d87943]/30">
              <Sparkles className="w-4 h-4" />
            </div>
            <div>
              <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
                Handy Floating Pill Dictation Overlay
                <span className="px-2 py-0.5 text-[10px] rounded bg-[#d87943]/10 border border-[#d87943]/20 text-[#d87943] font-mono font-medium uppercase">
                  {overlayPos}
                </span>
              </h3>
              <p className="text-xs text-muted-foreground mt-0.5">
                Minimalist floating pill with live real-time equalizer soundwaves, audio metering, and auto-injection
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={() => tauriApi.toggleSttOverlay()}
              className="flex items-center gap-2 px-3.5 py-1.5 rounded-lg bg-[#d87943] text-white hover:bg-[#d87943]/90 text-xs font-semibold shadow-md shadow-[#d87943]/20 transition-all whitespace-nowrap"
            >
              <Radio className="w-3.5 h-3.5" />
              <span>Launch Desktop Overlay</span>
            </button>
          </div>
        </div>

        {/* Live Interactive In-HUD Overlay Tester */}
        <div className="pt-1">
          <div className="text-[11px] font-mono text-muted-foreground mb-2 flex items-center justify-between">
            <span>LIVE INTERACTIVE OVERLAY PREVIEW:</span>
            <span className="text-[#d87943] font-medium">Click mic or hold/tap Space to test</span>
          </div>
          <SttOverlay standalone={false} position={overlayPos} soundEnabled={audioCues} />
        </div>
      </div>
    </div>
  );
};
