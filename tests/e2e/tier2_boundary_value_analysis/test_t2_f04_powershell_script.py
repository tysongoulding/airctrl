import unittest
from tests.e2e.e2e_harness import generate_run_ps1


class TestT2F04PowershellScript(unittest.TestCase):
    """
    Feature 4: PowerShell Deployment Script (Boundary Value Analysis)
    """

    def test_port_minimum_boundary_1(self):
        """BVA: Port 1 boundary in PowerShell generation."""
        script = generate_run_ps1("127.0.0.1", 1)
        self.assertIn('$Port = 1', script)

    def test_port_maximum_boundary_65535(self):
        """BVA: Port 65535 boundary in PowerShell generation."""
        script = generate_run_ps1("192.168.1.100", 65535)
        self.assertIn('$Port = 65535', script)

    def test_single_character_binary_name(self):
        """BVA: Single-character executable name 'a.exe'."""
        script = generate_run_ps1("10.0.0.1", 8080, binary_name="a.exe")
        self.assertIn('$BinaryName = "a.exe"', script)

    def test_host_ip_with_ipv6_loopback(self):
        """BVA: IPv6 localhost boundary string '[::1]'."""
        script = generate_run_ps1("[::1]", 8080)
        self.assertIn("$HostIp = '[::1]'", script)

    def test_binary_name_with_spaces_and_hyphens(self):
        """BVA: Binary name with hyphenated characters 'air-ctrl_preview.exe'."""
        script = generate_run_ps1("192.168.1.5", 8080, binary_name="air-ctrl_preview.exe")
        self.assertIn('air-ctrl_preview.exe', script)
