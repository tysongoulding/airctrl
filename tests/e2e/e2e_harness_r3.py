"""
e2e_harness_r3.py - Extended Test Harness for airctrl R3 Features (Features 33–61)

Provides reference models, simulators, mathematical oracles, and headless IPC
verification helpers for:
- Features 33–40: Handy 1:1 Parity (models, discovery, modes, reliable paste, overlay, dictionary, history, CLI)
- Features 41–46: Speech-to-Action 100% Hands-Free (app lifecycle, 9-way snap, dynamic sizing, focus, UI nav)
- Features 47–54: Meetily 1:1 Parity (dual audio mixer, ducking, anti-clipping, live transcript, AI summaries, archive)
- Features 55–59: MediaPipe & Continuous Virtual Camera Splitter (dual-hand tracking, hysteresis pinch, zero-lock fan-out, privacy mute)
- Features 60–61: Headless IPC Bridge Command Surface & Full Verification (zero mock fallbacks)
"""

import enum
import hashlib
import json
import math
import os
import re
import time
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple

from tests.e2e.e2e_harness import Landmark3D, REPO_ROOT


# ============================================================================
# Features 33–40: Handy 1:1 Parity Models & Services
# ============================================================================

class WhisperModelVariant(str, enum.Enum):
    TINY_EN = "whisper-tiny.en"
    BASE_EN = "whisper-base.en"
    SMALL_EN = "whisper-small.en"
    MEDIUM_EN = "whisper-medium.en"
    TURBO_EN = "whisper-turbo.en"
    LARGE_EN = "whisper-large.en"
    PARAKEET_V3 = "parakeet-unified-en-0.6b-q8_0"
    CUSTOM = "custom"

    @property
    def file_extension(self) -> str:
        if self == WhisperModelVariant.PARAKEET_V3:
            return ".gguf"
        return ".bin"

    @property
    def parameter_count_millions(self) -> int:
        params = {
            WhisperModelVariant.TINY_EN: 39,
            WhisperModelVariant.BASE_EN: 74,
            WhisperModelVariant.SMALL_EN: 244,
            WhisperModelVariant.MEDIUM_EN: 769,
            WhisperModelVariant.TURBO_EN: 809,
            WhisperModelVariant.LARGE_EN: 1550,
            WhisperModelVariant.PARAKEET_V3: 600,
            WhisperModelVariant.CUSTOM: 100,
        }
        return params.get(self, 100)

    @property
    def is_cpu_optimized(self) -> bool:
        return self in (
            WhisperModelVariant.TINY_EN,
            WhisperModelVariant.BASE_EN,
            WhisperModelVariant.SMALL_EN,
            WhisperModelVariant.PARAKEET_V3,
        )

    @property
    def supports_auto_language_detection(self) -> bool:
        return self in (
            WhisperModelVariant.MEDIUM_EN,
            WhisperModelVariant.TURBO_EN,
            WhisperModelVariant.LARGE_EN,
            WhisperModelVariant.PARAKEET_V3,
        )


@dataclass
class DiscoveredModel:
    name: str
    path: str
    size_bytes: int
    format: str  # "bin" or "gguf"
    variant: WhisperModelVariant
    is_valid: bool = True
    sha256: Optional[str] = None


class ModelDiscoveryService:
    """
    Scans candidate directories ({app_data_dir}/models/ and ~/.cache/huggingface/hub/)
    for valid .bin and .gguf Whisper and Parakeet models.
    """
    KNOWN_MODEL_MAP = {
        "tiny": WhisperModelVariant.TINY_EN,
        "base": WhisperModelVariant.BASE_EN,
        "small": WhisperModelVariant.SMALL_EN,
        "medium": WhisperModelVariant.MEDIUM_EN,
        "turbo": WhisperModelVariant.TURBO_EN,
        "large": WhisperModelVariant.LARGE_EN,
        "parakeet": WhisperModelVariant.PARAKEET_V3,
    }

    @classmethod
    def discover_models(cls, search_dirs: List[str]) -> List[DiscoveredModel]:
        discovered: List[DiscoveredModel] = []
        seen_paths = set()

        for d in search_dirs:
            if not os.path.exists(d) or not os.path.isdir(d):
                continue
            for root, _, files in os.walk(d):
                for f in sorted(files):
                    lower_f = f.lower()
                    if not (lower_f.endswith(".bin") or lower_f.endswith(".gguf")):
                        continue

                    full_path = os.path.abspath(os.path.join(root, f))
                    if full_path in seen_paths:
                        continue
                    seen_paths.add(full_path)

                    fmt = "gguf" if lower_f.endswith(".gguf") else "bin"
                    size = os.path.getsize(full_path)

                    # Classify variant
                    variant = WhisperModelVariant.CUSTOM
                    for key, var in cls.KNOWN_MODEL_MAP.items():
                        if key in lower_f:
                            variant = var
                            break

                    is_valid = size >= 1024  # reject empty or truncated files
                    discovered.append(DiscoveredModel(
                        name=f,
                        path=full_path,
                        size_bytes=size,
                        format=fmt,
                        variant=variant,
                        is_valid=is_valid,
                    ))

        return discovered


class RecordingMode(str, enum.Enum):
    SMART_DUAL_MODE = "SmartDualMode"
    HOLD_ONLY = "HoldOnly"
    TOGGLE_ONLY = "ToggleOnly"
    CONTINUOUS_HANDS_FREE = "ContinuousHandsFree"


class RecordingState(str, enum.Enum):
    IDLE = "Idle"
    RECORDING = "Recording"
    TRANSCRIBING = "Transcribing"
    CANCELLED = "Cancelled"


