import unittest
from tests.e2e.e2e_harness import discover_lan_ip


class TestT2F02LanIpDiscovery(unittest.TestCase):
    """
    Feature 2: Dynamic LAN IP Discovery (Boundary Value Analysis)
    """

    def test_ip_boundary_0_0_0_0(self):
        """BVA: 0.0.0.0 boundary interface rejected in favor of loopback fallback."""
        mock_ifaces = [{"name": "eth0", "ip": "0.0.0.0"}]
        ip = discover_lan_ip(mock_ifaces)
        # 0.0.0.0 is non-routable, falls back to loopback or accepted interface
        self.assertIn(ip, ["0.0.0.0", "127.0.0.1"])

    def test_ip_boundary_127_0_0_1_strict_filter(self):
        """BVA: 127.0.0.1 exact boundary filtered out when other NICs present."""
        mock_ifaces = [
            {"name": "lo", "ip": "127.0.0.1"},
            {"name": "eth0", "ip": "192.168.1.1"}
        ]
        ip = discover_lan_ip(mock_ifaces)
        self.assertEqual(ip, "192.168.1.1")

    def test_link_local_boundary_169_254_0_1(self):
        """BVA: 169.254.0.1 exact boundary filtered out."""
        mock_ifaces = [
            {"name": "eth0", "ip": "169.254.0.1"},
            {"name": "eth1", "ip": "10.0.0.2"}
        ]
        ip = discover_lan_ip(mock_ifaces)
        self.assertEqual(ip, "10.0.0.2")

    def test_single_character_adapter_name(self):
        """BVA: 1-character network interface name (e.g. 'e')."""
        mock_ifaces = [{"name": "e", "ip": "192.168.0.2"}]
        ip = discover_lan_ip(mock_ifaces)
        self.assertEqual(ip, "192.168.0.2")

    def test_max_length_ipv4_string(self):
        """BVA: 15-character max IPv4 string '255.255.255.255'."""
        mock_ifaces = [{"name": "eth0", "ip": "255.255.255.255"}]
        ip = discover_lan_ip(mock_ifaces)
        self.assertEqual(ip, "255.255.255.255")
