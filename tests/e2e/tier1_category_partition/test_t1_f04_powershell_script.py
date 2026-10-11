import unittest
import urllib.request
from tests.e2e.e2e_harness import RealLanDaemonProcess, PowerShellValidator


class TestT1F04PowershellScript(unittest.TestCase):
    """
    Feature 4: PowerShell Deployment Script (Equivalence Partitions against Real Output)
    """

    @classmethod
    def setUpClass(cls):
        cls.server = RealLanDaemonProcess()
        cls.host, cls.port = cls.server.start()
        with urllib.request.urlopen(f"http://{cls.host}:{cls.port}/run.ps1") as resp:
            cls.script_content = resp.read().decode("utf-8")

    @classmethod
    def tearDownClass(cls):
        cls.server.stop()

    def test_run_ps1_syntax_validation(self):
        """Partition: Strict PowerShell AST and lexical bracket balancing."""
        val = PowerShellValidator.validate(self.script_content)
        self.assertTrue(val["valid"], f"Script syntax errors: {val['errors']}")
        self.assertTrue(val["balanced_brackets"])

    def test_run_ps1_contains_host_ip_and_port(self):
        """Partition: Host IP and port interpolation in deployment script."""
        self.assertIn(f'$Port = {self.port}', self.script_content)
        self.assertIn('$BinaryName = "airctrl.exe"', self.script_content)

    def test_run_ps1_sha256_cache_checking(self):
        """Partition: Script includes Get-FileHash SHA256 caching logic."""
        self.assertIn("Get-FileHash", self.script_content)
        self.assertIn("Algorithm SHA256", self.script_content)

    def test_run_ps1_terminates_existing_process(self):
        """Partition: Script includes process termination logic before overwrite."""
        self.assertIn('Stop-Process -Name "airctrl"', self.script_content)
        self.assertIn('-ErrorAction SilentlyContinue', self.script_content)

    def test_run_ps1_atomic_download_and_launch(self):
        """Partition: Script downloads to temporary file before atomic move and execution."""
        self.assertIn("airctrl.exe.tmp", self.script_content)
        self.assertIn("Start-Process", self.script_content)