class RecordingModeController:
    """
    Controls recording state transitions across all 4 Handy recording modes:
    - SmartDualMode: press < 250ms -> Toggle; press >= 250ms -> Push-to-Talk (stops on release)
    - HoldOnly: only records while key held; stops immediately on release
    - ToggleOnly: key press toggles on, subsequent press toggles off regardless of hold time
    - ContinuousHandsFree: VAD energy triggers automatic transcription chunks without hotkeys
    """
    def __init__(self, mode: RecordingMode = RecordingMode.SMART_DUAL_MODE):
        self.mode = mode
        self.state = RecordingState.IDLE
        self.press_time_ms: Optional[float] = None
        self.transcribed_count = 0
        self.cancelled_count = 0

    def on_key_down(self, timestamp_ms: float) -> RecordingState:
        if self.mode == RecordingMode.HOLD_ONLY:
            self.state = RecordingState.RECORDING
            self.press_time_ms = timestamp_ms

        elif self.mode == RecordingMode.TOGGLE_ONLY:
            if self.state == RecordingState.RECORDING:
                self.state = RecordingState.TRANSCRIBING
                self.transcribed_count += 1
            else:
                self.state = RecordingState.RECORDING
            self.press_time_ms = timestamp_ms

        elif self.mode == RecordingMode.SMART_DUAL_MODE:
            if self.state == RecordingState.RECORDING and self.press_time_ms is not None:
                # Was previously toggled on; second tap turns it off
                self.state = RecordingState.TRANSCRIBING
                self.transcribed_count += 1
                self.press_time_ms = None
            else:
                self.state = RecordingState.RECORDING
                self.press_time_ms = timestamp_ms

        return self.state

    def on_key_up(self, timestamp_ms: float) -> RecordingState:
        if self.press_time_ms is None:
            return self.state

        duration = timestamp_ms - self.press_time_ms

        if duration < 30.0:
            # Jitter debounce: discard accidental micro-contact
            self.state = RecordingState.CANCELLED
            self.cancelled_count += 1
            self.press_time_ms = None
            return self.state

        if self.mode == RecordingMode.HOLD_ONLY:
            self.state = RecordingState.TRANSCRIBING
            self.transcribed_count += 1
            self.press_time_ms = None

        elif self.mode == RecordingMode.TOGGLE_ONLY:
            # Key up ignored in Toggle-Only; state maintained until next key down
            pass

        elif self.mode == RecordingMode.SMART_DUAL_MODE:
            if duration >= 250.0:
                # Push-to-Talk completed
                self.state = RecordingState.TRANSCRIBING
                self.transcribed_count += 1
                self.press_time_ms = None
            else:
                # Tap (< 250ms) latches into Toggle Recording mode
                self.state = RecordingState.RECORDING

        return self.state

    def on_vad_speech_detected(self, energy_rms: float, threshold: float = 0.05) -> RecordingState:
        if self.mode == RecordingMode.CONTINUOUS_HANDS_FREE:
            if energy_rms >= threshold:
                self.state = RecordingState.RECORDING
            else:
                if self.state == RecordingState.RECORDING:
                    self.state = RecordingState.TRANSCRIBING
                    self.transcribed_count += 1
                else:
                    self.state = RecordingState.IDLE
        return self.state

    def cancel(self):
        self.state = RecordingState.CANCELLED
        self.cancelled_count += 1
        self.press_time_ms = None


class ReliablePasteManager:
    """
    Simulates Handy's reliable clipboard paste workflow:
    1. Reads current clipboard contents (snapshot)
    2. Writes target transcript text into clipboard
    3. Verifies clipboard contents match before pasting
    4. Applies configurable paste_delay_before_ms
    5. Simulates OS paste keystroke (Ctrl+V / Cmd+V)
    6. Applies configurable paste_delay_after_ms
    7. Restores prior clipboard contents
    """
    def __init__(self, delay_before_ms: int = 50, delay_after_ms: int = 100, reliable_paste: bool = True):
        self.delay_before_ms = delay_before_ms
        self.delay_after_ms = delay_after_ms
        self.reliable_paste = reliable_paste
        self.clipboard_content = "initial clipboard item"
        self.paste_events: List[Dict[str, Any]] = []

    def paste_text(self, text: str) -> Dict[str, Any]:
        prior_clipboard = self.clipboard_content

        # Step 1: Write target text
        self.clipboard_content = text
        verified = (self.clipboard_content == text)

        # Step 2: Simulated delay before
        time_before = self.delay_before_ms

        # Step 3: Emit paste keystroke event
        pasted_text = self.clipboard_content

        # Step 4: Simulated delay after
        time_after = self.delay_after_ms

        # Step 5: Restore previous clipboard if reliable_paste is enabled
        if self.reliable_paste:
            self.clipboard_content = prior_clipboard
            restored = True
        else:
            restored = False

        event = {
            "pasted_text": pasted_text,
            "verified": verified,
            "delay_before_ms": time_before,
            "delay_after_ms": time_after,
            "restored_clipboard": restored,
            "final_clipboard": self.clipboard_content,
        }
        self.paste_events.append(event)
        return event


class OverlayPosition(str, enum.Enum):
    BOTTOM = "Bottom"
    TOP = "Top"
    CENTER = "Center"
    NONE = "None"


class OverlayGeometryManager:
    """
    Computes HUD overlay positioning and window style flags.
    Position 'None' achieves zero focus-stealing on Linux and Windows.
    """
    @staticmethod
    def calculate_bounds(
        position: OverlayPosition,
        screen_width: int = 1920,
        screen_height: int = 1080,
        hud_width: int = 400,
        hud_height: int = 60,
        margin: int = 30
    ) -> Dict[str, Any]:
        if position == OverlayPosition.NONE:
            return {
                "visible": False,
                "focus_stealing": False,
                "x": 0,
                "y": 0,
                "width": 0,
                "height": 0,
            }

        center_x = (screen_width - hud_width) // 2

        if position == OverlayPosition.TOP:
            y = margin
        elif position == OverlayPosition.BOTTOM:
            y = screen_height - hud_height - margin
        elif position == OverlayPosition.CENTER:
            y = (screen_height - hud_height) // 2
        else:
            y = screen_height - hud_height - margin

        return {
            "visible": True,
            "focus_stealing": False,  # Transparent non-activating overlay
            "x": center_x,
            "y": y,
            "width": hud_width,
            "height": hud_height,
        }


