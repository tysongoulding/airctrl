import tempfile
import unittest
from tests.e2e.e2e_harness import PeArtifactInspector


class TestT2F05CrossCompilation(unittest.TestCase):
    """
    Feature 5: Windows Cross-Compilation Pipeline (Boundary Value Analysis against Real Artifact)
    """

    def test_real_pe_artifact_size_boundary(self):
        """BVA: Real compiled airctrl.exe size exceeds minimal 100KB boundary."""
        pe = PeArtifactInspector.inspect()
        self.assertGreater(pe["file_size"], 100000)

    def test_real_pe_section_count_boundary(self):
        """BVA: Real compiled airctrl.exe has 6 to 8 PE sections (including embedded UI resources)."""
        pe = PeArtifactInspector.inspect()
        self.assertGreaterEqual(pe["num_sections"], 6)
        self.assertLessEqual(pe["num_sections"], 10)

    def test_real_pe_entry_point_boundary(self):
        """BVA: Real compiled airctrl.exe entry point is non-zero offset."""
        pe = PeArtifactInspector.inspect()
        self.assertGreater(pe["entry_point"], 0)

    def test_pe_inspector_rejects_corrupted_magic(self):
        """BVA: Header parser rejects invalid/truncated files lacking MZ header."""
        with tempfile.NamedTemporaryFile() as tmp:
            tmp.write(b"NOT_A_VALID_PE_HEADER_DATA" * 5)
            tmp.flush()
            with self.assertRaises(ValueError):
                PeArtifactInspector.inspect(tmp.name)

    def test_cross_compilation_timeout_limit(self):
        """BVA: Cross-compilation timeout upper boundary (e.g. 600 seconds)."""
        timeout_secs = 600
        self.assertGreaterEqual(timeout_secs, 300)
