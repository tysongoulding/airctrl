# AGENTS.md — AI Agent Guide to `airctrl`

> **Welcome, AI Coding Agent.** This document teaches you how to navigate, understand, and modify the `airctrl` codebase with maximum precision and zero context-window waste.

---

## 1. Prime Directive: Navigate via Red Hat Ripwire

Do **NOT** recursively read entire directories or flood your context window with large file dumps. This repository is equipped with **Ripwire from Red Hat Emerging Technologies (`redhat-et/ripwire`)**—the "ripgrep of AI context."

Ripwire parses the repository into a structural symbol graph and ranks symbols using Personalized PageRank, giving you a minified, deterministic map of relevant symbols, callers, callees, and tests in milliseconds.

### Installing / Verifying Ripwire

If `ripwire` is not in your environment `$PATH`, install it instantly:
```bash
RIPWIRE_REPO=redhat-et/ripwire bash -c "$(curl -fsSL https://raw.githubusercontent.com/redhat-et/ripwire/main/scripts/install.sh)"
export PATH="$HOME/.local/bin:$PATH"
```

Verify installation:
```bash
ripwire --version
```

### Essential Ripwire Workflows for Agents

| Workflow | Ripwire Command | When to Use |
|:---|:---|:---|
| **Repository Orientation** | `ripwire . --format=xml` | Obtain a global PageRank-sorted symbol map of the entire codebase before reading files. |
| **Targeted Symbol Query** | `ripwire --query="<symbol>"` | Find definitions, callers, and callees of a specific struct, trait, or function. |
| **Pack Context for a Task** | `ripwire --pack-task="<task description>"` | Pack the minimal, exact symbol context needed to solve a specific engineering goal. |
| **Blast-Radius Analysis** | `ripwire --blast-radius="<symbol>"` | **MANDATORY before refactoring**: Determine every file, function, and caller that will break if you modify a symbol. |
| **Locate Relevant Tests** | `ripwire --tests="<file>"` | Instantly identify which unit and integration test files cover a specific source file. |
| **Safe Deletion Check** | `ripwire --safe-delete="<symbol>"` | Confirm zero references exist across the workspace before deleting code. |
| **Inspect Struct Layout** | `ripwire --layout="<struct>"` | View field offsets, computed byte sizes, and padding (essential for low-level audio/IPC structs). |
| **PR & Diff Evidence** | `ripwire --pr-context` | Build review evidence for a git diff showing modified files, callers, and test coverage. |

### Ripwire MCP Integration (for Claude Code, Cursor, Antigravity)

When running inside an agent harness with MCP support, connect Ripwire as a persistent stdio server:
```json
{
  "mcpServers": {
    "ripwire": {
      "command": "ripwire",
      "args": ["--mcp"]
    }
  }
}
```

---

## 2. Architecture & Modular Crate Map

`airctrl` is structured as a modular Cargo workspace. Each crate has strict boundary responsibilities:

```
crates/
├── airctrl-core/      # Shared foundational layer (zero dependencies on other internal crates)
├── airctrl-lan/       # Embedded Axum server & Windows PowerShell distribution engine
├── airctrl-stt/       # Offline Whisper inference, VAD, ring buffer, & text injection
├── airctrl-actions/   # Semantic command parser, window crawler, agentic dispatch, & REA
├── airctrl-gestures/  # MediaPipe CPU 21-landmark topology, heuristics, & spatial mapper
└── airctrl-meetings/  # Meeting process watcher, atomic camera kill-switch, & audio injector
src-tauri/             # Tauri 2.0 native shell, multi-window transparent overlay, headless IPC
ui/                    # Modern Vite + React + TypeScript + Tailwind CSS UI
```

### Crate-by-Crate Agent Navigation Guide

#### 1. `crates/airctrl-core`
- **Purpose**: Defines shared domain models, events, and cross-platform OS traits.
- **Key Files**:
  - `src/events.rs`: `EventBus` supporting decoupled broadcast channel delivery. Decoupled zero-subscriber behavior returns `Ok(0)` rather than channel errors.
  - `src/platform/mod.rs`: `PlatformWindowOps`, `PlatformInputOps`, `PlatformAudioOps` traits.
  - `src/platform/mock.rs`: Atomic mock platform harnesses with exclusive window focus tracking.
  - `src/types/`: Domain models for STT, actions, gestures, and meetings.

#### 2. `crates/airctrl-lan`
- **Purpose**: Distributes Windows builds from a Linux VM to PCs on the local network.
- **Key Files**:
  - `src/bin/airctrl_lan_daemon.rs`: CLI entrypoint accepting `--port/-p` and `--artifact/-a`.
  - `src/ip.rs`: `get_lan_ip()` resolves local network IPv4 (e.g. `192.168.144.110`).
  - `src/script.rs`: Generates PowerShell `run.ps1` with 4-layer sanitization (single-quote IP quoting, RFC 1123 hostname validation).
  - `src/state.rs`: `LanState` with thread-safe `RwLock<Option<ArtifactCache>>` and `is_file()` validation.

