import unittest
from tests.e2e.e2e_harness import (
    AiSummaryService,
    DocumentExporter,
    ExportFormat,
    SummaryTemplate,
)


class TestT1F53TemplatesDocExporters(unittest.TestCase):
    """
    Feature 53: Customizable Templates & Doc Exporters (Equivalence Partitions)
    """

    def setUp(self):
        self.service = AiSummaryService()

    def test_executive_summary_template_partition(self):
        """Partition: Executive summary template includes overview and key points."""
        res = self.service.generate_summary("Meeting notes", SummaryTemplate.EXECUTIVE_SUMMARY)
        md = res["summary_markdown"]
        self.assertIn("# Executive Summary", md)
        self.assertIn("## Overview", md)
        self.assertIn("## Key Points", md)

    def test_action_items_template_partition(self):
        """Partition: Action items template outputs checklist with tasks and deadlines."""
        res = self.service.generate_summary("Meeting notes", SummaryTemplate.ACTION_ITEMS)
        md = res["summary_markdown"]
        self.assertIn("# Action Items", md)
        self.assertIn("- [ ]", md)

    def test_technical_decisions_template_partition(self):
        """Partition: Technical decisions template enumerates architectural choices."""
        res = self.service.generate_summary("Meeting notes", SummaryTemplate.TECHNICAL_DECISIONS)
        md = res["summary_markdown"]
        self.assertIn("# Technical Decisions", md)

    def test_export_to_markdown_and_pdf(self):
        """Partition: Exporter creates valid UTF-8 markdown and valid PDF with %PDF header."""
        sample_text = "# Executive Summary\nContent"

        md_bytes = DocumentExporter.export(sample_text, ExportFormat.MARKDOWN)
        self.assertEqual(md_bytes.decode("utf-8"), sample_text)

        pdf_bytes = DocumentExporter.export(sample_text, ExportFormat.PDF)
        self.assertTrue(pdf_bytes.startswith(b"%PDF-1.4"))

    def test_export_to_docx_partition(self):
        """Partition: Exporter creates DOCX package starting with PK zip header."""
        sample_text = "# Meeting Summary"
        docx_bytes = DocumentExporter.export(sample_text, ExportFormat.DOCX)
        self.assertTrue(docx_bytes.startswith(b"PK\x03\x04"))
