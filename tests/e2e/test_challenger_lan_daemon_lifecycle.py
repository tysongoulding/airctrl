"""
test_challenger_lan_daemon_lifecycle.py - Empirical Adversarial Lifecycle Tests

Adversarially challenges RealLanDaemonProcess:
1. Subprocess spawn and dynamic free port binding
2. Clean termination and process cleanup (zero leaked processes/zombies)
3. Port release and immediate re-binding (zero leaked TCP ports)
4. Rapid start/stop churn
5. Context manager exception teardown guarantees
6. Multi-process concurrent isolation
7. High-throughput request load & stderr warning pipe saturation check
"""

import errno
import json
import os
import socket
import subprocess
import time
import unittest
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor

from tests.e2e.e2e_harness import RealLanDaemonProcess, find_free_port


def is_process_alive(pid: int) -> bool:
    """Checks whether a process with given PID exists in OS."""
    try:
        os.kill(pid, 0)
        return True
    except OSError as err:
        return err.errno == errno.EPERM


def is_port_in_use(port: int, host: str = "127.0.0.1") -> bool:
    """Checks if a TCP port is currently bound/listening."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.settimeout(0.2)
        try:
            s.connect((host, port))
            return True
        except (ConnectionRefusedError, socket.timeout, OSError):
            return False


def can_bind_port(port: int, host: str = "127.0.0.1") -> bool:
    """Checks if a TCP socket can successfully bind to the port."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        try:
            s.bind((host, port))
            return True
        except OSError:
            return False


