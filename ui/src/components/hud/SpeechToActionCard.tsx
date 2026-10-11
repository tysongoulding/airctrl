import React, { useState } from "react";
import { Zap, Bot, Terminal, Play, Sparkles, Cpu } from "lucide-react";
import { SettingsState, ApplicationItem } from "../../types/config";
import { tauriApi } from "../../services/ipc";
import { AppCatalogCard } from "./AppCatalogCard";

interface Props {
  settings: SettingsState;
  onUpdate: (partial: Partial<SettingsState>) => void;
  onAssignToSector?: (sectorId: number, app: ApplicationItem) => void;
}

const AGENTS = [
  { id: "cursor", name: "Cursor IDE", binary: "cursor.exe", icon: Cpu, desc: "In-editor AI coding assistant & agent" },
  { id: "claude", name: "Claude Code", binary: "claude", icon: Bot, desc: "Terminal-driven autonomous software engineering" },
  { id: "antigravity", name: "Google Antigravity", binary: "agy", icon: Sparkles, desc: "Multi-agent autonomous development harness" },
  { id: "grok", name: "Grokbot", binary: "grok", icon: Terminal, desc: "Fast conversational assistant & shell dispatcher" },
];

export const SpeechToActionCard: React.FC<Props> = ({ settings, onUpdate, onAssignToSector }) => {
  const [testCommand, setTestCommand] = useState("Focus Cursor");
  const [dispatching, setDispatching] = useState(false);
  const [dispatchResult, setDispatchResult] = useState<{ success: boolean; message: string } | null>(null);
  const [subView, setSubView] = useState<"agents" | "catalog">("agents");

  const hotkeySta = settings.hotkey_sta || "Ctrl+Shift+Space";
  const selectedAgent = settings.sta_agent_target || "cursor";

  const handleTestDispatch = async (cmdToRun?: string) => {
    const cmd = cmdToRun || testCommand;
    if (!cmd.trim()) return;
    setDispatching(true);
    setDispatchResult(null);
    try {
      const res = await tauriApi.triggerAction(cmd);
      if (res?.success) {
        setDispatchResult({
          success: true,
          message: `Dispatched "${cmd}" successfully to action router.`,
        });
      } else {
        setDispatchResult({
          success: false,
          message: `Action router returned unexpected response for "${cmd}".`,
        });
      }
    } catch (err: any) {
      setDispatchResult({
        success: false,
        message: `Execution failed: ${err?.message || err}`,
      });
    } finally {
      setDispatching(false);
      setTimeout(() => setDispatchResult(null), 5000);
    }
  };

  return (
    <div className="flex flex-col gap-6">
      {/* Top Header Card */}
      <div className="p-6 rounded-2xl border border-border bg-card/60 backdrop-blur-xl flex flex-col gap-6 shadow-sm">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 border-b border-border pb-4">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-lg bg-primary text-primary-foreground flex items-center justify-center shadow-sm">
              <Zap className="w-4 h-4" />
            </div>
            <div>
              <h2 className="text-sm font-semibold text-foreground flex items-center gap-2">
                Speech-to-Action (STA) Engine
                <span className="px-2 py-0.5 text-[10px] rounded bg-emerald-500/10 border border-emerald-500/20 text-emerald-500 font-mono">
                  Router Active
                </span>
              </h2>
              <p className="text-xs text-muted-foreground mt-0.5">
                Semantic voice command parser, autonomous AI agent dispatch, and OS window crawler
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <div className="flex items-center gap-1 bg-muted p-1 rounded-lg border border-border">
              <button
                onClick={() => setSubView("agents")}
                className={`px-3 py-1 rounded-md text-xs font-medium transition-all ${
                  subView === "agents"
                    ? "bg-background text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground"
                }`}
              >
                Agent Dispatch & Keybinds
              </button>
              <button
                onClick={() => setSubView("catalog")}
                className={`px-3 py-1 rounded-md text-xs font-medium transition-all ${
                  subView === "catalog"
                    ? "bg-background text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground"
                }`}
              >
                App Catalog & Crawler
              </button>
            </div>
          </div>
        </div>

        {subView === "agents" ? (
          <>
            {/* Keybinds & Intent Parser Row */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              {/* Global Voice Action Hotkey */}
              <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
                <div>
                  <label className="text-xs font-medium text-foreground block mb-1">
                    Voice Action Hotkey
                  </label>
                  <p className="text-xs text-muted-foreground mb-3">
                    Trigger voice action command listening into airctrl router
                  </p>
                </div>
                <div className="flex items-center justify-between">
                  <select
                    value={hotkeySta}
                    onChange={(e) => onUpdate({ hotkey_sta: e.target.value })}
                    className="px-3 py-1.5 rounded-md bg-background border border-border text-foreground font-mono text-xs font-medium focus:outline-none focus:ring-1 focus:ring-ring"
                  >
                    <option value="Ctrl+Shift+Space">Ctrl+Shift+Space (Recommended)</option>
                    <option value="Ctrl+Alt+Space">Ctrl+Alt+Space</option>
                    <option value="Alt+Space">Alt+Space</option>
                    <option value="Ctrl+Win+A">Ctrl+Win+A</option>
                    <option value="Win+Shift+A">Win+Shift+A</option>
                  </select>
                </div>
              </div>

              {/* Intent Matching Sensitivity */}
              <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
                <div>
                  <label className="text-xs font-medium text-foreground block mb-1">
                    Intent Classification Tolerance
                  </label>
                  <p className="text-xs text-muted-foreground mb-3">
                    Fuzzy Jaro-Winkler + Levenshtein intent threshold
                  </p>
                </div>
                <div className="flex items-center justify-between">
                  <select
                    value={settings.sta_fuzzy_threshold ?? 0.75}
                    onChange={(e) => onUpdate({ sta_fuzzy_threshold: parseFloat(e.target.value) })}
                    className="px-3 py-1.5 rounded-md bg-background border border-border text-foreground font-mono text-xs font-medium focus:outline-none focus:ring-1 focus:ring-ring"
                  >
                    <option value="0.85">Strict (0.85) - Exact Commands</option>
                    <option value="0.75">Balanced (0.75) - Default</option>
                    <option value="0.65">Fuzzy (0.65) - Maximum Flexibility</option>
                  </select>
                </div>
              </div>

              {/* Acronym & Shorthand Boost */}
              <div className="p-4 rounded-lg border border-border bg-card flex flex-col justify-between shadow-sm">
                <div>
                  <label className="text-xs font-medium text-foreground block mb-1">
                    Acronym & Shorthand Boost
                  </label>
                  <p className="text-xs text-muted-foreground mb-3">
                    Maps abbreviations like &quot;WT&quot; to Windows Terminal, &quot;VS&quot; to VS Code
                  </p>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-xs text-foreground font-medium">
                    {settings.sta_acronym_boost !== false ? "Acronym Boost Enabled" : "Disabled"}
                  </span>
                  <button
                    onClick={() => onUpdate({ sta_acronym_boost: settings.sta_acronym_boost === false })}
                    className={`relative inline-flex h-5 w-10 items-center rounded-full transition-colors ${
                      settings.sta_acronym_boost !== false ? "bg-primary" : "bg-muted border border-border"
                    }`}
                  >
                    <span
                      className={`inline-block h-3.5 w-3.5 transform rounded-full bg-background shadow transition-transform ${
                        settings.sta_acronym_boost !== false ? "translate-x-5" : "translate-x-1"
                      }`}
                    />
                  </button>
                </div>
              </div>
            </div>

            {/* Live Command Dispatch Sandbox */}
            <div className="p-5 rounded-xl border border-border bg-card/40 flex flex-col gap-3 shadow-inner">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold text-foreground flex items-center gap-1.5">
                  <Play className="w-3.5 h-3.5 text-primary" /> Test Voice Action Dispatcher
                </span>
                <span className="text-[11px] text-muted-foreground font-mono">
                  Simulate voice utterance or command string
                </span>
              </div>

              <div className="flex items-center gap-2">
                <input
                  type="text"
                  value={testCommand}
                  onChange={(e) => setTestCommand(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && handleTestDispatch()}
                  placeholder='Try "Focus Cursor", "Snap Left", "Ask Claude", or "Radial Dial"...'
                  className="flex-1 px-3 py-2 rounded-lg bg-background border border-border text-foreground text-xs placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring font-mono"
                />
                <button
                  onClick={() => handleTestDispatch()}
                  disabled={dispatching || !testCommand.trim()}
                  className="px-4 py-2 bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50 rounded-lg text-xs font-medium shadow-sm transition-all whitespace-nowrap"
                >
                  {dispatching ? "Dispatching..." : "Execute Action"}
                </button>
              </div>

              {/* Preset Quick Chips */}
              <div className="flex items-center gap-2 flex-wrap pt-1">
                <span className="text-[11px] text-muted-foreground">Quick Presets:</span>
                {[
                  "Focus Cursor",
                  "Snap Left",
                  "Snap Right",
                  "Radial Dial",
                  "Ask Claude to review code",
                  "Focus Slack",
                ].map((preset) => (
                  <button
                    key={preset}
                    onClick={() => {
                      setTestCommand(preset);
                      handleTestDispatch(preset);
                    }}
                    className="px-2 py-1 rounded bg-muted/80 hover:bg-muted border border-border text-[11px] font-mono text-foreground transition-colors"
                  >
                    {preset}
                  </button>
                ))}
              </div>

              {/* Result Notice */}
              {dispatchResult && (
                <div
                  className={`p-3 rounded-lg border text-xs font-mono transition-all ${
                    dispatchResult.success
                      ? "bg-emerald-500/10 border-emerald-500/30 text-emerald-500"
                      : "bg-rose-500/10 border-rose-500/30 text-rose-500"
                  }`}
                >
                  {dispatchResult.message}
                </div>
              )}
            </div>

            {/* Autonomous Agent Dispatch Hub */}
            <div className="flex flex-col gap-3">
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="text-xs font-semibold text-foreground flex items-center gap-2">
                    <Bot className="w-4 h-4 text-foreground" /> Supported Autonomous Agent Targets
                  </h3>
                  <p className="text-[11px] text-muted-foreground mt-0.5">
                    Speech-to-Action routes prompts directly to active developer agents and tools
                  </p>
                </div>
                <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-muted border border-border text-foreground">
                  4 Adapters Armed
                </span>
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
                {AGENTS.map((agent) => {
                  const Icon = agent.icon;
                  const isSelected = selectedAgent === agent.id;
                  return (
                    <div
                      key={agent.id}
                      onClick={() => onUpdate({ sta_agent_target: agent.id })}
                      className={`p-4 rounded-xl border cursor-pointer transition-all flex flex-col justify-between ${
                        isSelected
                          ? "bg-primary/5 border-primary shadow-sm"
                          : "bg-card border-border hover:border-border/80 hover:bg-muted/30"
                      }`}
                    >
                      <div className="flex flex-col gap-2">
                        <div className="flex items-center justify-between">
                          <div className={`w-8 h-8 rounded-lg flex items-center justify-center border ${
                            isSelected ? "bg-primary text-primary-foreground border-primary" : "bg-muted border-border text-foreground"
                          }`}>
                            <Icon className="w-4 h-4" />
                          </div>
                          {isSelected && (
                            <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-primary/10 border border-primary/20 text-primary font-medium">
                              Default Target
                            </span>
                          )}
                        </div>
                        <div>
                          <div className="text-xs font-semibold text-foreground">{agent.name}</div>
                          <div className="text-[10px] text-muted-foreground mt-0.5 leading-relaxed">{agent.desc}</div>
                        </div>
                      </div>
                      <div className="mt-3 pt-2 border-t border-border/50 text-[10px] font-mono text-muted-foreground">
                        Binary: {agent.binary}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          </>
        ) : (
          /* Discovered Windows Application Catalog */
          <AppCatalogCard
            onAssignToSector={onAssignToSector}
            sectors={settings.dial_sectors}
          />
        )}
      </div>
    </div>
  );
};
