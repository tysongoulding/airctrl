import unittest
from tests.e2e.e2e_harness import MockWindowCrawler


class TestT1F15AppCrawler(unittest.TestCase):
    """
    Feature 15: Application & Window Crawler (Equivalence Partitions)
    """

    def setUp(self):
        self.crawler = MockWindowCrawler()

    def test_crawl_running_applications_count(self):
        """Partition: Crawler returns non-empty list of window descriptors."""
        windows = self.crawler.get_open_windows()
        self.assertGreaterEqual(len(windows), 2)

    def test_window_metadata_fields_present(self):
        """Partition: Window descriptors contain id, process_name, title, bounds."""
        windows = self.crawler.get_open_windows()
        w1 = windows[0]
        self.assertIn("id", w1)
        self.assertIn("process_name", w1)
        self.assertIn("title", w1)
        self.assertIn("bounds", w1)

    def test_active_window_focus_flag(self):
        """Partition: Exactly one active window marked focused."""
        windows = self.crawler.get_open_windows()
        focused_wins = [w for w in windows if w["is_focused"]]
        self.assertEqual(len(focused_wins), 1)
        self.assertEqual(focused_wins[0]["process_name"], "Code.exe")

    def test_window_geometry_bounds(self):
        """Partition: Bounding box contains positive dimensions."""
        windows = self.crawler.get_open_windows()
        w1 = windows[0]
        self.assertGreater(w1["bounds"]["width"], 0)
        self.assertGreater(w1["bounds"]["height"], 0)

    def test_process_id_mapping(self):
        """Partition: Each window maps to a non-zero OS process ID."""
        windows = self.crawler.get_open_windows()
        for w in windows:
            self.assertGreater(w["process_id"], 0)
