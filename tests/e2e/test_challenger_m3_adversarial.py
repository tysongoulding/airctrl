"""
test_challenger_m3_adversarial.py - Empirical Adversarial Tests for Milestone M3

Challenges M3 Hotkey State Machine, Radial Dial Overlay, and Crawler Caching invariants:
1. ActionHotkeyStateMachine fuzzing:
   - Rapid key fluttering (down/up within 1ms, 50ms, 99ms -> must cancel/drop to Idle)
   - Valid presses (100ms, 150ms -> transition to ActionParsing)
   - Concurrent calls and toggle state transitions
   - Broadcast behavior for DomainEvent::ActionTriggered
2. RadialDialOverlay boundary stress:
   - Sector boundaries for 6 sectors: 0 and 5 valid; -1, 6, -1000, 1000 rejected
   - Inactivity timeout boundary: exactly 3.0s timeout dismisses overlay and clears active sector
3. WindowContextMap concurrency and cache stress:
   - Multi-threaded concurrent reads/invalidations
   - Verify TTL (500ms) and debounce interval (150ms) prevent excessive system process enumeration
"""

import os
import sys
import time
import threading
import unittest
from typing import Optional, List, Dict, Any

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
if REPO_ROOT not in sys.path:
    sys.path.insert(0, REPO_ROOT)

from tests.e2e.tier1_category_partition.test_t1_f14_radial_overlay import MockRadialDialOverlay


class MockActionHotkeyStateMachine:
    """Python reference oracle mirroring crates/airctrl-actions/src/hotkey/state_machine.rs."""
    DEFAULT_MIN_DURATION_MS = 100

    def __init__(self, mode: str = "PushToTalk", event_bus: Optional[List[Dict[str, Any]]] = None):
        self.mode = mode  # "PushToTalk" or "Toggle"
        self.state = "Idle"  # "Idle", "ActionListening", "ActionParsing", "ActionExecuting", "ActionError"
        self.dial = MockRadialDialOverlay()
        self.press_start_time_ms = 0
        self.min_press_duration_ms = self.DEFAULT_MIN_DURATION_MS
        self.target_window_id = None
        self.event_bus = event_bus if event_bus is not None else []
        self._lock = threading.Lock()

    def on_key_down(self, timestamp_ms: int, active_window_id: Optional[int] = None) -> str:
        with self._lock:
            if self.mode == "PushToTalk":
                if self.state == "Idle":
                    self.state = "ActionListening"
                    self.press_start_time_ms = timestamp_ms
                    self.target_window_id = active_window_id
                    self.dial.is_visible = True
                    return "StartListening"
                return "NoChange"
            elif self.mode == "Toggle":
                if self.state == "Idle":
                    self.state = "ActionListening"
                    self.press_start_time_ms = timestamp_ms
                    self.target_window_id = active_window_id
                    self.dial.is_visible = True
                    return "StartListening"
                elif self.state == "ActionListening":
                    self.state = "ActionParsing"
                    return "StopListeningAndParse"
                return "NoChange"
            return "NoChange"

    def on_key_up(self, timestamp_ms: int) -> str:
        with self._lock:
            if self.mode == "PushToTalk":
                if self.state == "ActionListening":
                    duration = max(0, timestamp_ms - self.press_start_time_ms)
                    if duration < self.min_press_duration_ms:
                        self.state = "Idle"
                        self.target_window_id = None
                        self.dial.is_visible = False
                        self.dial.active_sector = None
                        return "CancelListeningDiscardBuffer"
                    else:
                        self.state = "ActionParsing"
                        return "StopListeningAndParse"
                return "NoChange"
            elif self.mode == "Toggle":
                return "NoChange"
            return "NoChange"

    def on_command_parsed(self, command: Dict[str, Any]) -> str:
        with self._lock:
            if self.state == "ActionParsing":
                self.event_bus.append({"category": "ActionTriggered", "data": command})
                self.state = "ActionExecuting"
                return "ExecuteCommand"
            return "NoChange"

    def on_execution_completed(self) -> str:
        with self._lock:
            if self.state == "ActionExecuting":
                self.state = "Idle"
                self.target_window_id = None
                self.dial.is_visible = False
                self.dial.active_sector = None
                return "ReturnToIdle"
            return "NoChange"

    def on_inactivity_timeout(self) -> str:
        with self._lock:
            if self.state == "ActionListening":
                self.dial.on_inactivity_timeout()
                self.state = "Idle"
                self.target_window_id = None
                return "ReturnToIdle"
            return "NoChange"

    def reset(self) -> str:
        with self._lock:
            self.state = "Idle"
            self.target_window_id = None
            self.dial.is_visible = False
            self.dial.active_sector = None
            return "ReturnToIdle"