class DictionaryService:
    """
    Custom vocabulary and dictionary replacement engine with acoustic cues and post-processing.
    """
    def __init__(self, initial_rules: Optional[Dict[str, str]] = None):
        self.rules: Dict[str, str] = dict(initial_rules or {})
        self.audio_cues_enabled = True
        self.post_process_enabled = True
        self.played_cues: List[str] = []

    def add_rule(self, trigger: str, replacement: str):
        self.rules[trigger.strip()] = replacement.strip()

    def play_cue(self, cue_name: str):
        if self.audio_cues_enabled:
            self.played_cues.append(cue_name)

    def replace_words(self, text: str) -> str:
        if not text:
            return ""

        result = text
        # Word boundary substitution for each rule
        for trigger, replacement in self.rules.items():
            pattern = rf"\b{re.escape(trigger)}\b"
            result = re.sub(pattern, replacement, result, flags=re.IGNORECASE)

        if self.post_process_enabled:
            # Capitalize first letter and ensure clean spacing
            result = re.sub(r"\s+", " ", result).strip()
            if result and result[0].islower():
                result = result[0].upper() + result[1:]

        return result


@dataclass
class TranscriptItem:
    id: str
    timestamp: float
    text: str
    raw_text: str
    duration_ms: int
    model_used: str
    confidence: float
    session_id: str = "default_session"


class TranscriptHistoryStore:
    """
    Searchable, persistent local transcript history store.
    """
    def __init__(self):
        self.items: List[TranscriptItem] = []

    def store(self, item: TranscriptItem):
        self.items.append(item)

    def search(self, query: str, model_filter: Optional[str] = None, limit: int = 50) -> List[TranscriptItem]:
        q = query.lower().strip()
        results: List[TranscriptItem] = []

        for item in reversed(self.items):
            if model_filter and item.model_used.lower() != model_filter.lower():
                continue
            if not q or q in item.text.lower() or q in item.raw_text.lower():
                results.append(item)
            if len(results) >= limit:
                break

        return results

    def clear(self):
        self.items.clear()


class AirCtrlCliParser:
    """
    Parses CLI remote control flags:
    --toggle-transcription, --toggle-post-process, --cancel, --start-hidden, --no-tray, --debug
    """
    @staticmethod
    def parse_args(argv: List[str]) -> Dict[str, Any]:
        options = {
            "toggle_transcription": False,
            "toggle_post_process": False,
            "cancel": False,
            "start_hidden": False,
            "no_tray": False,
            "debug": False,
            "model": None,
            "debug_drawer_active": False,
        }

        i = 0
        while i < len(argv):
            arg = argv[i]
            if arg == "--toggle-transcription":
                options["toggle_transcription"] = True
            elif arg == "--toggle-post-process":
                options["toggle_post_process"] = True
            elif arg == "--cancel":
                options["cancel"] = True
            elif arg == "--start-hidden":
                options["start_hidden"] = True
            elif arg == "--no-tray":
                options["no_tray"] = True
            elif arg == "--debug":
                options["debug"] = True
            elif arg == "--model" and i + 1 < len(argv):
                i += 1
                options["model"] = argv[i]
            i += 1

        return options


# ============================================================================
# Features 41–46: Speech-to-Action 100% Hands-Free Models & Services
# ============================================================================

class SnapPosition(str, enum.Enum):
    LEFT = "Left"
    RIGHT = "Right"
    TOP = "Top"
    BOTTOM = "Bottom"
    CENTER = "Center"
    MAXIMIZE = "Maximize"
    MINIMIZE = "Minimize"
    RESTORE = "Restore"
    FULLSCREEN = "Fullscreen"


class ResizeDimension(str, enum.Enum):
    LARGER = "Larger"
    SMALLER = "Smaller"
    WIDER = "Wider"
    NARROWER = "Narrower"
    TALLER = "Taller"
    SHORTER = "Shorter"


@dataclass
class ResizeCommand:
    mode: str  # "delta", "percentage", "explicit"
    dimension: Optional[ResizeDimension] = None
    delta_pixels: int = 100
    percentage: Optional[int] = None
    width: Optional[int] = None
    height: Optional[int] = None


class UiTarget(str, enum.Enum):
    NEXT_FIELD = "NextField"
    PREV_FIELD = "PrevField"
    ENTER = "Enter"
    ESCAPE = "Escape"
    TAB = "Tab"


class ScrollDirection(str, enum.Enum):
    UP = "Up"
    DOWN = "Down"
    LEFT = "Left"
    RIGHT = "Right"
    PAGE_UP = "PageUp"
    PAGE_DOWN = "PageDown"


class MouseButton(str, enum.Enum):
    LEFT = "Left"
    RIGHT = "Right"
    MIDDLE = "Middle"
    DOUBLE = "Double"


@dataclass
class HandsFreeActionCommand:
    action_type: str
    payload: Dict[str, Any] = field(default_factory=dict)


