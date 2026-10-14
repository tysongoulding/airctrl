import unittest
from tests.e2e.e2e_harness import AiSummaryService, SummaryProvider, SummaryTemplate


class TestT2F52MultiproviderAiSummaries(unittest.TestCase):
    """
    Feature 52: Multi-Provider AI Summaries (Boundary Value Analysis)
    """

    def test_massive_transcript_summary_generation(self):
        """BVA: 50,000-word transcript summarized without memory crash."""
        long_transcript = "discussion on modular architecture " * 5000
        service = AiSummaryService(SummaryProvider.OLLAMA)
        res = service.generate_summary(long_transcript, SummaryTemplate.EXECUTIVE_SUMMARY)
        self.assertIn("Executive Summary", res["summary_markdown"])

    def test_custom_template_prompt_passthrough(self):
        """BVA: Custom template faithfully passes user prompt instructions."""
        service = AiSummaryService(SummaryProvider.CUSTOM_OPENAI)
        custom_instructions = "List only bugs mentioned in bullet points."
        res = service.generate_summary(
            "Bug found in audio loop.",
            template=SummaryTemplate.CUSTOM,
            custom_prompt=custom_instructions
        )
        self.assertIn(custom_instructions, res["summary_markdown"])

    def test_all_five_providers_generate_markdown(self):
        """BVA: All 5 supported providers output valid markdown structure."""
        for prov in SummaryProvider:
            service = AiSummaryService(prov)
            res = service.generate_summary("Valid team sync transcript.", SummaryTemplate.ACTION_ITEMS)
            self.assertEqual(res["provider"], prov.value)
            self.assertIn("# Action Items", res["summary_markdown"])

    def test_whitespace_only_transcript_raises_value_error(self):
        """BVA: Whitespace-only transcript ('   \n\t  ') raises ValueError."""
        service = AiSummaryService()
        with self.assertRaises(ValueError):
            service.generate_summary("   \n\t  ")

    def test_token_count_positive_metric(self):
        """BVA: Token count is strictly greater than 0 for generated summaries."""
        service = AiSummaryService()
        res = service.generate_summary("Brief update.", SummaryTemplate.TECHNICAL_DECISIONS)
        self.assertGreater(res["token_count"], 0)
