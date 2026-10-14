import unittest
from tests.e2e.e2e_harness import DocumentExporter, ExportFormat


class TestT2F53TemplatesDocExporters(unittest.TestCase):
    """
    Feature 53: Customizable Templates & Doc Exporters (Boundary Value Analysis)
    """

    def test_empty_content_export(self):
        """BVA: Exporting empty string content returns valid empty/minimal payloads."""
        md = DocumentExporter.export("", ExportFormat.MARKDOWN)
        self.assertEqual(md, b"")

        pdf = DocumentExporter.export("", ExportFormat.PDF)
        self.assertTrue(pdf.startswith(b"%PDF"))

    def test_multilingual_unicode_summary_export(self):
        """BVA: Multilingual characters (Japanese, Arabic, Cyrillic) exported without encoding crash."""
        text = "# 会議の概要 (Overview)\n- Точка обсуждения\n- نقطة نقاش"
        md_bytes = DocumentExporter.export(text, ExportFormat.MARKDOWN)
        self.assertEqual(md_bytes.decode("utf-8"), text)

    def test_pdf_trailer_and_eof_markers(self):
        """BVA: PDF output contains mandatory %%EOF closing marker."""
        pdf_bytes = DocumentExporter.export("Test summary", ExportFormat.PDF)
        self.assertTrue(pdf_bytes.endswith(b"%%EOF\n"))

    def test_docx_pk_magic_bytes_boundary(self):
        """BVA: DOCX output verifies exact ZIP container magic bytes PK\\x03\\x04."""
        docx_bytes = DocumentExporter.export("Summary text", ExportFormat.DOCX)
        self.assertEqual(docx_bytes[:4], b"PK\x03\x04")

    def test_unsupported_format_raises_value_error(self):
        """BVA: Passing unsupported format string raises ValueError."""
        with self.assertRaises(ValueError):
            DocumentExporter.export("content", "EPUB")