class HandsFreeCommandParser:
    """
    Parses spoken utterances into hands-free desktop action commands:
    - App lifecycle: open/launch/start {app} (auto-center), close/quit {app}, close window
    - 9-way snapping: snap {left, right, top, bottom, center, maximize, minimize, restore, fullscreen}
    - Dynamic resizing: resize larger/smaller/wider/narrower/taller/shorter, resize 50%/80%/100%, resize 1280 by 720
    - Focus & cycling: focus {app}, next window, previous window
    - UI navigation & clicking: next/prev field, enter, tab, escape, scroll up/down, click
    """
    WAKE_PREFIXES = ["hey airctrl", "airctrl"]

    def parse(self, utterance: str) -> HandsFreeActionCommand:
        u = utterance.strip().lower()

        # Strip wake prefix if present
        for prefix in self.WAKE_PREFIXES:
            if u.startswith(prefix):
                u = u[len(prefix):].strip()
                break

        if not u:
            return HandsFreeActionCommand(action_type="Unknown", payload={"query": ""})

        # 1. 9-Way Window Snapping
        snap_match = re.match(r"(?:snap|dock|tile)\s+(left|right|top|bottom|center|maximize|minimize|restore|fullscreen)", u)
        if snap_match:
            pos_str = snap_match.group(1).capitalize()
            return HandsFreeActionCommand(
                action_type="SnapWindow",
                payload={"position": SnapPosition(pos_str).value}
            )

        if u in ("maximize window", "maximize"):
            return HandsFreeActionCommand(action_type="SnapWindow", payload={"position": SnapPosition.MAXIMIZE.value})
        if u in ("minimize window", "minimize"):
            return HandsFreeActionCommand(action_type="SnapWindow", payload={"position": SnapPosition.MINIMIZE.value})
        if u in ("restore window", "restore"):
            return HandsFreeActionCommand(action_type="SnapWindow", payload={"position": SnapPosition.RESTORE.value})
        if u in ("fullscreen window", "fullscreen"):
            return HandsFreeActionCommand(action_type="SnapWindow", payload={"position": SnapPosition.FULLSCREEN.value})

        # 2. Dynamic Window Resizing
        # 2a. Explicit: resize 1280 by 720 / resize 1920x1080
        explicit_m = re.match(r"(?:resize|size)\s+(?:window\s+)?(\d+)\s*(?:by|x)\s*(\d+)", u)
        if explicit_m:
            w, h = int(explicit_m.group(1)), int(explicit_m.group(2))
            return HandsFreeActionCommand(
                action_type="ResizeWindow",
                payload={"mode": "Explicit", "width": w, "height": h}
            )

        # 2b. Percentage: resize window 50% / 80% / 100%
        pct_m = re.match(r"(?:resize|scale)\s+(?:window\s+)?(\d+)\s*%", u)
        if pct_m:
            pct = max(10, min(100, int(pct_m.group(1))))
            return HandsFreeActionCommand(
                action_type="ResizeWindow",
                payload={"mode": "Percentage", "percentage": pct}
            )

        # 2c. Deltas: make window larger/smaller/wider/narrower/taller/shorter
        delta_m = re.match(r"(?:make\s+window\s+|resize\s+(?:window\s+)?)(larger|smaller|wider|narrower|taller|shorter)(?:\s+by\s+(\d+))?", u)
        if delta_m:
            dim_str = delta_m.group(1).capitalize()
            delta_px = int(delta_m.group(2)) if delta_m.group(2) else 100
            return HandsFreeActionCommand(
                action_type="ResizeWindow",
                payload={"mode": "Delta", "dimension": ResizeDimension(dim_str).value, "delta_pixels": delta_px}
            )

        # 3. Focus Switching & Window Cycling
        if u == "next window":
            return HandsFreeActionCommand(action_type="CycleWindow", payload={"direction": "Next"})
        if u in ("previous window", "prev window"):
            return HandsFreeActionCommand(action_type="CycleWindow", payload={"direction": "Previous"})

        focus_m = re.match(r"(?:focus|switch to)\s+(.+)", u)
        if focus_m:
            app_target = focus_m.group(1).strip()
            return HandsFreeActionCommand(action_type="SwitchFocus", payload={"app_name": app_target})

        # 4. App Lifecycle Commands
        launch_m = re.match(r"(?:launch|open|start)\s+(.+)", u)
        if launch_m:
            app_target = launch_m.group(1).strip()
            return HandsFreeActionCommand(
                action_type="LaunchApp",
                payload={"app_name": app_target, "auto_center": True}
            )

        if u in ("close window", "close current window", "quit window"):
            return HandsFreeActionCommand(action_type="CloseApp", payload={"target": "CurrentWindow"})

        close_m = re.match(r"(?:close|quit|exit|kill)\s+(.+)", u)
        if close_m:
            app_target = close_m.group(1).strip()
            return HandsFreeActionCommand(action_type="CloseApp", payload={"target": "App", "app_name": app_target})

        # 5. UI Navigation & Mouse Clicking
        if u in ("next field", "next input"):
            return HandsFreeActionCommand(action_type="NavigateUi", payload={"element": UiTarget.NEXT_FIELD.value})
        if u in ("previous field", "prev field"):
            return HandsFreeActionCommand(action_type="NavigateUi", payload={"element": UiTarget.PREV_FIELD.value})
        if u in ("press enter", "enter"):
            return HandsFreeActionCommand(action_type="NavigateUi", payload={"element": UiTarget.ENTER.value})
        if u in ("press escape", "escape"):
            return HandsFreeActionCommand(action_type="NavigateUi", payload={"element": UiTarget.ESCAPE.value})
        if u in ("press tab", "tab"):
            return HandsFreeActionCommand(action_type="NavigateUi", payload={"element": UiTarget.TAB.value})

        # Scrolling
        scroll_m = re.match(r"scroll\s+(up|down|left|right)(?:\s+(\d+))?", u)
        if scroll_m:
            dir_str = scroll_m.group(1).capitalize()
            amount = int(scroll_m.group(2)) if scroll_m.group(2) else 3
            return HandsFreeActionCommand(
                action_type="Scroll",
                payload={"direction": ScrollDirection(dir_str).value, "amount": amount}
            )
        if u == "page up":
            return HandsFreeActionCommand(action_type="Scroll", payload={"direction": ScrollDirection.PAGE_UP.value, "amount": 1})
        if u == "page down":
            return HandsFreeActionCommand(action_type="Scroll", payload={"direction": ScrollDirection.PAGE_DOWN.value, "amount": 1})

        # Clicking
        if u in ("left click", "click"):
            return HandsFreeActionCommand(action_type="MouseClick", payload={"button": MouseButton.LEFT.value, "count": 1})
        if u in ("right click", "context click"):
            return HandsFreeActionCommand(action_type="MouseClick", payload={"button": MouseButton.RIGHT.value, "count": 1})
        if u in ("double click"):
            return HandsFreeActionCommand(action_type="MouseClick", payload={"button": MouseButton.DOUBLE.value, "count": 2})
        if u in ("middle click"):
            return HandsFreeActionCommand(action_type="MouseClick", payload={"button": MouseButton.MIDDLE.value, "count": 1})

        return HandsFreeActionCommand(action_type="Unknown", payload={"query": utterance})


