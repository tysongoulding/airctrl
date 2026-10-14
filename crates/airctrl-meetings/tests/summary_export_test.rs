use airctrl_meetings::{
    interpolate_template, AiSummaryService, DocumentExporter, ExportFormat, MeetingError,
    SummaryProvider, SummaryTemplate, TemplateContext,
};

#[test]
fn test_template_variable_interpolation() {
    let ctx = TemplateContext {
        transcript: "Team agreed to use RocksDB for caching.",
        meeting_title: Some("Database Architecture Sync"),
        date: Some("2026-10-14"),
        duration: Some("45m"),
        participants: Some("Alice, Bob"),
        platform: Some("Zoom"),
    };

    let tmpl = "# {{meeting_title}} ({{date}})\nPlatform: {{platform}} | Duration: {{duration}}\nParticipants: {{participants}}\n\nNotes:\n{{transcript}}\nCustom: {{unknown_variable}}";
    let rendered = interpolate_template(tmpl, &ctx);

    assert!(rendered.contains("# Database Architecture Sync (2026-10-14)"));
    assert!(rendered.contains("Platform: Zoom | Duration: 45m"));
    assert!(rendered.contains("Participants: Alice, Bob"));
    assert!(rendered.contains("Team agreed to use RocksDB for caching."));
    // Unknown variables must be preserved verbatim
    assert!(rendered.contains("Custom: {{unknown_variable}}"));
}

#[test]
fn test_ai_summary_service_templates_and_empty_rejection() {
    let service = AiSummaryService::new(SummaryProvider::Claude);

    // Empty or whitespace-only transcript must be rejected with MeetingError::EmptyTranscript
    let err_empty = service.generate_summary("   \n\t   ", SummaryTemplate::ExecutiveSummary, None);
    assert!(matches!(err_empty, Err(MeetingError::EmptyTranscript(_))));

    // Executive Summary
    let res_exec = service
        .generate_summary("We discussed Q4 roadmaps and resource allocations.", SummaryTemplate::ExecutiveSummary, None)
        .expect("ExecutiveSummary should generate successfully");
    assert_eq!(res_exec.template, "ExecutiveSummary");
    assert!(res_exec.summary_markdown.contains("Executive Summary"));

    // Action Items
    let res_actions = service
        .generate_summary("Alice will deploy to staging. Bob will write integration tests.", SummaryTemplate::ActionItems, None)
        .expect("ActionItems should generate successfully");
    assert_eq!(res_actions.template, "ActionItems");
    assert!(res_actions.summary_markdown.contains("Action Items"));

    // Technical Decisions
    let res_decisions = service
        .generate_summary("We chose gRPC over REST for internal microservices.", SummaryTemplate::TechnicalDecisions, None)
        .expect("TechnicalDecisions should generate successfully");
    assert_eq!(res_decisions.template, "TechnicalDecisions");
    assert!(res_decisions.summary_markdown.contains("Technical Decisions"));
}

#[test]
fn test_document_exporter_markdown_pdf_docx() {
    let markdown_content = "# Meeting Summary\n\n## Overview\nMeeting concluded successfully.\n\n## Next Steps\n- Ship release candidate\n";

    // 1. Markdown Export
    let md_bytes = DocumentExporter::export(markdown_content, ExportFormat::Markdown)
        .expect("Markdown export should succeed");
    assert_eq!(md_bytes, markdown_content.as_bytes());

    // 2. PDF Export: must start with %PDF-1.4 and contain standard trailer
    let pdf_bytes = DocumentExporter::export(markdown_content, ExportFormat::Pdf)
        .expect("PDF export should succeed");
    assert!(
        pdf_bytes.starts_with(b"%PDF-1.4"),
        "PDF output must begin with %PDF-1.4 magic bytes"
    );
    let pdf_str = String::from_utf8_lossy(&pdf_bytes);
    assert!(pdf_str.contains("/Type /Catalog"), "PDF must contain Catalog object");
    assert!(pdf_str.contains("%%EOF"), "PDF must terminate with %%EOF marker");

    // 3. DOCX Export: must start with PK\x03\x04 (ZIP magic) and contain OpenXML parts
    let docx_bytes = DocumentExporter::export(markdown_content, ExportFormat::Docx)
        .expect("DOCX export should succeed");
    assert!(
        docx_bytes.starts_with(b"PK\x03\x04"),
        "DOCX output must begin with PK\\x03\\x04 ZIP magic header"
    );

    // Verify DOCX contains standard OpenXML filenames in its ZIP local headers / central directory
    assert!(
        docx_bytes.windows(b"[Content_Types].xml".len()).any(|w| w == b"[Content_Types].xml"),
        "DOCX must contain [Content_Types].xml"
    );
    assert!(
        docx_bytes.windows(b"word/document.xml".len()).any(|w| w == b"word/document.xml"),
        "DOCX must contain word/document.xml"
    );
}
