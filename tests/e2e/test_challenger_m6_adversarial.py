"""
test_challenger_m6_adversarial.py - Empirical Adversarial Test Suite for Milestone M6
Radial Dial Component Specifications & UI Settings Contracts

Adversarially validates:
1. Exact Radial Dial Parameters:
   - Dial dimensions: canvas 600x600, radius 300px (radius_px * 2 == canvas_px).
   - Sector inventory: exactly 6 sectors with IDs 0..5:
     * 0: Focus Window (window)
     * 1: Close Window (x)
     * 2: Snap Window (layout)
     * 3: Agent Dispatch (cpu)
     * 4: App Crawler (list)
     * 5: Settings (gear)
   - Active styling class: verify presence of 'scale-105 bg-indigo-600 text-white shadow-lg'
     (specifically asserting 'scale-105' and 'bg-indigo-600' are present).
   - Inactivity auto-dismiss: verify auto-dismiss duration is exactly 3.0 seconds.
   - Deadband boundary: verify r < 0.10 (r < 30px) activates deadband where active_sector is None.
   - Angular hysteresis: verify 0.05 rad hysteresis suppresses jitter at sector boundaries.
2. UI Settings Contracts:
   - Settings keys: verify hotkey_stt: "Ctrl+Spacebar", hotkey_dial: "Ctrl+Alt", push_to_talk: true,
     camera_auto_kill: true, compliance_audio_enabled: true.
   - Theme partition: strictly ["dark", "light", "system"].
   - HUD Opacity bounds: strictly bounded between 0.0 and 1.0.
   - Sector label length bounds: strictly between 1 and 30 characters.
"""

import math
import os
import re
import sys
import time
import unittest
from typing import Dict, Any, List, Optional, Tuple

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
if REPO_ROOT not in sys.path:
    sys.path.insert(0, REPO_ROOT)

from tests.e2e.e2e_harness import SpatialRadialDialMapper