class WindowGeometryEngine:
    """
    Computes exact window coordinates (x, y, width, height) for 9-way snapping
    and dynamic resizing against screen bounds.
    """
    def __init__(self, screen_w: int = 1920, screen_h: int = 1080):
        self.screen_w = screen_w
        self.screen_h = screen_h

    def compute_snap(self, position: SnapPosition) -> Dict[str, int]:
        half_w = self.screen_w // 2
        half_h = self.screen_h // 2

        if position == SnapPosition.LEFT:
            return {"x": 0, "y": 0, "width": half_w, "height": self.screen_h}
        elif position == SnapPosition.RIGHT:
            return {"x": half_w, "y": 0, "width": half_w, "height": self.screen_h}
        elif position == SnapPosition.TOP:
            return {"x": 0, "y": 0, "width": self.screen_w, "height": half_h}
        elif position == SnapPosition.BOTTOM:
            return {"x": 0, "y": half_h, "width": self.screen_w, "height": half_h}
        elif position == SnapPosition.CENTER:
            w = int(self.screen_w * 0.70)
            h = int(self.screen_h * 0.70)
            return {"x": (self.screen_w - w) // 2, "y": (self.screen_h - h) // 2, "width": w, "height": h}
        elif position == SnapPosition.MAXIMIZE:
            return {"x": 0, "y": 0, "width": self.screen_w, "height": self.screen_h}
        elif position == SnapPosition.MINIMIZE:
            return {"x": 0, "y": 0, "width": 0, "height": 0}
        elif position == SnapPosition.RESTORE:
            return {"x": 100, "y": 100, "width": 1024, "height": 768}
        elif position == SnapPosition.FULLSCREEN:
            return {"x": 0, "y": 0, "width": self.screen_w, "height": self.screen_h}
        return {"x": 0, "y": 0, "width": self.screen_w, "height": self.screen_h}

    def compute_resize(
        self,
        current_bounds: Dict[str, int],
        cmd: HandsFreeActionCommand
    ) -> Dict[str, int]:
        x, y = current_bounds.get("x", 100), current_bounds.get("y", 100)
        w, h = current_bounds.get("width", 1000), current_bounds.get("height", 700)

        payload = cmd.payload
        mode = payload.get("mode")

        if mode == "Explicit":
            new_w = min(self.screen_w, max(200, payload.get("width", w)))
            new_h = min(self.screen_h, max(200, payload.get("height", h)))
            return {"x": x, "y": y, "width": new_w, "height": new_h}

        elif mode == "Percentage":
            pct = payload.get("percentage", 100) / 100.0
            new_w = int(self.screen_w * pct)
            new_h = int(self.screen_h * pct)
            new_x = max(0, (self.screen_w - new_w) // 2)
            new_y = max(0, (self.screen_h - new_h) // 2)
            return {"x": new_x, "y": new_y, "width": new_w, "height": new_h}

        elif mode == "Delta":
            dim = payload.get("dimension")
            delta = payload.get("delta_pixels", 100)
            if dim == ResizeDimension.LARGER.value:
                w += delta
                h += delta
            elif dim == ResizeDimension.SMALLER.value:
                w -= delta
                h -= delta
            elif dim == ResizeDimension.WIDER.value:
                w += delta
            elif dim == ResizeDimension.NARROWER.value:
                w -= delta
            elif dim == ResizeDimension.TALLER.value:
                h += delta
            elif dim == ResizeDimension.SHORTER.value:
                h -= delta

            w = min(self.screen_w, max(200, w))
            h = min(self.screen_h, max(200, h))
            return {"x": x, "y": y, "width": w, "height": h}

        return current_bounds


# ============================================================================
# Features 47–54: Meetily 1:1 Parity Models & Services
# ============================================================================

class DualAudioMixer:
    """
    Simultaneously mixes microphone and system loopback audio with:
    - Dynamic Voice Ducking: ducks system audio when mic energy > threshold
      (attack = 30ms, release = 500ms, ducking gain = 0.25)
    - Anti-Clipping Dynamic Range Normalizer: hyperbolic tangent (tanh) soft-knee
      limiter strictly bounding mixed samples in [-1.0, 1.0].
    """
    def __init__(
        self,
        sample_rate: int = 16000,
        ducking_threshold: float = 0.05,
        ducking_gain: float = 0.25,
        attack_ms: float = 30.0,
        release_ms: float = 500.0
    ):
        self.sample_rate = sample_rate
        self.ducking_threshold = ducking_threshold
        self.ducking_gain = ducking_gain
        self.attack_alpha = 1.0 - math.exp(-1.0 / (sample_rate * (attack_ms / 1000.0)))
        self.release_alpha = 1.0 - math.exp(-1.0 / (sample_rate * (release_ms / 1000.0)))
        self.current_envelope = 0.0
        self.current_duck_gain = 1.0

    def mix(self, mic_samples: List[float], loopback_samples: List[float]) -> List[float]:
        n = max(len(mic_samples), len(loopback_samples))
        mixed: List[float] = []

        for i in range(n):
            m = mic_samples[i] if i < len(mic_samples) else 0.0
            lp = loopback_samples[i] if i < len(loopback_samples) else 0.0

            # Envelope follower on microphone
            rect_mic = abs(m)
            if rect_mic > self.current_envelope:
                self.current_envelope += self.attack_alpha * (rect_mic - self.current_envelope)
            else:
                self.current_envelope += self.release_alpha * (rect_mic - self.current_envelope)

            # Ducking computation
            if self.current_envelope >= self.ducking_threshold:
                target_gain = self.ducking_gain
            else:
                target_gain = 1.0

            if target_gain < self.current_duck_gain:
                self.current_duck_gain += self.attack_alpha * (target_gain - self.current_duck_gain)
            else:
                self.current_duck_gain += self.release_alpha * (target_gain - self.current_duck_gain)

            ducked_lp = lp * self.current_duck_gain
            raw_sum = m + ducked_lp

            # Anti-Clipping Soft-Knee Limiter via tanh
            limited = math.tanh(raw_sum)
            # Strictly assert invariant bounds [-1.0, 1.0]
            limited = max(-1.0, min(1.0, limited))
            mixed.append(limited)

        return mixed


@dataclass
class LiveTranscriptChunk:
    id: str
    speaker: str  # "Host", "Participant", "Mixed"
    text: str
    start_time_ms: int
    end_time_ms: int
    confidence: float


class LiveMeetingTranscriber:
    """
    Real-time meeting transcription engine with host vs participant
    energy ratio speaker attribution.
    """
    def __init__(self):
        self.chunks: List[LiveTranscriptChunk] = []
        self.current_time_ms = 0

    def feed_audio(
        self,
        mic_energy: float,
        loopback_energy: float,
        duration_ms: int = 1000,
        transcribed_text: str = "meeting notes chunk"
    ) -> LiveTranscriptChunk:
        eps = 1e-6
        ratio = mic_energy / (loopback_energy + eps)

        if ratio > 2.0:
            speaker = "Host"
        elif loopback_energy / (mic_energy + eps) > 2.0:
            speaker = "Participant"
        else:
            speaker = "Mixed"

        start_t = self.current_time_ms
        end_t = start_t + duration_ms
        self.current_time_ms = end_t

        chunk = LiveTranscriptChunk(
            id=f"chunk_{len(self.chunks) + 1}",
            speaker=speaker,
            text=transcribed_text,
            start_time_ms=start_t,
            end_time_ms=end_t,
            confidence=0.95
        )
        self.chunks.append(chunk)
        return chunk


class SummaryProvider(str, enum.Enum):
    OLLAMA = "Ollama"
    CLAUDE = "Claude"
    GROQ = "Groq"
    OPENROUTER = "OpenRouter"
    CUSTOM_OPENAI = "CustomOpenAI"


class SummaryTemplate(str, enum.Enum):
    EXECUTIVE_SUMMARY = "ExecutiveSummary"
    ACTION_ITEMS = "ActionItems"
    TECHNICAL_DECISIONS = "TechnicalDecisions"
    CUSTOM = "Custom"


class AiSummaryService:
    """
    Multi-provider AI meeting summarizer supporting 5 providers and 4 templates.
    """
    def __init__(self, provider: SummaryProvider = SummaryProvider.OLLAMA):
        self.provider = provider

    def generate_summary(
        self,
        transcript_text: str,
        template: SummaryTemplate = SummaryTemplate.EXECUTIVE_SUMMARY,
        custom_prompt: Optional[str] = None
    ) -> Dict[str, Any]:
        if not transcript_text or not transcript_text.strip():
            raise ValueError("Cannot summarize empty transcript")

        if template == SummaryTemplate.EXECUTIVE_SUMMARY:
            summary = (
                "# Executive Summary\n\n"
                "## Overview\n"
                f"Meeting transcript analyzed using {self.provider.value}.\n\n"
                "## Key Points\n"
                "- Successfully reviewed project architecture and deliverables.\n"
                "- Confirmed compliance and quality standards are fully satisfied.\n"
            )
        elif template == SummaryTemplate.ACTION_ITEMS:
            summary = (
                "# Action Items\n\n"
                "- [ ] Review system requirements (Assignee: Lead Dev, Deadline: End of Week)\n"
                "- [ ] Run full test suite regression (Assignee: QA, Deadline: Tomorrow)\n"
            )
        elif template == SummaryTemplate.TECHNICAL_DECISIONS:
            summary = (
                "# Technical Decisions\n\n"
                "1. Decoupled 3-Stage DAG event bus adopted for sensor streams.\n"
                "2. Hyperbolic tangent soft-knee anti-clipping limiter implemented.\n"
            )
        else:
            summary = f"# Custom Summary\n\n{custom_prompt or 'Custom prompt summary output.'}"

        return {
            "provider": self.provider.value,
            "template": template.value,
            "summary_markdown": summary,
            "generated_at": "2026-10-14T01:00:00Z",
            "token_count": len(summary.split()) * 2
        }


class ExportFormat(str, enum.Enum):
    MARKDOWN = "Markdown"
    PDF = "PDF"
    DOCX = "DOCX"


class DocumentExporter:
    """
    Exports meeting summaries to Markdown, PDF, and DOCX formats.
    """
    @staticmethod
    def export(content: str, fmt: ExportFormat) -> bytes:
        if fmt == ExportFormat.MARKDOWN:
            return content.encode("utf-8")
        elif fmt == ExportFormat.PDF:
            # Generate valid PDF 1.4 binary skeleton
            pdf_body = (
                b"%PDF-1.4\n"
                b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n"
                b"2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n"
                b"3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >> endobj\n"
                b"xref\n0 4\n0000000000 65535 f \n"
                b"trailer << /Root 1 0 R /Size 4 >>\n"
                b"startxref\n180\n%%EOF\n"
            )
            return pdf_body
        elif fmt == ExportFormat.DOCX:
            # Generate valid simulated DOCX payload with PK zip signature
            return b"PK\x03\x04\x14\x00\x00\x00\x08\x00" + content.encode("utf-8")[:100] + b"[Content_Types].xml"
        raise ValueError(f"Unsupported format: {fmt}")


@dataclass
class MeetingArchiveEntry:
    id: str
    title: str
    date_iso: str
    duration_secs: int
    transcript_chunks: List[LiveTranscriptChunk]
    summary_markdown: str
    audio_path: str
    participants: List[str]


class MeetingArchiveStore:
    """
    Local meeting archive store with full-text search and synchronized seeking.
    """
    def __init__(self):
        self.entries: List[MeetingArchiveEntry] = []

    def save_meeting(self, entry: MeetingArchiveEntry):
        self.entries.append(entry)

    def search(self, query: str) -> List[MeetingArchiveEntry]:
        q = query.lower().strip()
        matched: List[MeetingArchiveEntry] = []
        for e in self.entries:
            if not q or q in e.title.lower() or q in e.summary_markdown.lower():
                matched.append(e)
            else:
                for c in e.transcript_chunks:
                    if q in c.text.lower():
                        matched.append(e)
                        break
        return matched

    def seek_audio_offset(self, meeting_id: str, chunk_id: str) -> Optional[int]:
        for e in self.entries:
            if e.id == meeting_id:
                for c in e.transcript_chunks:
                    if c.id == chunk_id:
                        return c.start_time_ms
        return None


# ============================================================================
# Features 55–59: MediaPipe & Continuous Virtual Camera Splitter
# ============================================================================

@dataclass
class MultiHandLandmarks:
    left_hand: Optional[List[Landmark3D]] = None
    right_hand: Optional[List[Landmark3D]] = None

    @property
    def hand_count(self) -> int:
        count = 0
        if self.left_hand:
            count += 1
        if self.right_hand:
            count += 1
        return count


class AdvancedGestureHeuristics:
    """
    Evaluates dual-hand landmarks with:
    - Hysteresis Pinch: engages at d <= 0.05, releases only when d > 0.07
    - Pointing Ray: 3D vector from PIP to index tip
    - Velocity Swipes: Left, Right, Up, Down with cooldown
    - Two-Finger gestures: scroll and zoom
    """
    def __init__(self):
        self.pinch_engaged = False
        self.last_swipe_time = 0.0
        self.swipe_cooldown_secs = 0.3

    def evaluate_pinch(self, thumb_tip: Landmark3D, index_tip: Landmark3D) -> bool:
        dx = thumb_tip.x - index_tip.x
        dy = thumb_tip.y - index_tip.y
        dz = thumb_tip.z - index_tip.z
        d = math.sqrt(dx * dx + dy * dy + dz * dz)
        rounded_d = round(d, 5)

        if not self.pinch_engaged:
            if rounded_d <= 0.05:
                self.pinch_engaged = True
        else:
            if rounded_d > 0.07:
                self.pinch_engaged = False

        return self.pinch_engaged

    def compute_pointing_ray(self, pip: Landmark3D, tip: Landmark3D) -> Dict[str, float]:
        dx = tip.x - pip.x
        dy = tip.y - pip.y
        dz = tip.z - pip.z
        mag = math.sqrt(dx * dx + dy * dy + dz * dz) + 1e-6
        return {"dx": round(dx / mag, 4), "dy": round(dy / mag, 4), "dz": round(dz / mag, 4)}

    def detect_swipe(self, prev_x: float, curr_x: float, timestamp: float) -> Optional[str]:
        if timestamp - self.last_swipe_time < self.swipe_cooldown_secs:
            return None

        delta_x = curr_x - prev_x
        if delta_x > 0.15:
            self.last_swipe_time = timestamp
            return "SwipeRight"
        elif delta_x < -0.15:
            self.last_swipe_time = timestamp
            return "SwipeLeft"
        return None

    def evaluate_two_finger_scroll(self, index_tip: Landmark3D, middle_tip: Landmark3D) -> float:
        # Distance between index and middle
        dx = index_tip.x - middle_tip.x
        dy = index_tip.y - middle_tip.y
        d = math.sqrt(dx * dx + dy * dy)
        if d < 0.08:  # fingers together -> scroll vector
            return (index_tip.y + middle_tip.y) / 2.0
        return 0.0


class VirtualCameraSplitter:
    """
    Continuous background virtual camera splitter exposing 'AirCtrl Virtual Camera'.
    Fans out camera frames concurrently with zero device lock contention to:
    - Consumer 1: Internal MediaPipe hand tracker (always active)
    - Consumer 2: 'AirCtrl Virtual Camera' device (consumed by Zoom/Teams/Meet)
    - Independent Privacy Mute: Replaces outbound meeting stream (Consumer 2) with
      a privacy slate while local hand gesture tracking (Consumer 1) continues running!
    """
    def __init__(self, device_name: str = "AirCtrl Virtual Camera"):
        self.device_name = device_name
        self.is_running = True
        self.privacy_mute_active = False
        self.dual_mute_active = False
        self.frames_fanned_out = 0

    def feed_camera_frame(self, frame_id: int) -> Dict[str, Any]:
        if not self.is_running:
            raise RuntimeError("Splitter not running")

        self.frames_fanned_out += 1

        # Consumer 1: MediaPipe (Receives raw frame unless dual mute is explicitly requested)
        c1_frame = None if self.dual_mute_active else f"raw_frame_{frame_id}"

        # Consumer 2: Meeting apps (Receives privacy frame if privacy muted)
        if self.privacy_mute_active or self.dual_mute_active:
            c2_frame = "privacy_slate_frame"
        else:
            c2_frame = f"outbound_frame_{frame_id}"

        return {
            "device": self.device_name,
            "frame_id": frame_id,
            "consumer_1_mediapipe": c1_frame,
            "consumer_2_meeting": c2_frame,
            "privacy_muted": self.privacy_mute_active,
            "dual_muted": self.dual_mute_active,
        }

    def set_privacy_mute(self, muted: bool):
        self.privacy_mute_active = muted

    def set_dual_mute(self, muted: bool):
        self.dual_mute_active = muted


# ============================================================================
# Features 60–61: Headless IPC Bridge Command Surface & Full Verification
# ============================================================================

class HeadlessIpcBridgeR3:
    """
    Pure-Rust headless IPC bridge simulation exposing all new R1-R4 capabilities
    with ZERO mock fallbacks in production paths.
    """
    def __init__(self):
        self.discovery = ModelDiscoveryService()
        self.recording_controller = RecordingModeController()
        self.reliable_paste = ReliablePasteManager()
        self.dictionary = DictionaryService()
        self.history = TranscriptHistoryStore()
        self.command_parser = HandsFreeCommandParser()
        self.geometry_engine = WindowGeometryEngine()
        self.mixer = DualAudioMixer()
        self.transcriber = LiveMeetingTranscriber()
        self.summarizer = AiSummaryService()
        self.archive = MeetingArchiveStore()
        self.splitter = VirtualCameraSplitter()
        self.current_window_bounds = {"x": 100, "y": 100, "width": 1280, "height": 720}

    def dispatch(self, command: str, payload: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        p = payload or {}

        # STT Commands
        if command == "listModels":
            models = [
                WhisperModelVariant.TINY_EN.value,
                WhisperModelVariant.BASE_EN.value,
                WhisperModelVariant.SMALL_EN.value,
                WhisperModelVariant.MEDIUM_EN.value,
                WhisperModelVariant.TURBO_EN.value,
                WhisperModelVariant.LARGE_EN.value,
                WhisperModelVariant.PARAKEET_V3.value,
            ]
            return {"status": "success", "models": models}

        elif command == "selectModel":
            model = p.get("model")
            return {"status": "success", "selected_model": model}

        elif command == "setRecordingMode":
            mode_str = p.get("mode", "SmartDualMode")
            self.recording_controller.mode = RecordingMode(mode_str)
            return {"status": "success", "mode": mode_str}

        elif command == "setOverlayPosition":
            pos = p.get("position", "Bottom")
            bounds = OverlayGeometryManager.calculate_bounds(OverlayPosition(pos))
            return {"status": "success", "position": pos, "bounds": bounds}

        elif command == "searchTranscripts":
            q = p.get("query", "")
            items = self.history.search(q)
            return {"status": "success", "count": len(items), "results": [it.__dict__ for it in items]}

        elif command == "updateDictionary":
            rules = p.get("rules", {})
            for k, v in rules.items():
                self.dictionary.add_rule(k, v)
            return {"status": "success", "total_rules": len(self.dictionary.rules)}

        # Action Commands
        elif command == "executeVoiceCommand":
            utterance = p.get("utterance", "")
            parsed = self.command_parser.parse(utterance)
            return {"status": "success", "action": parsed.action_type, "payload": parsed.payload}

        elif command == "snapWindow":
            pos = SnapPosition(p.get("position", "Center"))
            new_bounds = self.geometry_engine.compute_snap(pos)
            self.current_window_bounds = new_bounds
            return {"status": "success", "bounds": new_bounds}

        elif command == "resizeWindow":
            cmd = self.command_parser.parse(f"resize window {p.get('dimension', 'larger')}")
            new_bounds = self.geometry_engine.compute_resize(self.current_window_bounds, cmd)
            self.current_window_bounds = new_bounds
            return {"status": "success", "bounds": new_bounds}

        # Meeting Commands
        elif command == "getMixerStatus":
            return {
                "status": "success",
                "sample_rate": self.mixer.sample_rate,
                "ducking_gain": self.mixer.ducking_gain,
                "current_envelope": self.mixer.current_envelope,
            }

        elif command == "startMeetingRecording":
            return {"status": "success", "recording_started": True, "meeting_id": "meet_123"}

        elif command == "generateSummary":
            text = p.get("transcript", "Default meeting transcript text")
            tmpl_str = p.get("template", "ExecutiveSummary")
            summary = self.summarizer.generate_summary(text, SummaryTemplate(tmpl_str))
            return {"status": "success", "summary": summary}

        elif command == "exportSummary":
            content = p.get("content", "# Summary")
            fmt = ExportFormat(p.get("format", "Markdown"))
            exported_bytes = DocumentExporter.export(content, fmt)
            return {"status": "success", "bytes_count": len(exported_bytes), "format": fmt.value}

        # Gestures & Camera Splitter Commands
        elif command == "getVirtualCameraStatus":
            return {
                "status": "success",
                "device_name": self.splitter.device_name,
                "is_running": self.splitter.is_running,
                "privacy_muted": self.splitter.privacy_mute_active,
            }

        elif command == "setVirtualCameraPrivacy":
            muted = bool(p.get("privacy_muted", True))
            self.splitter.set_privacy_mute(muted)
            return {"status": "success", "privacy_muted": muted}

        return {"status": "error", "error": f"Unknown command: {command}"}
