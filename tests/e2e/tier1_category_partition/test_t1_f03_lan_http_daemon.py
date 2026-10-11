import json
import unittest
import urllib.error
import urllib.request
from tests.e2e.e2e_harness import RealLanDaemonProcess


class TestT1F03LanHttpDaemon(unittest.TestCase):
    """
    Feature 3: Embedded LAN HTTP Daemon (Equivalence Partitions against Real Binary)
    """

    @classmethod
    def setUpClass(cls):
        cls.server = RealLanDaemonProcess()
        cls.host, cls.port = cls.server.start()
        cls.base_url = f"http://{cls.host}:{cls.port}"

    @classmethod
    def tearDownClass(cls):
        cls.server.stop()

    def test_run_ps1_endpoint_returns_200_and_script(self):
        """Partition: Live GET /run.ps1 endpoint from real daemon."""
        url = f"{self.base_url}/run.ps1"
        with urllib.request.urlopen(url) as resp:
            self.assertEqual(resp.status, 200)
            content_type = resp.headers.get("Content-Type", "")
            self.assertIn("text/plain", content_type)
            body = resp.read().decode("utf-8")
            self.assertIn("airctrl", body)
            self.assertIn(str(self.port), body)
            self.assertIn("Invoke-RestMethod", body)

    def test_airctrl_exe_binary_download(self):
        """Partition: Live GET /airctrl.exe binary serving matches PE32+ file."""
        url = f"{self.base_url}/airctrl.exe"
        with urllib.request.urlopen(url) as resp:
            self.assertEqual(resp.status, 200)
            content_type = resp.headers.get("Content-Type", "")
            self.assertIn("portable-executable", content_type)
            content = resp.read()
            self.assertTrue(content.startswith(b"MZ"))
            self.assertGreater(len(content), 100000)

    def test_health_check_endpoint(self):
        """Partition: Live GET /health endpoint returning JSON status."""
        url = f"{self.base_url}/health"
        with urllib.request.urlopen(url) as resp:
            self.assertEqual(resp.status, 200)
            data = json.loads(resp.read().decode("utf-8"))
            self.assertEqual(data["status"], "healthy")
            self.assertTrue(data["artifact_available"])
            self.assertEqual(data["port"], self.port)

    def test_version_metadata_endpoint(self):
        """Partition: Live GET /version endpoint from real daemon."""
        url = f"{self.base_url}/version"
        with urllib.request.urlopen(url) as resp:
            self.assertEqual(resp.status, 200)
            data = json.loads(resp.read().decode("utf-8"))
            self.assertEqual(data["version"], "0.1.0")
            self.assertEqual(data["target"], "x86_64-pc-windows-msvc")
            self.assertEqual(data["artifact_name"], "airctrl.exe")
            self.assertTrue(data["artifact_available"])

    def test_unknown_path_returns_404(self):
        """Partition: Non-existent endpoint on real daemon returns HTTP 404."""
        url = f"{self.base_url}/nonexistent-path"
        try:
            with urllib.request.urlopen(url) as resp:
                self.fail("Expected HTTP 404 error")
        except urllib.error.HTTPError as e:
            self.assertEqual(e.code, 404)
            e.close()
