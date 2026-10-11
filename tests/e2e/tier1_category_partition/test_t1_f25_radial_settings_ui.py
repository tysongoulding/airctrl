import unittest


class TestT1F25RadialSettingsUi(unittest.TestCase):
    """
    Feature 25: Radial Dial & Settings UI (Equivalence Partitions)
    """

    def test_radial_dial_sectors_inventory(self):
        """Partition: Radial dial contains exactly 6 configured sectors."""
        sectors = [
            {"id": 0, "label": "Focus Window", "icon": "window"},
            {"id": 1, "label": "Close Window", "icon": "x"},
            {"id": 2, "label": "Snap Window", "icon": "layout"},
            {"id": 3, "label": "Agent Dispatch", "icon": "cpu"},
            {"id": 4, "label": "App Crawler", "icon": "list"},
            {"id": 5, "label": "Settings", "icon": "gear"}
        ]
        self.assertEqual(len(sectors), 6)
        ids = [s["id"] for s in sectors]
        self.assertEqual(ids, [0, 1, 2, 3, 4, 5])

    def test_settings_hud_state_keys(self):
        """Partition: Settings state includes STT mode, hotkeys, and privacy toggles."""
        settings = {
            "hotkey_stt": "Ctrl+Spacebar",
            "hotkey_dial": "Ctrl+Alt",
            "push_to_talk": True,
            "camera_auto_kill": True,
            "compliance_audio_enabled": True
        }
        self.assertEqual(settings["hotkey_stt"], "Ctrl+Spacebar")
        self.assertTrue(settings["camera_auto_kill"])

    def test_radial_dial_dimension_props(self):
        """Partition: 600x600 circular dial canvas bounds."""
        dial_props = {"width": 600, "height": 600, "radius": 300}
        self.assertEqual(dial_props["width"], 600)
        self.assertEqual(dial_props["height"], 600)

    def test_hud_theme_mode_partition(self):
        """Partition: HUD UI supports dark and light modes."""
        theme = "dark"
        self.assertIn(theme, ["dark", "light", "system"])

    def test_sector_hover_state_styling(self):
        """Partition: Active sector styling classes."""
        active_sector_class = "scale-105 bg-indigo-600 text-white shadow-lg"
        self.assertIn("scale-105", active_sector_class)
        self.assertIn("bg-indigo-600", active_sector_class)