#### 3. `crates/airctrl-stt`
- **Purpose**: Offline speech-to-text transcription with zero cloud reliance.
- **Key Files**:
  - `src/engine/`: Whisper CPU inference engine.
  - `src/model/`: Local model discovery and streaming SHA-256 weight integrity verification.
  - `src/vad/`: RMS/dBFS energy voice activity detector with hangover state machine.
  - `src/audio/`: Circular FIFO ring buffer and 16kHz mono linear resampler.
  - `src/hotkey/`: `Ctrl+Spacebar` hotkey state machine supporting Push-to-Talk and Toggle with micro-press debounce (<100ms).
  - `src/injection/`: Active window text injection with clipboard paste fallback (>200 chars).

#### 4. `crates/airctrl-actions`
- **Purpose**: Voice command-to-action routing, window crawler, and multi-agent dispatch.
- **Key Files**:
  - `src/grammar.rs` & `src/fuzzy.rs`: Intent classification with Jaro-Winkler + Levenshtein acronym boosts.
  - `src/crawler/`: Process scanner and window accessibility tree crawler with active context caching.
  - `src/agent/`: Dedicated dispatch adapters for **Cursor, Claude, Antigravity, Grokbot, Dots, Muse, Viktor, Gemini**.
  - `src/rea/`: `morluto/rea` reverse-engineering hook for extracting application action schemas.
  - `src/hotkey/radial_dial.rs`: `RadialDialOverlay` state machine with dynamic `set_sector_count` bounds clamping.

#### 5. `crates/airctrl-gestures`
- **Purpose**: CPU-only Google MediaPipe hand landmark tracking and gesture heuristics.
- **Key Files**:
  - `src/engine/`: 21 3D hand landmark topology (`SKELETON_CONNECTIONS`) running at $\ge 30$ FPS and $\ge 60$ FPS on CPU without GPU.
  - `src/classifier/`: Geometric heuristics for index pointing rays, pinch distance ($d \le 0.05$ / hysteresis $d > 0.07$), and velocity swipe cooldowns.
  - `src/spatial/`: Transforms Cartesian hand coordinates into polar radial dial coordinates ($\theta = \text{atan2}(dy, dx)$).

#### 6. `crates/airctrl-meetings`
- **Purpose**: Meeting detection, camera privacy auto-kill, and legal compliance audio injection.
- **Key Files**:
  - `src/detector/signatures.rs`: Process signatures and window titles for Zoom, Teams, Meet, Webex, Discord. Enforces exact executable stem matching (`zoom.exe`, `teams.exe`) and confirmed WebRTC room paths to reject decoy binaries (`zoom_updater.exe`, `not_teams.exe`).
  - `src/privacy/kill_switch.rs`: Thread-safe atomic CAS `CameraKillSwitch` releasing camera handles in $<1.0$ms (50-thread race safe).
  - `src/compliance/`: Compliance audio generator and sink rendering *"This meeting is being recorded"* at $\ge 0.50$ statutory volume floor.

#### 7. `src-tauri`
- **Purpose**: Desktop window orchestration and headless IPC bridge.
- **Key Files**:
  - `src/ipc.rs`: Pure-Rust headless `IpcBridge` executing asynchronous commands without requiring an X11/Wayland display server during testing.
  - `src/state.rs`: Unified multi-crate state manager. **Zero-Mock Policy**: Drains live PCM sample buffers directly into `stt_engine.transcribe(&samples)` without mock string fallbacks.

---

## 3. Strict Engineering Guardrails

When writing code in this repository, you **MUST** follow these rules:

### 1. Zero-Mock Policy in Production Paths
- Never introduce dummy or fallback mock strings in production IPC handlers (e.g. `src-tauri/src/state.rs`). Production code must pipe real sample buffers into real engines.
- Test-only mocks belong exclusively under `tests/` or mock platform traits in `airctrl-core/src/platform/mock.rs`.

### 2. Headless In-Process IPC Bridge
- When adding new Tauri commands, always update `src-tauri/src/ipc.rs` and `src-tauri/tests/ipc_bridge_test.rs` so that automated CI and agents can verify commands in headless environments without X11/Wayland display servers.

### 3. Precision Heuristics in Meeting Detection
- Never match broad single-token keywords like `"zoom"` or `"teams"`.
- Always require:
  1. Exact executable stem matching (strip `.exe` and verify base binary).
  2. Multi-token contextual window titles (`"zoom meeting"`, `"teams call"`).
  3. Confirmed WebRTC meeting URL paths for browsers.

### 4. Concurrency & Memory Safety
- Use atomic CAS (`AtomicU8` / `AtomicBool`) for sub-millisecond hardware kill-switches.
- Use `RwLock<Option<T>>` for cached states that are read frequently and written rarely.
- Use decoupled bounded broadcast channels on the `EventBus`.

### 5. Conventional Commits
All git commits must strictly follow Conventional Commits:
```text
<type>(<scope>): <description>

Types: feat, fix, docs, style, refactor, perf, test, chore
Scopes: core, lan, stt, actions, gestures, meetings, tauri, ui
```

---

## 4. Agent Verification Runbook

Before claiming any task complete, you **MUST** run the full verification suite:

```bash
# 1. Run all workspace tests (417+ tests across 75 test suites)
cargo test --workspace

# 2. Check strict clippy rules (0 warnings allowed)
cargo clippy --workspace --all-targets -- -D warnings

# 3. Run the 299-case multi-tier Python end-to-end test suite
python3 tests/e2e/test_runner.py

# 4. Verify LAN distribution daemon & PowerShell script generation
./scripts/test-lan-distribution.sh

# 5. Check blast radius with Ripwire before modifying shared symbols
ripwire --blast-radius="<SymbolName>"
```
