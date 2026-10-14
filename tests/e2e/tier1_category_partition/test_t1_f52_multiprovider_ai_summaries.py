import unittest
from tests.e2e.e2e_harness import AiSummaryService, SummaryProvider, SummaryTemplate


class TestT1F52MultiproviderAiSummaries(unittest.TestCase):
    """
    Feature 52: Multi-Provider AI Summaries (Equivalence Partitions)
    """

    def test_ollama_local_summary_partition(self):
        """Partition: Ollama local provider generates summary with zero external calls."""
        service = AiSummaryService(SummaryProvider.OLLAMA)
        res = service.generate_summary("Meeting about project launch.", SummaryTemplate.EXECUTIVE_SUMMARY)
        self.assertEqual(res["provider"], "Ollama")
        self.assertIn("Executive Summary", res["summary_markdown"])

    def test_claude_provider_partition(self):
        """Partition: Claude provider generates structured meeting summary."""
        service = AiSummaryService(SummaryProvider.CLAUDE)
        res = service.generate_summary("Architecture discussion.", SummaryTemplate.TECHNICAL_DECISIONS)
        self.assertEqual(res["provider"], "Claude")
        self.assertIn("Technical Decisions", res["summary_markdown"])

    def test_groq_and_openrouter_partitions(self):
        """Partition: Groq and OpenRouter summary provider configurations."""
        service_groq = AiSummaryService(SummaryProvider.GROQ)
        res_groq = service_groq.generate_summary("Team sync.", SummaryTemplate.ACTION_ITEMS)
        self.assertEqual(res_groq["provider"], "Groq")

        service_or = AiSummaryService(SummaryProvider.OPENROUTER)
        res_or = service_or.generate_summary("Team sync.", SummaryTemplate.ACTION_ITEMS)
        self.assertEqual(res_or["provider"], "OpenRouter")

    def test_custom_openai_endpoint_partition(self):
        """Partition: Custom OpenAI endpoint configuration."""
        service_custom = AiSummaryService(SummaryProvider.CUSTOM_OPENAI)
        res_custom = service_custom.generate_summary("Internal review.", SummaryTemplate.EXECUTIVE_SUMMARY)
        self.assertEqual(res_custom["provider"], "CustomOpenAI")

    def test_empty_transcript_summary_error(self):
        """Partition: Empty transcript input raises ValueError."""
        service = AiSummaryService()
        with self.assertRaises(ValueError):
            service.generate_summary("")
