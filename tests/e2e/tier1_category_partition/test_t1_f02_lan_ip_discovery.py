import subprocess
import unittest
from tests.e2e.e2e_harness import discover_lan_ip, REPO_ROOT


class TestT1F02LanIpDiscovery(unittest.TestCase):
    """
    Feature 2: Dynamic LAN IP Discovery (Equivalence Partitions)
    """

    def test_real_kernel_udp_socket_inquiry(self):
        """Partition: Real zero-packet UDP socket inquiry retrieves non-loopback IP."""
        real_ip = discover_lan_ip()
        self.assertTrue(len(real_ip.split(".")) == 4)
        self.assertNotEqual(real_ip, "0.0.0.0")

    def test_rust_lan_ip_discovery_unit_test_execution(self):
        """Partition: Execute Rust airctrl-lan IP discovery test binary."""
        proc = subprocess.run(
            ["cargo", "test", "-p", "airctrl-lan", "--lib", "ip::tests::test_lan_ip_discovery_returns_valid_ip"],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True
        )
        self.assertEqual(proc.returncode, 0, f"Rust IP discovery test failed:\n{proc.stderr}")
        self.assertIn("test result: ok", proc.stdout)

    def test_physical_lan_subnet_partition(self):
        """Partition: Standard 192.168.x.x LAN interface detection."""
        mock_ifaces = [
            {"name": "lo", "ip": "127.0.0.1"},
            {"name": "ens18", "ip": "192.168.144.110"}
        ]
        ip = discover_lan_ip(mock_ifaces)
        self.assertEqual(ip, "192.168.144.110")

    def test_virtual_adapter_filter_partition(self):
        """Partition: Virtual bridge adapters (docker, tailscale) are skipped."""
        mock_ifaces = [
            {"name": "lo", "ip": "127.0.0.1"},
            {"name": "docker0", "ip": "172.17.0.1"},
            {"name": "tailscale0", "ip": "100.64.0.1"},
            {"name": "ens18", "ip": "192.168.1.50"}
        ]
        ip = discover_lan_ip(mock_ifaces)
        self.assertEqual(ip, "192.168.1.50")

    def test_offline_fallback_partition(self):
        """Partition: Empty network interface list defaults safely to 127.0.0.1."""
        ip = discover_lan_ip([])
        self.assertEqual(ip, "127.0.0.1")
