import subprocess
import unittest
from tests.e2e.e2e_harness import PeArtifactInspector, REPO_ROOT


class TestT1F05CrossCompilation(unittest.TestCase):
    """
    Feature 5: Windows Cross-Compilation Pipeline (Equivalence Partitions against Real Artifact)
    """

    @classmethod
    def setUpClass(cls):
        cls.pe_info = PeArtifactInspector.inspect()

    def test_pe_artifact_dos_and_pe_signatures(self):
        """Partition: Artifact contains valid DOS 'MZ' and PE signature."""
        self.assertEqual(self.pe_info["dos_magic"], "MZ")
        self.assertGreater(self.pe_info["e_lfanew"], 0)

    def test_pe_machine_is_x86_64(self):
        """Partition: Machine header matches IMAGE_FILE_MACHINE_AMD64 (0x8664)."""
        self.assertTrue(self.pe_info["is_x86_64"])
        self.assertEqual(self.pe_info["machine"], 0x8664)

    def test_pe_optional_header_pe32_plus_gui(self):
        """Partition: Optional Header is PE32+ (0x020B) with Windows GUI subsystem (2)."""
        self.assertTrue(self.pe_info["is_pe32_plus"])
        self.assertTrue(self.pe_info["is_gui"])
        self.assertEqual(self.pe_info["subsystem"], 2)

    def test_artifact_sha256_checksum_verification(self):
        """Partition: Cryptographic SHA256 digest matches dist/airctrl.exe.sha256."""
        self.assertTrue(self.pe_info["sha_matches"])
        self.assertEqual(len(self.pe_info["sha256"]), 64)

    def test_cross_compilation_verification_script(self):
        """Partition: scripts/cross-compile-windows.sh --verify-only exits 0."""
        res = subprocess.run(
            ["bash", "scripts/cross-compile-windows.sh", "--verify-only"],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True
        )
        self.assertEqual(res.returncode, 0, f"Verify script failed: {res.stderr}")
        self.assertIn("PE header validation PASSED", res.stdout)