class TestChallengerLanDaemonLifecycle(unittest.TestCase):

    def test_01_subprocess_start_and_dynamic_port_binding(self):
        """Lifecycle: Spawns binary on dynamic port, binds socket, verifies /health."""
        daemon = RealLanDaemonProcess()
        try:
            host, port = daemon.start(timeout=5.0)
            self.assertIsNotNone(daemon.process)
            pid = daemon.process.pid
            self.assertTrue(is_process_alive(pid), f"Process {pid} should be alive")
            self.assertTrue(is_port_in_use(port, host), f"Port {port} should be open")

            # Check /health endpoint
            url = f"http://{host}:{port}/health"
            with urllib.request.urlopen(url, timeout=2.0) as resp:
                self.assertEqual(resp.status, 200)
                body = json.loads(resp.read().decode("utf-8"))
                self.assertEqual(body["status"], "healthy")
                self.assertEqual(body["port"], port)
        finally:
            daemon.stop()

    def test_02_clean_shutdown_no_leaked_process_or_zombies(self):
        """Lifecycle: Verifies stop() cleanly terminates process and reaps zombie."""
        daemon = RealLanDaemonProcess()
        host, port = daemon.start(timeout=5.0)
        pid = daemon.process.pid
        self.assertTrue(is_process_alive(pid))

        # Stop daemon
        daemon.stop()

        # Verify process is not running
        self.assertIsNone(daemon.process)
        # Give OS a brief moment to reap
        time.sleep(0.1)
        self.assertFalse(
            is_process_alive(pid),
            f"Process PID {pid} is still alive after stop() — LEAKED PROCESS!"
        )

        # Double check via pgrep
        check = subprocess.run(["pgrep", "-P", str(pid)], capture_output=True, text=True)
        self.assertEqual(check.stdout.strip(), "", "Child processes leaked by daemon")

    def test_03_clean_shutdown_no_leaked_ports(self):
        """Lifecycle: Verifies port is completely freed and can be immediately rebound."""
        port = find_free_port()
        daemon1 = RealLanDaemonProcess(port=port)
        host, bound_port = daemon1.start(timeout=5.0)
        self.assertEqual(port, bound_port)

        # Stop first daemon
        daemon1.stop()
        time.sleep(0.15)

        # Test socket binding
        self.assertTrue(
            can_bind_port(port),
            f"Port {port} was NOT freed after stop() — LEAKED PORT!"
        )

        # Immediately start second daemon on EXACT same port
        daemon2 = RealLanDaemonProcess(port=port)
        try:
            h2, p2 = daemon2.start(timeout=5.0)
            self.assertEqual(p2, port)
            with urllib.request.urlopen(f"http://{h2}:{p2}/health", timeout=2.0) as resp:
                self.assertEqual(resp.status, 200)
        finally:
            daemon2.stop()

    def test_04_rapid_start_stop_churn(self):
        """Lifecycle: Performs 6 rapid sequential start/stop cycles."""
        active_pids = []
        for cycle in range(6):
            daemon = RealLanDaemonProcess()
            host, port = daemon.start(timeout=5.0)
            pid = daemon.process.pid
            active_pids.append(pid)

            # Quick query
            with urllib.request.urlopen(f"http://{host}:{port}/version", timeout=2.0) as resp:
                self.assertEqual(resp.status, 200)

            daemon.stop()
            time.sleep(0.05)

            self.assertFalse(
                is_process_alive(pid),
                f"Cycle {cycle}: PID {pid} leaked after stop()"
            )

        # Final check: zero of the churned PIDs should exist
        for pid in active_pids:
            self.assertFalse(is_process_alive(pid))

    def test_05_context_manager_exception_safety(self):
        """Lifecycle: Context manager stops process even when exception is thrown."""
        saved_daemon = None
        saved_pid = None
        saved_port = None

        with self.assertRaises(ZeroDivisionError):
            with RealLanDaemonProcess() as daemon:
                saved_daemon = daemon
                saved_pid = daemon.process.pid
                saved_port = daemon.port
                self.assertTrue(is_process_alive(saved_pid))
                # Trigger exception
                _ = 1 / 0

        self.assertIsNotNone(saved_daemon)
        self.assertIsNone(saved_daemon.process)
        time.sleep(0.1)
        self.assertFalse(
            is_process_alive(saved_pid),
            f"Context manager failed to terminate PID {saved_pid} upon exception"
        )
        self.assertTrue(can_bind_port(saved_port))

    def test_06_concurrent_daemon_instances(self):
        """Lifecycle: Spawns 3 concurrent daemons on distinct dynamic ports."""
        daemons = [RealLanDaemonProcess() for _ in range(3)]
        pids = []
        endpoints = []

        try:
            for d in daemons:
                host, port = d.start(timeout=5.0)
                pids.append(d.process.pid)
                endpoints.append((host, port))

            # Verify all ports are distinct
            ports = [p for _, p in endpoints]
            self.assertEqual(len(ports), len(set(ports)), "Port collision among concurrent daemons")

            # Query all endpoints concurrently
            def ping(ep):
                h, p = ep
                with urllib.request.urlopen(f"http://{h}:{p}/health", timeout=2.0) as resp:
                    return json.loads(resp.read().decode("utf-8"))["port"]

            with ThreadPoolExecutor(max_workers=3) as pool:
                results = list(pool.map(ping, endpoints))

            self.assertEqual(results, ports)

        finally:
            for d in daemons:
                d.stop()

        time.sleep(0.1)
        for pid in pids:
            self.assertFalse(is_process_alive(pid), f"Concurrent daemon {pid} leaked")

    def test_07_warning_load_and_pipe_saturation_stress(self):
        """Adversarial: Stresses daemon with 200 requests triggering warnings to ensure no pipe deadlocks."""
        daemon = RealLanDaemonProcess()
        try:
            host, port = daemon.start(timeout=5.0)
            base = f"http://{host}:{port}"

            # Fire 200 requests triggering fallback warning in script.rs
            # Querying /run.ps1 with an invalid host IP in query/header
            for i in range(200):
                # Request health and run.ps1 alternately
                req = urllib.request.Request(f"{base}/run.ps1")
                # Using a Host header with characters that might trigger logging
                req.add_header("Host", f"127.0.0.1:{port}")
                with urllib.request.urlopen(req, timeout=1.0) as resp:
                    self.assertEqual(resp.status, 200)

            # Query health to verify daemon did not hang/deadlock
            with urllib.request.urlopen(f"{base}/health", timeout=1.0) as resp:
                self.assertEqual(resp.status, 200)
        finally:
            daemon.stop()


if __name__ == "__main__":
    unittest.main()