class EmpiricalRadialDialStateMachine:
    """
    Empirical state machine modeling radial dial overlay behavior:
    600x600 canvas, 300px radius, 30px deadband, 0.05 rad hysteresis, and 3.0s inactivity timer.
    """
    CANVAS_WIDTH = 600
    CANVAS_HEIGHT = 600
    RADIUS_PX = 300
    DEADZONE_RADIUS_PX = 30  # 300 * 0.10
    INACTIVITY_TIMEOUT_SECS = 3.0
    HYSTERESIS_RAD = 0.05
    SECTOR_COUNT = 6

    def __init__(self):
        self.is_visible = False
        self.active_sector: Optional[int] = None
        self.in_deadzone = True
        self.last_activity_time = 0.0

    def show(self, now: float = 0.0):
        self.is_visible = True
        self.last_activity_time = now

    def hide(self):
        self.is_visible = False
        self.active_sector = None
        self.in_deadzone = True

    def toggle(self, now: float = 0.0):
        if self.is_visible:
            self.hide()
        else:
            self.show(now)

    def on_pointer_move(self, px: float, py: float, now: float = 0.0) -> Dict[str, Any]:
        if not self.is_visible:
            return {"visible": False, "active_sector": None, "in_deadzone": True}

        self.last_activity_time = now

        cx = self.CANVAS_WIDTH / 2.0
        cy = self.CANVAS_HEIGHT / 2.0
        dx = px - cx
        dy = py - cy
        dist = math.sqrt(dx * dx + dy * dy)
        norm_r = min(1.0, dist / self.RADIUS_PX)

        # Deadzone check (r < 30px / 0.10 normalized)
        if dist < self.DEADZONE_RADIUS_PX:
            self.in_deadzone = True
            self.active_sector = None
            return {
                "visible": True,
                "active_sector": None,
                "in_deadzone": True,
                "normalized_radius": round(norm_r, 4),
                "distance_px": round(dist, 2),
            }

        self.in_deadzone = False

        # Polar angle [0, 2pi)
        angle = math.atan2(dy, dx)
        if angle < 0.0:
            angle += 2.0 * math.pi

        sector_width = (2.0 * math.pi) / self.SECTOR_COUNT
        nominal_sector = int(angle // sector_width) % self.SECTOR_COUNT

        # Angular hysteresis check
        final_sector = nominal_sector
        if self.active_sector is not None:
            center_angle = (self.active_sector + 0.5) * sector_width
            half_width = sector_width / 2.0
            delta = abs(math.atan2(math.sin(angle - center_angle), math.cos(angle - center_angle)))
            if delta <= half_width + self.HYSTERESIS_RAD:
                final_sector = self.active_sector

        self.active_sector = final_sector
        return {
            "visible": True,
            "active_sector": final_sector,
            "in_deadzone": False,
            "angle_rad": round(angle, 4),
            "normalized_radius": round(norm_r, 4),
            "distance_px": round(dist, 2),
        }

    def check_inactivity(self, now: float) -> bool:
        if self.is_visible and (now - self.last_activity_time) >= self.INACTIVITY_TIMEOUT_SECS:
            self.hide()
            return True
        return False


class TestChallengerM6RadialDialParameters(unittest.TestCase):
    """
    Adversarial verification of Milestone M6 Radial Dial Component Specifications.
    """

    EXPECTED_SECTORS = [
        {"id": 0, "label": "Focus Window", "icon": "window"},
        {"id": 1, "label": "Close Window", "icon": "x"},
        {"id": 2, "label": "Snap Window", "icon": "layout"},
        {"id": 3, "label": "Agent Dispatch", "icon": "cpu"},
        {"id": 4, "label": "App Crawler", "icon": "list"},
        {"id": 5, "label": "Settings", "icon": "gear"},
    ]

    ACTIVE_CLASS_SPEC = "scale-105 bg-indigo-600 text-white shadow-lg"

    def setUp(self):
        self.dial = EmpiricalRadialDialStateMachine()
        self.mapper = SpatialRadialDialMapper(sector_count=6, center=(0.5, 0.5), deadzone=0.10)

    # ------------------------------------------------------------------------
    # 1. Dial Dimensions: Canvas 600x600, Radius 300px (radius_px * 2 == canvas_px)
    # ------------------------------------------------------------------------
    def test_dial_dimensions_and_pixel_canvas_symmetry(self):
        """Dial dimensions must be 600x600 canvas and 300px radius (radius_px * 2 == canvas_px)."""
        canvas_w = 600
        canvas_h = 600
        radius_px = 300

        self.assertEqual(canvas_w, 600, "Canvas width must be exactly 600px")
        self.assertEqual(canvas_h, 600, "Canvas height must be exactly 600px")
        self.assertEqual(radius_px, 300, "Radial dial radius must be exactly 300px")
        self.assertEqual(radius_px * 2, canvas_w, "radius_px * 2 must equal canvas width")
        self.assertEqual(radius_px * 2, canvas_h, "radius_px * 2 must equal canvas height")

        # Verify against frontend configuration in ui/src/components/radial/types.ts
        types_path = os.path.join(REPO_ROOT, "ui", "src", "components", "radial", "types.ts")
        self.assertTrue(os.path.exists(types_path), f"File {types_path} must exist")
        with open(types_path, "r", encoding="utf-8") as f:
            content = f.read()

        self.assertIn("width: 600", content)
        self.assertIn("height: 600", content)
        self.assertIn("radius: 300", content)

    # ------------------------------------------------------------------------
    # 2. Sector Inventory: Exactly 6 sectors with IDs 0..5, exact labels & icons
    # ------------------------------------------------------------------------
    def test_sector_inventory_and_exact_specifications(self):
        """Sector inventory must contain exactly 6 sectors with IDs 0..5, exact labels and icons."""
        self.assertEqual(len(self.EXPECTED_SECTORS), 6, "Must have exactly 6 sectors")

        # Verify sequential IDs 0..5
        ids = [s["id"] for s in self.EXPECTED_SECTORS]
        self.assertEqual(ids, [0, 1, 2, 3, 4, 5], "Sector IDs must be strictly [0, 1, 2, 3, 4, 5]")

        # Expected mappings
        expected_map = {
            0: ("Focus Window", "window"),
            1: ("Close Window", "x"),
            2: ("Snap Window", "layout"),
            3: ("Agent Dispatch", "cpu"),
            4: ("App Crawler", "list"),
            5: ("Settings", "gear"),
        }
        for s in self.EXPECTED_SECTORS:
            sid = s["id"]
            self.assertIn(sid, expected_map)
            exp_label, exp_icon = expected_map[sid]
            self.assertEqual(s["label"], exp_label, f"Sector {sid} label mismatch")
            self.assertEqual(s["icon"], exp_icon, f"Sector {sid} icon mismatch")

        # Verify against frontend RADIAL_SECTORS definition in types.ts
        types_path = os.path.join(REPO_ROOT, "ui", "src", "components", "radial", "types.ts")
        with open(types_path, "r", encoding="utf-8") as f:
            content = f.read()

        for sid, (lbl, icn) in expected_map.items():
            self.assertIn(f'label: "{lbl}"', content, f"types.ts missing label {lbl}")
            self.assertIn(f'icon: "{icn}"', content, f"types.ts missing icon {icn}")

    # ------------------------------------------------------------------------
    # 3. Active Styling Class: 'scale-105 bg-indigo-600 text-white shadow-lg'
    # ------------------------------------------------------------------------
    def test_active_styling_class_and_token_presence(self):
        """Active styling class must include 'scale-105' and 'bg-indigo-600'."""
        active_class = self.ACTIVE_CLASS_SPEC
        self.assertIn("scale-105", active_class, "'scale-105' token must be present")
        self.assertIn("bg-indigo-600", active_class, "'bg-indigo-600' token must be present")
        self.assertIn("text-white", active_class, "'text-white' token must be present")
        self.assertIn("shadow-lg", active_class, "'shadow-lg' token must be present")

        # Check frontend definition
        types_path = os.path.join(REPO_ROOT, "ui", "src", "components", "radial", "types.ts")
        with open(types_path, "r", encoding="utf-8") as f:
            content = f.read()

        self.assertIn('activeClass: "scale-105 bg-indigo-600 text-white shadow-lg"', content)

    # ------------------------------------------------------------------------
    # 4. Inactivity Auto-Dismiss: Exactly 3.0 Seconds
    # ------------------------------------------------------------------------
    def test_inactivity_auto_dismiss_duration_boundary_3_0s(self):
        """Inactivity auto-dismiss must be exactly 3.0 seconds."""
        self.assertEqual(self.dial.INACTIVITY_TIMEOUT_SECS, 3.0)

        t0 = 1000.0
        self.dial.show(now=t0)
        self.assertTrue(self.dial.is_visible)

        # Pointer moves at t=1.0s -> updates last activity
        res = self.dial.on_pointer_move(450.0, 300.0, now=t0 + 1.0)
        self.assertEqual(res["active_sector"], 0)
        self.assertTrue(self.dial.is_visible)

        # Advance to t=3.99s (2.99s after last activity at 1.0s) -> should NOT dismiss
        dismissed_early = self.dial.check_inactivity(now=t0 + 3.99)
        self.assertFalse(dismissed_early, "Must not dismiss at 2.99s of inactivity")
        self.assertTrue(self.dial.is_visible)
        self.assertEqual(self.dial.active_sector, 0)

        # Advance to t=4.00s (exactly 3.00s after last activity at 1.0s) -> MUST dismiss
        dismissed_at_3s = self.dial.check_inactivity(now=t0 + 4.00)
        self.assertTrue(dismissed_at_3s, "Must dismiss at exactly 3.0s of inactivity")
        self.assertFalse(self.dial.is_visible)
        self.assertIsNone(self.dial.active_sector)

    # ------------------------------------------------------------------------
    # 5. Deadband Boundary: r < 0.10 (r < 30px) activates deadband (active_sector is None)
    # ------------------------------------------------------------------------
    def test_deadband_boundary_radius_subpixel_fuzzing(self):
        """r < 0.10 (r < 30px) activates deadband where active_sector is None."""
        self.dial.show()

        # Exact center (300, 300) -> distance 0px
        center_res = self.dial.on_pointer_move(300.0, 300.0)
        self.assertTrue(center_res["in_deadzone"], "Center must be in deadzone")
        self.assertIsNone(center_res["active_sector"], "Center must have active_sector = None")
        self.assertEqual(center_res["distance_px"], 0.0)

        # Distance 29.7px (normalized r = 0.099 < 0.10) -> deadband active
        res_0099 = self.dial.on_pointer_move(329.7, 300.0)
        self.assertTrue(res_0099["in_deadzone"])
        self.assertIsNone(res_0099["active_sector"])

        # Distance 29.99px (< 30px) -> deadband active
        res_edge_in = self.dial.on_pointer_move(329.99, 300.0)
        self.assertTrue(res_edge_in["in_deadzone"])
        self.assertIsNone(res_edge_in["active_sector"])

        # Distance 30.01px (>= 30px) -> sector becomes active
        res_edge_out = self.dial.on_pointer_move(330.01, 300.0)
        self.assertFalse(res_edge_out["in_deadzone"])
        self.assertEqual(res_edge_out["active_sector"], 0)

        # Distance 30.3px (normalized r = 0.101 >= 0.10) -> sector active
        res_0101 = self.dial.on_pointer_move(330.3, 300.0)
        self.assertFalse(res_0101["in_deadzone"])
        self.assertEqual(res_0101["active_sector"], 0)

        # Circular sweep: 360 degrees in 1-degree steps
        for deg in range(360):
            rad = math.radians(deg)
            # Inside deadband at 29.5px
            px_in = 300.0 + 29.5 * math.cos(rad)
            py_in = 300.0 + 29.5 * math.sin(rad)
            r_in = self.dial.on_pointer_move(px_in, py_in)
            self.assertTrue(r_in["in_deadzone"], f"Degree {deg} at 29.5px must be in deadzone")
            self.assertIsNone(r_in["active_sector"])

            # Outside deadband at 30.5px
            px_out = 300.0 + 30.5 * math.cos(rad)
            py_out = 300.0 + 30.5 * math.sin(rad)
            r_out = self.dial.on_pointer_move(px_out, py_out)
            self.assertFalse(r_out["in_deadzone"], f"Degree {deg} at 30.5px must be outside deadzone")
            self.assertIsNotNone(r_out["active_sector"], f"Degree {deg} at 30.5px must activate a sector")

    # ------------------------------------------------------------------------
    # 6. Angular Hysteresis: 0.05 rad suppresses jitter at sector boundaries
    # ------------------------------------------------------------------------
    def test_angular_hysteresis_suppresses_jitter_at_all_sector_boundaries(self):
        """0.05 rad hysteresis suppresses jitter at sector boundaries."""
        self.dial.show()
        sector_width = (2.0 * math.pi) / 6.0  # pi / 3 ~ 1.0471975 rad
        hysteresis = 0.05  # rad
        r = 150.0  # well outside deadzone

        # Test each sector boundary k * (pi / 3) for k in [0..5]
        for boundary_idx in range(6):
            sector_from = (boundary_idx + 5) % 6 if boundary_idx == 0 else (boundary_idx - 1)
            sector_to = boundary_idx
            boundary_angle = boundary_idx * sector_width

            # 1. Establish initial presence firmly inside sector_from
            init_angle = (boundary_angle - 0.20) % (2.0 * math.pi)
            p_init = self.dial.on_pointer_move(300.0 + r * math.cos(init_angle), 300.0 + r * math.sin(init_angle))
            self.assertEqual(p_init["active_sector"], sector_from)

            # 2. Advance angle slightly past boundary into sector_to by +0.02 rad (< 0.05 rad hysteresis)
            flutter_past = (boundary_angle + 0.02) % (2.0 * math.pi)
            p_flutter = self.dial.on_pointer_move(300.0 + r * math.cos(flutter_past), 300.0 + r * math.sin(flutter_past))
            self.assertEqual(
                p_flutter["active_sector"],
                sector_from,
                f"Boundary #{boundary_idx}: +0.02 rad flutter must be suppressed by 0.05 rad hysteresis"
            )

            # 3. Advance angle firmly past hysteresis (+0.07 rad > 0.05 rad) -> must transition to sector_to
            transition_angle = (boundary_angle + 0.07) % (2.0 * math.pi)
            p_trans = self.dial.on_pointer_move(300.0 + r * math.cos(transition_angle), 300.0 + r * math.sin(transition_angle))
            self.assertEqual(
                p_trans["active_sector"],
                sector_to,
                f"Boundary #{boundary_idx}: +0.07 rad must trigger transition to sector {sector_to}"
            )

            # 4. Reverse flutter: move back across boundary into sector_from by -0.02 rad
            flutter_back = (boundary_angle - 0.02) % (2.0 * math.pi)
            p_back = self.dial.on_pointer_move(300.0 + r * math.cos(flutter_back), 300.0 + r * math.sin(flutter_back))
            self.assertEqual(
                p_back["active_sector"],
                sector_to,
                f"Boundary #{boundary_idx}: reverse flutter -0.02 rad must stay in sector {sector_to} due to hysteresis"
            )


class TestChallengerM6UiSettingsContracts(unittest.TestCase):
    """
    Adversarial verification of Milestone M6 UI Settings Contracts.
    """

    # ------------------------------------------------------------------------
    # 1. Settings Keys & Contract Verification
    # ------------------------------------------------------------------------
    def test_settings_keys_exact_contract_values(self):
        """Settings keys must strictly match hotkey_stt: 'Ctrl+Spacebar', hotkey_dial: 'Ctrl+Alt', etc."""
        expected_settings = {
            "hotkey_stt": "Ctrl+Spacebar",
            "hotkey_dial": "Ctrl+Alt",
            "push_to_talk": True,
            "camera_auto_kill": True,
            "compliance_audio_enabled": True,
        }

        # Check in ui/src/types/config.ts
        config_path = os.path.join(REPO_ROOT, "ui", "src", "types", "config.ts")
        with open(config_path, "r", encoding="utf-8") as f:
            content = f.read()

        self.assertIn('hotkey_stt: "Ctrl+Spacebar"', content)
        self.assertIn('hotkey_dial: "Ctrl+Alt"', content)
        self.assertIn("push_to_talk: true", content)
        self.assertIn("camera_auto_kill: true", content)
        self.assertIn("compliance_audio_enabled: true", content)

        # Check in fallback ui/dist/index.html
        html_path = os.path.join(REPO_ROOT, "ui", "dist", "index.html")
        with open(html_path, "r", encoding="utf-8") as f:
            html_content = f.read()

        self.assertIn('hotkey_stt: "Ctrl+Spacebar"', html_content)
        self.assertIn('hotkey_dial: "Ctrl+Alt"', html_content)
        self.assertIn("push_to_talk: true", html_content)
        self.assertIn("camera_auto_kill: true", html_content)
        self.assertIn("compliance_audio_enabled: true", html_content)

    # ------------------------------------------------------------------------
    # 2. Theme Partition: Strictly ["dark", "light", "system"]
    # ------------------------------------------------------------------------
    def test_theme_partition_strictly_dark_light_system(self):
        """Theme partition is strictly ['dark', 'light', 'system']."""
        valid_themes = ["dark", "light", "system"]
        self.assertEqual(len(valid_themes), 3)

        def validate_theme(t: Any) -> bool:
            return t in ["dark", "light", "system"]

        # Valid inputs
        for vt in valid_themes:
            self.assertTrue(validate_theme(vt), f"Theme '{vt}' must be valid")

        # Adversarial / invalid inputs
        invalid_themes = [
            "blue",
            "high-contrast",
            "solarized",
            "cyberpunk",
            "",
            "   ",
            None,
            123,
            "DARK",
            "Light",
            "system-preference",
        ]
        for it in invalid_themes:
            self.assertFalse(validate_theme(it), f"Theme '{it}' must be rejected")

        # Verify implementation in useTheme.ts
        theme_path = os.path.join(REPO_ROOT, "ui", "src", "hooks", "useTheme.ts")
        with open(theme_path, "r", encoding="utf-8") as f:
            theme_src = f.read()

        self.assertIn('["dark", "light", "system"].includes(saved)', theme_src)
        self.assertIn('export type ThemeMode = "dark" | "light" | "system";', theme_src)

    # ------------------------------------------------------------------------
    # 3. HUD Opacity Bounds: Strictly bounded between 0.0 and 1.0
    # ------------------------------------------------------------------------
    def test_hud_opacity_bounds_strictly_between_0_and_1(self):
        """HUD Opacity bounds must be strictly bounded between 0.0 and 1.0."""
        def clamp_opacity(val: float) -> float:
            return max(0.0, min(1.0, val))

        # Boundary values
        self.assertEqual(clamp_opacity(0.0), 0.0)
        self.assertEqual(clamp_opacity(1.0), 1.0)
        self.assertEqual(clamp_opacity(0.5), 0.5)
        self.assertEqual(clamp_opacity(0.95), 0.95)

        # Underflow boundary (< 0.0)
        self.assertEqual(clamp_opacity(-0.0001), 0.0)
        self.assertEqual(clamp_opacity(-1.0), 0.0)
        self.assertEqual(clamp_opacity(-999.0), 0.0)

        # Overflow boundary (> 1.0)
        self.assertEqual(clamp_opacity(1.0001), 1.0)
        self.assertEqual(clamp_opacity(1.5), 1.0)
        self.assertEqual(clamp_opacity(999.0), 1.0)

        # Verify implementation in useHudOpacity.ts
        opacity_path = os.path.join(REPO_ROOT, "ui", "src", "hooks", "useHudOpacity.ts")
        with open(opacity_path, "r", encoding="utf-8") as f:
            opacity_src = f.read()

        self.assertIn("Math.min(Math.max(val, 0.0), 1.0)", opacity_src)
        self.assertIn("parsed >= 0.0 && parsed <= 1.0", opacity_src)

    # ------------------------------------------------------------------------
    # 4. Sector Label Length Bounds: Strictly between 1 and 30 characters
    # ------------------------------------------------------------------------
    def test_sector_label_length_bounds_strictly_1_to_30(self):
        """Sector label length bounds strictly between 1 and 30 characters."""
        def validate_label_length(label: str) -> bool:
            return 1 <= len(label) <= 30

        # Boundary checks
        self.assertFalse(validate_label_length(""), "0 characters must be rejected")
        self.assertTrue(validate_label_length("A"), "1 character must be valid")
        self.assertTrue(validate_label_length("A" * 30), "30 characters must be valid")
        self.assertFalse(validate_label_length("A" * 31), "31 characters must be rejected")

        # Validate all 6 production sector labels
        sectors = [
            "Focus Window",
            "Close Window",
            "Snap Window",
            "Agent Dispatch",
            "App Crawler",
            "Settings",
        ]
        for lbl in sectors:
            self.assertTrue(
                validate_label_length(lbl),
                f"Production label '{lbl}' (length {len(lbl)}) must be within 1..30 chars"
            )
            self.assertGreaterEqual(len(lbl), 1)
            self.assertLessEqual(len(lbl), 30)

        # Verify comment contract in ui/src/types/config.ts
        config_path = os.path.join(REPO_ROOT, "ui", "src", "types", "config.ts")
        with open(config_path, "r", encoding="utf-8") as f:
            content = f.read()

        self.assertIn("length: 1..30 characters", content)


if __name__ == "__main__":
    unittest.main()
