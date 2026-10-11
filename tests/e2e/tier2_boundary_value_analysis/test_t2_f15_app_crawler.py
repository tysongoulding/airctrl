import unittest
from tests.e2e.e2e_harness import MockWindowCrawler


class TestT2F15AppCrawler(unittest.TestCase):
    """
    Feature 15: Application & Window Crawler (Boundary Value Analysis)
    """

    def setUp(self):
        self.crawler = MockWindowCrawler()

    def test_empty_window_list_boundary(self):
        """BVA: System with 0 open windows returns empty list."""
        self.crawler.windows = []
        self.assertEqual(len(self.crawler.get_open_windows()), 0)

    def test_single_window_list_boundary(self):
        """BVA: System with exactly 1 window returns list of length 1."""
        self.crawler.windows = [{"id": 1, "process_id": 10, "process_name": "app.exe", "title": "App"}]
        self.assertEqual(len(self.crawler.get_open_windows()), 1)

    def test_pid_minimum_boundary_1(self):
        """BVA: Lowest valid process ID 1 (init/system)."""
        self.crawler.windows = [{"id": 1, "process_id": 1, "process_name": "system", "title": "System"}]
        win = self.crawler.get_open_windows()[0]
        self.assertEqual(win["process_id"], 1)

    def test_pid_maximum_boundary_65535(self):
        """BVA: Process ID 65535 upper boundary."""
        self.crawler.windows = [{"id": 2, "process_id": 65535, "process_name": "app", "title": "App"}]
        win = self.crawler.get_open_windows()[0]
        self.assertEqual(win["process_id"], 65535)

    def test_window_title_maximum_length_boundary_500(self):
        """BVA: 500-character window title string."""
        long_title = "T" * 500
        self.crawler.windows = [{"id": 3, "process_id": 100, "process_name": "app", "title": long_title}]
        win = self.crawler.get_open_windows()[0]
        self.assertEqual(len(win["title"]), 500)