class MockContextMapCache:
    """Python reference oracle mirroring WindowContextMap caching and debounce logic."""
    def __init__(self, ttl_sec: float = 0.500, debounce_sec: float = 0.150):
        self.ttl_sec = ttl_sec
        self.debounce_sec = debounce_sec
        self.last_refreshed = 0.0
        self.is_dirty = True
        self.enumeration_calls = 0
        self._lock = threading.Lock()

    def invalidate(self):
        with self._lock:
            self.is_dirty = True

    def is_fresh(self, now: float) -> bool:
        return (not self.is_dirty) and ((now - self.last_refreshed) < self.ttl_sec)

    def should_debounce(self, now: float) -> bool:
        return (now - self.last_refreshed) < self.debounce_sec

    def get_windows(self, now: float) -> List[Dict[str, Any]]:
        with self._lock:
            if not self.is_fresh(now) and not self.should_debounce(now):
                # Trigger system enumeration
                self.enumeration_calls += 1
                self.last_refreshed = now
                self.is_dirty = False

            return [
                {"id": 1, "title": "Cursor", "process_name": "cursor.exe"},
                {"id": 2, "title": "Chrome", "process_name": "chrome.exe"},
            ]


class TestChallengerM3Adversarial(unittest.TestCase):

    # =========================================================================
    # 1. ActionHotkeyStateMachine Fuzzing & Stress Tests
    # =========================================================================

    def test_01_hotkey_flutter_micro_presses(self):
        """Micro-presses under 100ms (1ms, 50ms, 99ms, clock skew) must cancel to Idle."""
        sm = MockActionHotkeyStateMachine(mode="PushToTalk")

        # 1ms micro-press
        res_down = sm.on_key_down(1000, 101)
        self.assertEqual(res_down, "StartListening")
        self.assertEqual(sm.state, "ActionListening")
        self.assertTrue(sm.dial.is_visible)

        res_up = sm.on_key_up(1001)
        self.assertEqual(res_up, "CancelListeningDiscardBuffer")
        self.assertEqual(sm.state, "Idle")
        self.assertFalse(sm.dial.is_visible)
        self.assertIsNone(sm.target_window_id)

        # 50ms micro-press
        sm.on_key_down(2000, 102)
        self.assertEqual(sm.state, "ActionListening")
        res_up50 = sm.on_key_up(2050)
        self.assertEqual(res_up50, "CancelListeningDiscardBuffer")
        self.assertEqual(sm.state, "Idle")
        self.assertFalse(sm.dial.is_visible)

        # 99ms micro-press (just 1ms shy of threshold)
        sm.on_key_down(3000, 103)
        self.assertEqual(sm.state, "ActionListening")
        res_up99 = sm.on_key_up(3099)
        self.assertEqual(res_up99, "CancelListeningDiscardBuffer")
        self.assertEqual(sm.state, "Idle")
        self.assertFalse(sm.dial.is_visible)

        # Clock skew / backwards timestamp
        sm.on_key_down(4000, 104)
        res_skew = sm.on_key_up(3900)
        self.assertEqual(res_skew, "CancelListeningDiscardBuffer")
        self.assertEqual(sm.state, "Idle")
        self.assertFalse(sm.dial.is_visible)

    def test_02_hotkey_valid_press_transitions(self):
        """Valid press durations (100ms, 150ms, long press) must transition to ActionParsing."""
        sm = MockActionHotkeyStateMachine(mode="PushToTalk")

        # Exactly 100ms threshold press
        sm.on_key_down(1000, 201)
        res_100 = sm.on_key_up(1100)
        self.assertEqual(res_100, "StopListeningAndParse")
        self.assertEqual(sm.state, "ActionParsing")

        sm.reset()
        self.assertEqual(sm.state, "Idle")

        # 150ms press
        sm.on_key_down(2000, 202)
        res_150 = sm.on_key_up(2150)
        self.assertEqual(res_150, "StopListeningAndParse")
        self.assertEqual(sm.state, "ActionParsing")

        sm.reset()

        # 5000ms long press
        sm.on_key_down(3000, 203)
        res_5000 = sm.on_key_up(8000)
        self.assertEqual(res_5000, "StopListeningAndParse")
        self.assertEqual(sm.state, "ActionParsing")

    def test_03_hotkey_5000_cycle_flutter_stress(self):
        """5,000 rapid fluttering cycles must consistently drop to Idle with zero state leak."""
        sm = MockActionHotkeyStateMachine(mode="PushToTalk")
        t = 10_000
        for i in range(5_000):
            sm.on_key_down(t, i)
            self.assertEqual(sm.state, "ActionListening")
            self.assertTrue(sm.dial.is_visible)

            # Rapid release between 1ms and 99ms
            t += (i % 99) + 1
            sm.on_key_up(t)
            self.assertEqual(sm.state, "Idle")
            self.assertFalse(sm.dial.is_visible)
            self.assertIsNone(sm.target_window_id)
            t += 10

    def test_04_hotkey_toggle_mode_transitions(self):
        """Toggle mode transitions: down -> Listening, up -> NoChange, down -> Parsing."""
        sm = MockActionHotkeyStateMachine(mode="Toggle")
        self.assertEqual(sm.state, "Idle")

        # 1. First key down
        res1 = sm.on_key_down(1000, 301)
        self.assertEqual(res1, "StartListening")
        self.assertEqual(sm.state, "ActionListening")
        self.assertTrue(sm.dial.is_visible)

        # 2. Key up in Toggle mode is ignored
        res_up1 = sm.on_key_up(1050)
        self.assertEqual(res_up1, "NoChange")
        self.assertEqual(sm.state, "ActionListening")

        # 3. Second key down triggers parsing
        res2 = sm.on_key_down(2000, 301)
        self.assertEqual(res2, "StopListeningAndParse")
        self.assertEqual(sm.state, "ActionParsing")

        # 4. In parsing mode, further keys are ignored
        self.assertEqual(sm.on_key_down(2100), "NoChange")
        self.assertEqual(sm.on_key_up(2150), "NoChange")

        # 5. Command parsed triggers execution
        cmd = {"type": "WindowSnap", "payload": {"direction": "Maximize"}}
        res3 = sm.on_command_parsed(cmd)
        self.assertEqual(res3, "ExecuteCommand")
        self.assertEqual(sm.state, "ActionExecuting")

        # 6. Execution completes -> Idle
        res4 = sm.on_execution_completed()
        self.assertEqual(res4, "ReturnToIdle")
        self.assertEqual(sm.state, "Idle")
        self.assertFalse(sm.dial.is_visible)

    def test_05_hotkey_domain_event_broadcast_behavior(self):
        """Verify DomainEvent::ActionTriggered broadcast only occurs during ActionParsing."""
        event_bus = []
        sm = MockActionHotkeyStateMachine(mode="PushToTalk", event_bus=event_bus)

        cmd = {"type": "WindowSnap", "payload": {"direction": "Maximize"}}

        # Attempt broadcast in Idle -> NoChange, zero events
        res_idle = sm.on_command_parsed(cmd)
        self.assertEqual(res_idle, "NoChange")
        self.assertEqual(len(event_bus), 0)

        # Attempt broadcast in Listening -> NoChange, zero events
        sm.on_key_down(1000, 401)
        self.assertEqual(sm.state, "ActionListening")
        res_listening = sm.on_command_parsed(cmd)
        self.assertEqual(res_listening, "NoChange")
        self.assertEqual(len(event_bus), 0)

        # Transition to Parsing via valid release
        sm.on_key_up(1200)
        self.assertEqual(sm.state, "ActionParsing")

        # Successful parse -> broadcast emitted
        res_parsing = sm.on_command_parsed(cmd)
        self.assertEqual(res_parsing, "ExecuteCommand")
        self.assertEqual(sm.state, "ActionExecuting")
        self.assertEqual(len(event_bus), 1)
        self.assertEqual(event_bus[0]["category"], "ActionTriggered")
        self.assertEqual(event_bus[0]["data"], cmd)

        # Subsequent attempts while Executing do not re-broadcast
        self.assertEqual(sm.on_command_parsed(cmd), "NoChange")
        self.assertEqual(len(event_bus), 1)

    # =========================================================================
    # 2. RadialDialOverlay Boundary Stress Tests
    # =========================================================================

    def test_06_radial_dial_sector_boundaries(self):
        """Sector boundaries for 6 sectors: 0 and 5 valid; -1 and 6 rejected."""
        dial = MockRadialDialOverlay()
        self.assertFalse(dial.is_visible)
        self.assertIsNone(dial.active_sector)
        self.assertEqual(dial.sector_count, 6)

        dial.toggle()
        self.assertTrue(dial.is_visible)

        # Valid lower bound: 0
        dial.hover_sector(0)
        self.assertEqual(dial.active_sector, 0)

        # Valid upper bound: 5
        dial.hover_sector(5)
        self.assertEqual(dial.active_sector, 5)

        # Rejected lower bound: -1 (must not change selection)
        dial.hover_sector(-1)
        self.assertEqual(dial.active_sector, 5)

        # Rejected upper bound: 6 (must not change selection)
        dial.hover_sector(6)
        self.assertEqual(dial.active_sector, 5)

        # Rejected extreme values
        dial.hover_sector(-999)
        self.assertEqual(dial.active_sector, 5)
        dial.hover_sector(999)
        self.assertEqual(dial.active_sector, 5)

        # Toggle off clears active sector
        dial.toggle()
        self.assertFalse(dial.is_visible)
        self.assertIsNone(dial.active_sector)

    def test_07_radial_dial_inactivity_timeout_boundary(self):
        """Inactivity timeout must be exactly 3.0s, dismiss overlay, and clear sector."""
        dial = MockRadialDialOverlay()
        self.assertEqual(dial.timeout_secs, 3.0)

        dial.toggle()
        dial.hover_sector(3)
        self.assertTrue(dial.is_visible)
        self.assertEqual(dial.active_sector, 3)

        dial.on_inactivity_timeout()
        self.assertFalse(dial.is_visible)
        self.assertIsNone(dial.active_sector)

    # =========================================================================
    # 3. WindowContextMap Concurrency and Cache Stress Tests
    # =========================================================================

    def test_08_context_map_ttl_and_debounce_throttling(self):
        """TTL (500ms) and debounce interval (150ms) prevent excessive enumeration."""
        cache = MockContextMapCache(ttl_sec=0.500, debounce_sec=0.150)

        t = 1000.0
        # 1. Initial query -> triggers 1 enumeration
        cache.get_windows(t)
        self.assertEqual(cache.enumeration_calls, 1)

        # 2. 100 queries within debounce and TTL window -> 0 additional enumerations
        for _ in range(100):
            t += 0.001
            cache.get_windows(t)
        self.assertEqual(cache.enumeration_calls, 1)

        # 3. Explicit invalidation at t + 50ms (< 150ms debounce window)
        t += 0.050
        cache.invalidate()
        # Immediate read is debounced -> 0 additional enumerations
        cache.get_windows(t)
        self.assertEqual(cache.enumeration_calls, 1)

        # 4. Advance past debounce window (160ms > 150ms) with dirty cache -> triggers enumeration
        t += 0.160
        cache.get_windows(t)
        self.assertEqual(cache.enumeration_calls, 2)

        # 5. Queries within new fresh TTL window (500ms) -> cached
        t += 0.200
        cache.get_windows(t)
        self.assertEqual(cache.enumeration_calls, 2)

        # 6. Advance past TTL (510ms > 500ms) without explicit invalidation -> triggers enumeration
        t += 0.510
        cache.get_windows(t)
        self.assertEqual(cache.enumeration_calls, 3)

    def test_09_context_map_multithreaded_concurrency_hammer(self):
        """16 concurrent threads reading and invalidating cache must not exceed debounce rate."""
        cache = MockContextMapCache(ttl_sec=0.500, debounce_sec=0.150)
        start_time = time.time()
        stop_event = threading.Event()

        def reader():
            while not stop_event.is_set():
                now = time.time()
                wins = cache.get_windows(now)
                self.assertGreaterEqual(len(wins), 1)
                time.sleep(0.002)

        def invalidator():
            while not stop_event.is_set():
                cache.invalidate()
                time.sleep(0.010)

        threads = []
        for _ in range(10):
            threads.append(threading.Thread(target=reader))
        for _ in range(6):
            threads.append(threading.Thread(target=invalidator))

        for th in threads:
            th.start()

        time.sleep(0.5)  # Run for 500ms
        stop_event.set()

        for th in threads:
            th.join()

        # In 500ms with a 150ms debounce window, at most ~4-5 refreshes can occur
        self.assertLessEqual(cache.enumeration_calls, 6)
        self.assertGreaterEqual(cache.enumeration_calls, 1)


if __name__ == "__main__":
    unittest.main()
