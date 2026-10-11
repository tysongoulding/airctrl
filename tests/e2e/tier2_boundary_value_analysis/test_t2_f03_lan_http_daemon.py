import tempfile
import unittest
import urllib.error
import urllib.request
from tests.e2e.e2e_harness import RealLanDaemonProcess


class TestT2F03LanHttpDaemon(unittest.TestCase):
    """
    Feature 3: Embedded LAN HTTP Daemon (Boundary Value Analysis against Real Binary)
    """

    @classmethod
    def setUpClass(cls):
        cls.server = RealLanDaemonProcess()
        cls.host, cls.port = cls.server.start()
        cls.base_url = f"http://{cls.host}:{cls.port}"

    @classmethod
    def tearDownClass(cls):
        cls.server.stop()

    def test_rapid_sequential_burst_boundary(self):
        """BVA: 20 rapid sequential requests against real daemon without socket exhaustion."""
        url = f"{self.base_url}/health"
        for _ in range(20):
            with urllib.request.urlopen(url) as resp:
                self.assertEqual(resp.status, 200)

    def test_missing_artifact_none_boundary_returns_404(self):
        """BVA: Artifact pointing to nonexistent file triggers 404 response on real daemon."""
        custom_server = RealLanDaemonProcess(artifact_path="/tmp/nonexistent_airctrl_test_boundary.exe")
        h, p = custom_server.start()
        try:
            url = f"http://{h}:{p}/airctrl.exe"
            try:
                with urllib.request.urlopen(url) as resp:
                    self.fail("Expected 404")
            except urllib.error.HTTPError as e:
                self.assertEqual(e.code, 404)
                e.close()
        finally:
            custom_server.stop()

    def test_directory_artifact_boundary_returns_404(self):
        """BVA: Artifact pointing to a directory triggers 404 response without crash."""
        with tempfile.TemporaryDirectory() as tmpdir:
            custom_server = RealLanDaemonProcess(artifact_path=tmpdir)
            h, p = custom_server.start()
            try:
                url = f"http://{h}:{p}/airctrl.exe"
                try:
                    with urllib.request.urlopen(url) as resp:
                        self.fail("Expected 404")
                except urllib.error.HTTPError as e:
                    self.assertEqual(e.code, 404)
                    e.close()
            finally:
                custom_server.stop()

    def test_empty_query_string_boundary(self):
        """BVA: URL with trailing empty query string '/run.ps1?'."""
        url = f"{self.base_url}/run.ps1?"
        with urllib.request.urlopen(url) as resp:
            self.assertEqual(resp.status, 200)

    def test_oversized_url_path_404_boundary(self):
        """BVA: Very long URL path (>500 characters) returns 404 safely."""
        long_path = "a" * 500
        url = f"{self.base_url}/{long_path}"
        try:
            with urllib.request.urlopen(url) as resp:
                self.fail("Expected 404")
        except urllib.error.HTTPError as e:
            self.assertEqual(e.code, 404)
            e.close()
