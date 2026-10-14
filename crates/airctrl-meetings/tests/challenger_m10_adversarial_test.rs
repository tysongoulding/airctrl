use airctrl_meetings::{
    interpolate_template, AiSummaryService, DocumentExporter, ExportFormat,
    LiveTranscriptChunk, MeetingArchiveEntry, MeetingArchiveStore, MeetingError,
    SummaryProvider, SummaryTemplate, TemplateContext,
};
use std::str::FromStr;
use std::sync::Arc;
use std::thread;
use tempfile::tempdir;

#[test]
fn test_adversarial_empty_transcript_rejection_fuzzing() {
    let providers = [
        SummaryProvider::Ollama,
        SummaryProvider::Claude,
        SummaryProvider::Groq,
        SummaryProvider::OpenRouter,
        SummaryProvider::CustomOpenAi,
    ];

    let templates = [
        SummaryTemplate::ExecutiveSummary,
        SummaryTemplate::ActionItems,
        SummaryTemplate::TechnicalDecisions,
        SummaryTemplate::Custom,
    ];

    let empty_payloads = [
        "",
        "   ",
        "\t",
        "\n",
        "\r\n",
        "  \t \r\n \n \t  ",
        "       \n\n\n       ",
    ];

    for provider in providers {
        let service = AiSummaryService::new(provider);
        for template in templates {
            for payload in empty_payloads {
                // Synchronous generation
                let sync_res = service.generate_summary(payload, template, None);
                assert!(
                    matches!(sync_res, Err(MeetingError::EmptyTranscript(_))),
                    "Sync generate_summary must return EmptyTranscript for payload '{:?}' on provider {:?}",
                    payload,
                    provider
                );

                // Asynchronous generation
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("tokio runtime");
                let async_res = rt.block_on(service.generate_summary_async(payload, template, None, None));
                assert!(
                    matches!(async_res, Err(MeetingError::EmptyTranscript(_))),
                    "Async generate_summary_async must return EmptyTranscript for payload '{:?}' on provider {:?}",
                    payload,
                    provider
                );
            }
        }
    }
}

#[test]
fn test_adversarial_template_interpolation_fuzzing() {
    // 1. Missing context fields default gracefully
    let empty_ctx = TemplateContext {
        transcript: "Meeting transcript content.",
        meeting_title: None,
        date: None,
        duration: None,
        participants: None,
        platform: None,
    };
    let tmpl_all = "{{meeting_title}} | {{date}} | {{duration}} | {{participants}} | {{platform}}:\n{{transcript}}";
    let rendered_defaults = interpolate_template(tmpl_all, &empty_ctx);
    assert!(rendered_defaults.contains("Untitled Meeting"));
    assert!(rendered_defaults.contains("N/A"));
    assert!(rendered_defaults.contains("Meeting"));
    assert!(rendered_defaults.contains("Meeting transcript content."));

    // 2. Unclosed and nested braces, unknown tags, format specifiers, and injection strings
    let fuzz_cases = [
        ("{{{{transcript}}}}", "{{Meeting transcript content.}}"),
        ("{{transcript", "{{transcript"),
        ("transcript}}", "transcript}}"),
        ("{{}}", "{{}}"),
        ("{{unknown_var_xyz}}", "{{unknown_var_xyz}}"),
        ("{{__proto__}}", "{{__proto__}}"),
        ("{{$env:SECRET}}", "{{$env:SECRET}}"),
        ("%s%d%x%n{:?}", "%s%d%x%n{:?}"),
        ("Robert'); DROP TABLE Meetings;--", "Robert'); DROP TABLE Meetings;--"),
        ("<script>alert('XSS')</script>", "<script>alert('XSS')</script>"),
        ("<?xml version=\"1.0\"?><!DOCTYPE foo []>", "<?xml version=\"1.0\"?><!DOCTYPE foo []>"),
        ("🦀\0\n\r\t🔥", "🦀\0\n\r\t🔥"),
    ];

    let full_ctx = TemplateContext {
        transcript: "Meeting transcript content.",
        meeting_title: Some("Security Review"),
        date: Some("2026-10-14"),
        duration: Some("30m"),
        participants: Some("Alice, Bob"),
        platform: Some("Meet"),
    };

    for (input, expected_substring) in fuzz_cases {
        let output = interpolate_template(input, &full_ctx);
        assert!(
            output.contains(expected_substring),
            "Fuzz input '{:?}' did not preserve expected substring '{:?}' in output '{:?}'",
            input,
            expected_substring,
            output
        );
    }

    // 3. Massive template string (50,000 characters)
    let large_template = "{{meeting_title}} ".repeat(2500) + "\n{{transcript}}";
    let large_output = interpolate_template(&large_template, &full_ctx);
    assert!(large_output.contains("Security Review"));
    assert!(large_output.contains("Meeting transcript content."));
    assert_eq!(large_output.matches("Security Review").count(), 2500);
}

#[test]
fn test_adversarial_pdf_exporter_binary_structure_and_escaping() {
    let adversarial_markdown = "# Executive Summary\n\
        Line with (nested (parentheses) and backslashes: \\ and \\\\)!\n\
        Line with unbalanced parens: ((((((( \n\
        Special characters: <tag> & \"quotes\" 'single' `code`\n\
        Unicode symbols: 🦀 Rust, 🚀 Rocket, ⚡ Lightning, 汉语, العربية\n\
        End of summary.\n";

    let pdf_bytes = DocumentExporter::export(adversarial_markdown, ExportFormat::Pdf)
        .expect("PDF export must succeed");

    // Magic bytes check
    assert!(
        pdf_bytes.starts_with(b"%PDF-1.4\n"),
        "PDF header must start with %PDF-1.4\\n"
    );

    let pdf_str = String::from_utf8_lossy(&pdf_bytes);

    // Structural object checks
    assert!(pdf_str.contains("1 0 obj\n<< /Type /Catalog"), "Must contain Catalog");
    assert!(pdf_str.contains("2 0 obj\n<< /Type /Pages"), "Must contain Pages");
    assert!(pdf_str.contains("3 0 obj\n<< /Type /Page"), "Must contain Page");
    assert!(pdf_str.contains("4 0 obj\n<< /Length "), "Must contain Contents stream");
    assert!(pdf_str.contains("5 0 obj\n<< /Type /Font"), "Must contain Helvetica Font");
    assert!(pdf_str.contains("xref\n0 6\n"), "Must contain standard xref table with 6 entries");
    assert!(pdf_str.contains("trailer\n<< /Size 6 /Root 1 0 R >>"), "Must contain trailer");
    assert!(pdf_str.contains("startxref\n"), "Must contain startxref");
    assert!(pdf_bytes.ends_with(b"%%EOF\n"), "PDF must terminate with %%EOF\\n");

    // Parenthesis escaping verification inside stream
    assert!(pdf_str.contains("\\(nested \\(parentheses\\)"), "Parentheses must be escaped");
    assert!(pdf_str.contains("\\\\\\\\"), "Backslashes must be escaped");

    // Multiline stress: 2,000 lines
    let massive_text = "Standard report line in meeting summary\n".repeat(2000);
    let massive_pdf = DocumentExporter::export(&massive_text, ExportFormat::Pdf)
        .expect("2000-line PDF export must succeed");
    assert!(massive_pdf.starts_with(b"%PDF-1.4\n"));
    assert!(massive_pdf.ends_with(b"%%EOF\n"));
}

#[test]
fn test_adversarial_docx_exporter_zip_structure_and_xml_escaping() {
    let adversarial_markdown = "# Title with <script> & \"quotes\" &amp; entity\n\
        ## Subtitle <unsafe>\n\
        - [ ] Task with <angle brackets> & ampersands\n\
        - Bullet point & normal text\n\
        Regular line with </w:p></w:body> injection attack\n";

    let docx_bytes = DocumentExporter::export(adversarial_markdown, ExportFormat::Docx)
        .expect("DOCX export must succeed");

    // Magic bytes check (ZIP Local File Header)
    assert!(
        docx_bytes.starts_with(b"PK\x03\x04"),
        "DOCX must begin with PK\\x03\\x04 ZIP magic header"
    );

    // Verify presence of all 3 required OpenXML members
    assert!(
        docx_bytes.windows(b"[Content_Types].xml".len()).any(|w| w == b"[Content_Types].xml"),
        "Must contain [Content_Types].xml"
    );
    assert!(
        docx_bytes.windows(b"_rels/.rels".len()).any(|w| w == b"_rels/.rels"),
        "Must contain _rels/.rels"
    );
    assert!(
        docx_bytes.windows(b"word/document.xml".len()).any(|w| w == b"word/document.xml"),
        "Must contain word/document.xml"
    );

    // Verify Central Directory signature PK\x01\x02
    assert!(
        docx_bytes.windows(4).any(|w| w == [0x50, 0x4b, 0x01, 0x02]),
        "Must contain ZIP central directory header"
    );

    // Verify End of Central Directory signature PK\x05\x06
    assert!(
        docx_bytes.windows(4).any(|w| w == [0x50, 0x4b, 0x05, 0x06]),
        "Must contain ZIP end of central directory header"
    );

    // Verify XML escaping in word/document.xml
    let docx_str = String::from_utf8_lossy(&docx_bytes);
    assert!(
        docx_str.contains("&lt;script&gt;"),
        "Angle brackets must be XML escaped to &lt; &gt;"
    );
    assert!(
        docx_str.contains("&quot;quotes&quot;"),
        "Quotes must be XML escaped to &quot;"
    );
    assert!(
        docx_str.contains("&lt;/w:p&gt;&lt;/w:body&gt;"),
        "XML tag injection must be safely escaped"
    );
    assert!(
        !docx_str.contains("<script>"),
        "Unescaped script tags must never appear in OpenXML"
    );
}

#[test]
fn test_adversarial_archive_store_concurrent_multithreaded_hammer() {
    let temp = tempdir().expect("tempdir");
    let base_dir = temp.path().to_path_buf();
    let store = Arc::new(MeetingArchiveStore::new(Some(base_dir)));

    let num_threads = 16;
    let iterations = 10;
    let mut handles = Vec::new();

    for t_id in 0..num_threads {
        let store_clone = Arc::clone(&store);
        let handle = thread::spawn(move || {
            for i in 0..iterations {
                let meet_id = format!("concurrent_meet_{}_{}", t_id, i);
                let chunk = LiveTranscriptChunk {
                    id: format!("chk_{}_{}", t_id, i),
                    start_time_ms: (i * 1000) as u64,
                    end_time_ms: ((i + 1) * 1000) as u64,
                    speaker: if t_id % 2 == 0 { "Host".to_string() } else { "Participant".to_string() },
                    text: format!("Thread {} talking about architecture topic {}", t_id, i),
                    confidence: 0.95,
                };

                let entry = MeetingArchiveEntry::new(
                    &meet_id,
                    format!("Meeting from thread {}", t_id),
                    "2026-10-14T12:00:00Z",
                    10,
                    vec![chunk],
                    format!("Summary for thread {} iter {}", t_id, i),
                    "/path/audio.wav",
                    vec!["Host".to_string(), "Participant".to_string()],
                );

                // Save
                store_clone.save_meeting(entry).expect("concurrent save must succeed");

                // Get
                let fetched = store_clone.get_meeting(&meet_id);
                assert!(fetched.is_some(), "Fetched meeting must exist immediately after save");

                // Search
                let search_res = store_clone.search(&format!("thread {}", t_id));
                assert!(!search_res.is_empty(), "Search should find thread-specific meeting");

                // List
                let all = store_clone.list_meetings();
                assert!(!all.is_empty());
            }
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().expect("Worker thread panicked during concurrent archive operations");
    }

    let all_saved = store.list_meetings();
    assert_eq!(
        all_saved.len(),
        num_threads * iterations,
        "Every thread's meetings must be preserved with exact count"
    );
}

#[test]
fn test_adversarial_archive_store_corrupt_tmp_recovery_and_startup_reconciliation() {
    let temp = tempdir().expect("tempdir");
    let base_dir = temp.path().to_path_buf();

    // 1. Seed two meetings in store
    {
        let store = MeetingArchiveStore::new(Some(base_dir.clone()));
        for id in &["meet_alpha", "meet_beta"] {
            let entry = MeetingArchiveEntry::new(
                *id,
                format!("Meeting {}", id),
                "2026-10-14T10:00:00Z",
                5,
                vec![],
                "Notes",
                "/audio.wav",
                vec!["Host".to_string()],
            );
            store.save_meeting(entry).expect("initial save");
        }
    }

    // 2. Inject corrupt / partial / binary .tmp files
    let tmp_file1 = base_dir.join("meetings").join("meet_alpha").join("entry.json.tmp.crash_alpha");
    let tmp_file2 = base_dir.join("meetings").join("meet_beta").join("entry.json.tmp.crash_beta");
    let tmp_file3 = base_dir.join("index.json.tmp.crash_index");

    std::fs::write(&tmp_file1, b"PARTIAL_JSON_TRUNCATED").expect("write tmp 1");
    std::fs::write(&tmp_file2, [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0xFF]).expect("write tmp 2");
    std::fs::write(&tmp_file3, b"").expect("write tmp 3 (empty)");

    assert!(tmp_file1.exists());
    assert!(tmp_file2.exists());
    assert!(tmp_file3.exists());

    // 3. Inject an unindexed meeting directly into disk directory
    let unindexed_dir = base_dir.join("meetings").join("meet_unindexed_gamma");
    std::fs::create_dir_all(&unindexed_dir).expect("create unindexed dir");
    let unindexed_entry = MeetingArchiveEntry::new(
        "meet_unindexed_gamma",
        "Gamma Unindexed Session",
        "2026-10-14T15:00:00Z",
        15,
        vec![],
        "Restored from disk",
        "/gamma.wav",
        vec!["Host".to_string()],
    );
    let entry_json_str = serde_json::to_string_pretty(&unindexed_entry).expect("json");
    std::fs::write(unindexed_dir.join("entry.json"), entry_json_str).expect("write unindexed entry");

    // 4. Delete the primary index.json to simulate full index loss
    let index_file = base_dir.join("index.json");
    if index_file.exists() {
        std::fs::remove_file(&index_file).expect("remove index.json");
    }

    // 5. Restart store and trigger self-healing load_and_reconcile
    let recovered_store = MeetingArchiveStore::new(Some(base_dir.clone()));
    let recovered_entries = recovered_store.list_meetings();

    // Verify .tmp files are completely cleaned up
    assert!(!tmp_file1.exists(), "Corrupt tmp_file1 should be deleted");
    assert!(!tmp_file2.exists(), "Corrupt tmp_file2 should be deleted");
    assert!(!tmp_file3.exists(), "Corrupt tmp_file3 should be deleted");

    // Verify unindexed meeting was restored and total count is 3
    assert_eq!(recovered_entries.len(), 3, "Self-healing must restore all 3 meetings from disk");
    assert!(recovered_entries.iter().any(|m| m.id == "meet_alpha"));
    assert!(recovered_entries.iter().any(|m| m.id == "meet_beta"));
    assert!(recovered_entries.iter().any(|m| m.id == "meet_unindexed_gamma"));

    // Verify index.json was successfully re-persisted
    assert!(index_file.exists(), "index.json must be regenerated by reconciliation");
}

#[test]
fn test_adversarial_archive_store_fulltext_search_regex_and_wildcard_safety() {
    let store = MeetingArchiveStore::default();

    let special_entries = [
        ("meet_re1", "Regex .* Star Meeting", "Covers .* wildcards in regex engines"),
        ("meet_re2", "Bracket [a-z0-9]+ Match", "Covers [a-z0-9]+ range expressions"),
        ("meet_re3", "Anchors ^start and end$", "Covers anchors ^ and $"),
        ("meet_re4", "Windows Path c:\\sys\\cmd.exe", "Escaped backslashes \\\\ in paths"),
        ("meet_re5", "Pricing $100.00? (USD)", "Question marks and dollar signs"),
    ];

    for (id, title, notes) in &special_entries {
        let entry = MeetingArchiveEntry::new(
            *id,
            *title,
            "2026-10-14T10:00:00Z",
            10,
            vec![],
            *notes,
            "/audio.wav",
            vec!["Host".to_string()],
        );
        store.save_meeting(entry).expect("save meeting");
    }

    // Literal substring matching with metacharacters
    let search_star = store.search(".*");
    assert_eq!(search_star.len(), 1);
    assert_eq!(search_star[0].id, "meet_re1");

    let search_bracket = store.search("[a-z0-9]+");
    assert_eq!(search_bracket.len(), 1);
    assert_eq!(search_bracket[0].id, "meet_re2");

    let search_caret = store.search("^start");
    assert_eq!(search_caret.len(), 1);
    assert_eq!(search_caret[0].id, "meet_re3");

    let search_bs = store.search("c:\\sys");
    assert_eq!(search_bs.len(), 1);
    assert_eq!(search_bs[0].id, "meet_re4");

    let search_dollar = store.search("$100.00?");
    assert_eq!(search_dollar.len(), 1);
    assert_eq!(search_dollar[0].id, "meet_re5");

    // Case-insensitivity verification
    let search_caps = store.search("STAR MEETING");
    assert_eq!(search_caps.len(), 1);
    assert_eq!(search_caps[0].id, "meet_re1");

    // Empty search returns all 5 entries
    let search_all = store.search("   ");
    assert_eq!(search_all.len(), 5);
}

#[test]
fn test_adversarial_archive_store_playback_seek_and_byte_offset() {
    // 1. WAV Byte offset calculation across varied sample rates, channels, bit depths
    // 0 ms -> 44 RIFF header
    assert_eq!(MeetingArchiveStore::compute_wav_byte_offset(0, 16000, 1, 16), 44);

    // 1000 ms at 16kHz mono 16-bit: 44 + (1000 * 16000 * 1 * 2 / 1000) = 44 + 32000 = 32044
    assert_eq!(MeetingArchiveStore::compute_wav_byte_offset(1000, 16000, 1, 16), 32044);

    // 2500 ms at 48kHz stereo 24-bit: 44 + (2500 * 48000 * 2 * 3 / 1000) = 44 + 720000 = 720044
    assert_eq!(MeetingArchiveStore::compute_wav_byte_offset(2500, 48000, 2, 24), 720044);

    // 2. Playback seek and reverse lookup with timestamp gaps
    let store = MeetingArchiveStore::default();
    let chunks = vec![
        LiveTranscriptChunk {
            id: "chk_seg1".to_string(),
            start_time_ms: 0,
            end_time_ms: 2000,
            speaker: "Host".to_string(),
            text: "Opening remarks".to_string(),
            confidence: 0.99,
        },
        // Gap of 500ms between 2000ms and 2500ms
        LiveTranscriptChunk {
            id: "chk_seg2".to_string(),
            start_time_ms: 2500,
            end_time_ms: 5000,
            speaker: "Participant".to_string(),
            text: "Followup remarks".to_string(),
            confidence: 0.92,
        },
    ];

    let entry = MeetingArchiveEntry::new(
        "meet_seek_gap",
        "Seek Gap Meeting",
        "2026-10-14T10:00:00Z",
        5,
        chunks,
        "Notes",
        "/audio.wav",
        vec!["Host".to_string()],
    );
    store.save_meeting(entry).expect("save");

    // seek_audio_offset by chunk_id
    assert_eq!(store.seek_audio_offset("meet_seek_gap", "chk_seg1"), Some(0));
    assert_eq!(store.seek_audio_offset("meet_seek_gap", "chk_seg2"), Some(2500));
    assert_eq!(store.seek_audio_offset("meet_seek_gap", "chk_unknown"), None);
    assert_eq!(store.seek_audio_offset("meet_unknown", "chk_seg1"), None);

    // get_chunk_at_offset
    assert_eq!(store.get_chunk_at_offset("meet_seek_gap", 0).unwrap().id, "chk_seg1");
    assert_eq!(store.get_chunk_at_offset("meet_seek_gap", 1999).unwrap().id, "chk_seg1");
    // Boundary end timestamp 2000 is exclusive
    assert!(store.get_chunk_at_offset("meet_seek_gap", 2000).is_none());
    // Inside gap (2200ms)
    assert!(store.get_chunk_at_offset("meet_seek_gap", 2200).is_none());
    // Chunk 2 start
    assert_eq!(store.get_chunk_at_offset("meet_seek_gap", 2500).unwrap().id, "chk_seg2");
    assert_eq!(store.get_chunk_at_offset("meet_seek_gap", 4999).unwrap().id, "chk_seg2");
    // Beyond chunk 2
    assert!(store.get_chunk_at_offset("meet_seek_gap", 5000).is_none());
    assert!(store.get_chunk_at_offset("meet_seek_gap", 99999).is_none());
}

#[test]
fn test_adversarial_archive_store_extract_match_snippet_char_boundary_resilience() {
    // 1. Clean ASCII excerpt windowing
    let ascii_text = "This is a standard English meeting transcript discussing kubernetes deployments in cluster A.";
    let snippet = MeetingArchiveStore::extract_match_snippet(ascii_text, "kubernetes");
    assert!(snippet.is_some());
    let snip_str = snippet.unwrap();
    assert!(snip_str.contains("kubernetes"));
    assert!(snip_str.starts_with("..."));

    // 2. Multibyte UTF-8 stress testing (4-byte emojis)
    let utf8_text = "🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀 target 🦀🦀🦀🦀🦀🦀🦀🦀";
    let panic_result = std::panic::catch_unwind(|| {
        MeetingArchiveStore::extract_match_snippet(utf8_text, "target")
    });
    assert!(panic_result.is_ok(), "extract_match_snippet must never panic on multi-byte UTF-8 emojis");
    let snip_utf8 = panic_result.unwrap().expect("snippet should be extracted");
    assert!(snip_utf8.contains("target"));
    assert_eq!(snip_utf8, "🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀🦀 target 🦀🦀🦀🦀🦀🦀🦀🦀");

    // 3. Emojis exceeding 40-character window (must safely produce prefix and suffix ellipses)
    let many_emojis = "🦀".repeat(80) + " target " + &"🦀".repeat(80);
    let snip_many = MeetingArchiveStore::extract_match_snippet(&many_emojis, "target").expect("many emojis snippet");
    assert!(snip_many.starts_with("..."));
    assert!(snip_many.ends_with("..."));
    assert!(snip_many.contains("target"));

    // 4. Multilingual CJK text (3-byte UTF-8 code points)
    let cjk_text = "日本語の会議録において、重要な決定事項である kubernetes クラスタの展開について議論しました。";
    let snip_cjk_k8s = MeetingArchiveStore::extract_match_snippet(cjk_text, "kubernetes").expect("cjk k8s snippet");
    assert!(snip_cjk_k8s.contains("kubernetes"));
    let snip_cjk_decision = MeetingArchiveStore::extract_match_snippet(cjk_text, "決定事項").expect("cjk decision snippet");
    assert!(snip_cjk_decision.contains("決定事項"));

    // 5. Cyrillic text (2-byte UTF-8 code points)
    let ru_text = "Привет мир и важное совещание разработчиков по архитектуре проекта";
    let snip_ru = MeetingArchiveStore::extract_match_snippet(ru_text, "совещание").expect("cyrillic snippet");
    assert!(snip_ru.contains("совещание"));

    // 6. Accented Romance language text (e.g. French café crème)
    let french_text = "Café résumé avec crème brûlée target délicieuse";
    let snip_french = MeetingArchiveStore::extract_match_snippet(french_text, "target").expect("french snippet");
    assert!(snip_french.contains("target"));

    // 7. Unicode casing disparity (Turkish dotted capital I)
    let turkish_text = "İstanBUL toplantısı target burada tamamlandı.";
    let snip_turk = MeetingArchiveStore::extract_match_snippet(turkish_text, "target").expect("turkish snippet");
    assert!(snip_turk.contains("target"));

    // 8. Boundary positions: match at index 0 and match at final character
    let start_match = MeetingArchiveStore::extract_match_snippet("target at the start of sentence", "target").unwrap();
    assert!(!start_match.starts_with("..."));
    assert!(start_match.contains("target"));

    let end_match = MeetingArchiveStore::extract_match_snippet("in the very end we find target", "target").unwrap();
    assert!(!end_match.ends_with("..."));
    assert!(end_match.contains("target"));

    // 9. Exact match with single word
    assert_eq!(MeetingArchiveStore::extract_match_snippet("target", "target").unwrap(), "target");

    // 10. Non-match, empty string, and whitespace resilience
    assert_eq!(MeetingArchiveStore::extract_match_snippet("hello world", "missing"), None);
    assert_eq!(MeetingArchiveStore::extract_match_snippet("", "query"), None);
    assert_eq!(MeetingArchiveStore::extract_match_snippet("text", ""), None);
    assert_eq!(MeetingArchiveStore::extract_match_snippet("text", "   "), None);
}

#[test]
fn test_adversarial_utf8_char_boundary_exhaustive_fuzzing() {
    // 1. Adversarial string with emojis (🦀, 🚀, 🔥), CJK (日本語のテスト, 中文会议记录),
    // Cyrillic, Turkish dotted/dotless I, Arabic (RTL), and accented Latin.
    let emoji_part = "🦀🚀🔥";
    let cjk_ja = "日本語のテスト";
    let cjk_zh = "中文会议记录";
    let cyrillic_part = "Привет мир разработчиков";
    let turkish_part = "İstanbul ışık Diyarbakır";
    let arabic_part = "تسجيل الاجتماع والملاحظات العربية";
    let accented_latin = "Café résumé déjà vu crème brûlée Übergrößenmaßstab";

    // 2. All combinations of match locations:
    // 2a. Match at byte 0 (prefix position)
    let text_start_match = format!("TARGET_START {}{}{}{}{}{}{}", emoji_part, cjk_ja, cjk_zh, cyrillic_part, turkish_part, arabic_part, accented_latin);
    let snip_start = std::panic::catch_unwind(|| {
        MeetingArchiveStore::extract_match_snippet(&text_start_match, "TARGET_START")
    }).expect("panic-free at byte 0");
    assert!(snip_start.is_some());
    assert!(snip_start.unwrap().contains("TARGET_START"));

    // 2b. Match at string end (suffix position)
    let text_end_match = format!("{}{}{}{}{}{}{} TARGET_END", emoji_part, cjk_ja, cjk_zh, cyrillic_part, turkish_part, arabic_part, accented_latin);
    let snip_end = std::panic::catch_unwind(|| {
        MeetingArchiveStore::extract_match_snippet(&text_end_match, "TARGET_END")
    }).expect("panic-free at string end");
    assert!(snip_end.is_some());
    assert!(snip_end.unwrap().contains("TARGET_END"));

    // 2c. Match surrounded immediately by 4-byte UTF-8 emojis
    let text_surrounded_4byte = "🦀🚀🔥TARGET_SURROUNDED🔥🚀🦀".to_string();
    let snip_surrounded = std::panic::catch_unwind(|| {
        MeetingArchiveStore::extract_match_snippet(&text_surrounded_4byte, "TARGET_SURROUNDED")
    }).expect("panic-free surrounded by 4-byte emojis");
    assert!(snip_surrounded.is_some());
    assert!(snip_surrounded.unwrap().contains("TARGET_SURROUNDED"));

    // 2d. Query itself is a 4-byte emoji
    for &emoji in &["🦀", "🚀", "🔥"] {
        let text_with_emoji = format!("Leading text {} trailing text", emoji);
        let res = std::panic::catch_unwind(|| {
            MeetingArchiveStore::extract_match_snippet(&text_with_emoji, emoji)
        }).expect("panic-free when query is 4-byte emoji");
        assert!(res.is_some());
        assert!(res.unwrap().contains(emoji));
    }

    // 2e. Query is CJK (日本語のテスト, 中文会议记录)
    let text_cjk = format!("Prefix text {} and {} suffix text", cjk_ja, cjk_zh);
    let snip_ja = std::panic::catch_unwind(|| {
        MeetingArchiveStore::extract_match_snippet(&text_cjk, "日本語のテスト")
    }).expect("panic-free CJK JA");
    assert!(snip_ja.is_some());
    assert!(snip_ja.unwrap().contains("日本語のテスト"));

    let snip_zh = std::panic::catch_unwind(|| {
        MeetingArchiveStore::extract_match_snippet(&text_cjk, "中文会议记录")
    }).expect("panic-free CJK ZH");
    assert!(snip_zh.is_some());
    assert!(snip_zh.unwrap().contains("中文会议记录"));

    // 2f. Query is Arabic (RTL)
    let text_arabic = format!("Prefix English {} trailing English", arabic_part);
    let snip_ar = std::panic::catch_unwind(|| {
        MeetingArchiveStore::extract_match_snippet(&text_arabic, "تسجيل الاجتماع")
    }).expect("panic-free Arabic");
    assert!(snip_ar.is_some());
    assert!(snip_ar.unwrap().contains("تسجيل الاجتماع"));

    // 2g. Query is Turkish dotted/dotless I
    let text_turkish = format!("Prefix {} suffix", turkish_part);
    let snip_tr1 = std::panic::catch_unwind(|| {
        MeetingArchiveStore::extract_match_snippet(&text_turkish, "İstanbul")
    }).expect("panic-free Turkish dotted I");
    assert!(snip_tr1.is_some());

    let snip_tr2 = std::panic::catch_unwind(|| {
        MeetingArchiveStore::extract_match_snippet(&text_turkish, "ışık")
    }).expect("panic-free Turkish dotless i");
    assert!(snip_tr2.is_some());

    // 2h. Query is Accented Latin
    let text_latin = format!("Start {} finish", accented_latin);
    let snip_lat = std::panic::catch_unwind(|| {
        MeetingArchiveStore::extract_match_snippet(&text_latin, "crème brûlée")
    }).expect("panic-free accented Latin");
    assert!(snip_lat.is_some());
    assert!(snip_lat.unwrap().contains("crème brûlée"));

    // 3. Exhaustive sliding single-character and multi-character fuzzing
    // Build an adversarial multi-script composite string
    let composite = format!(
        "🦀{}🚀{}🔥{}A{}B{}C{}D{}E",
        cjk_ja, cjk_zh, cyrillic_part, turkish_part, arabic_part, accented_latin, emoji_part
    );

    let chars: Vec<char> = composite.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_whitespace() {
            continue;
        }
        let query = ch.to_string();
        let res = std::panic::catch_unwind(|| {
            MeetingArchiveStore::extract_match_snippet(&composite, &query)
        });
        assert!(
            res.is_ok(),
            "extract_match_snippet panicked on char '{}' (U+{:04X}) at char index {}",
            ch,
            ch as u32,
            i
        );
        let opt = res.unwrap();
        assert!(
            opt.is_some(),
            "extract_match_snippet should find char '{}' present in composite string",
            ch
        );
    }

    // 4. Test 2-char window queries crossing character boundaries across composite
    for window in chars.windows(2) {
        let q: String = window.iter().collect();
        if q.trim().is_empty() {
            continue;
        }
        let res = std::panic::catch_unwind(|| {
            MeetingArchiveStore::extract_match_snippet(&composite, &q)
        });
        assert!(
            res.is_ok(),
            "extract_match_snippet panicked on 2-char window '{:?}'",
            q
        );
    }
}

#[test]
fn test_adversarial_dynamic_summary_synthesis_not_static() {
    let service = AiSummaryService::new(SummaryProvider::Ollama);

    // 1. Two completely different meeting transcripts
    let transcript_a = "The backend engineering team decided to migrate from SQLite to ScyllaDB for telemetry. \
        Alice will implement the distributed schema migrations. \
        Bob shall configure the multi-datacenter replication cluster. \
        We must guarantee sub-5ms read latency under load.";

    let transcript_b = "The security team audited the camera kill switch mechanism. \
        Charlie will write the statutory compliance audio documentation. \
        Diana shall verify the atomic CAS hardware handle release within 1ms. \
        Zero privacy leaks were observed in testing.";

    // 2. Verify ExecutiveSummary is dynamic and reflects transcript content
    let sum_a_exec = service.generate_summary(transcript_a, SummaryTemplate::ExecutiveSummary, None)
        .expect("generate summary A");
    let sum_b_exec = service.generate_summary(transcript_b, SummaryTemplate::ExecutiveSummary, None)
        .expect("generate summary B");

    // Assert outputs are NOT identical static strings
    assert_ne!(
        sum_a_exec.summary_markdown, sum_b_exec.summary_markdown,
        "Summaries for different transcripts must not be identical static strings"
    );

    // Assert summary A contains words and clauses from transcript A
    assert!(sum_a_exec.summary_markdown.contains("ScyllaDB"));
    assert!(sum_a_exec.summary_markdown.contains("Alice"));
    assert!(sum_a_exec.summary_markdown.contains("Bob"));

    // Assert summary B contains words and clauses from transcript B
    assert!(sum_b_exec.summary_markdown.contains("camera kill switch"));
    assert!(sum_b_exec.summary_markdown.contains("Charlie"));
    assert!(sum_b_exec.summary_markdown.contains("Diana"));

    // Cross-contamination checks
    assert!(!sum_a_exec.summary_markdown.contains("camera kill switch"));
    assert!(!sum_b_exec.summary_markdown.contains("ScyllaDB"));

    // Verify absence of obsolete static mock strings from earlier iterations
    let legacy_static_strings = [
        "enhanced local meeting transcript",
        "Review architectural decisions",
        "Deploy to production",
        "Weekly Team Sync",
    ];
    for legacy in &legacy_static_strings {
        assert!(
            !sum_a_exec.summary_markdown.contains(legacy),
            "Summary A must not contain legacy static string: '{}'",
            legacy
        );
        assert!(
            !sum_b_exec.summary_markdown.contains(legacy),
            "Summary B must not contain legacy static string: '{}'",
            legacy
        );
    }

    // 3. Verify ActionItems produces dynamic checklist items matching the transcript
    let sum_a_actions = service.generate_summary(transcript_a, SummaryTemplate::ActionItems, None)
        .expect("generate action items A");
    let sum_b_actions = service.generate_summary(transcript_b, SummaryTemplate::ActionItems, None)
        .expect("generate action items B");

    assert_ne!(sum_a_actions.summary_markdown, sum_b_actions.summary_markdown);
    assert!(sum_a_actions.summary_markdown.contains("- [ ]"));
    assert!(sum_b_actions.summary_markdown.contains("- [ ]"));
    assert!(sum_a_actions.summary_markdown.contains("Alice will implement"));
    assert!(sum_b_actions.summary_markdown.contains("Charlie will write"));

    // 4. Verify TechnicalDecisions produces dynamic numbered lists matching the transcript
    let sum_a_tech = service.generate_summary(transcript_a, SummaryTemplate::TechnicalDecisions, None)
        .expect("generate tech decisions A");
    let sum_b_tech = service.generate_summary(transcript_b, SummaryTemplate::TechnicalDecisions, None)
        .expect("generate tech decisions B");

    assert_ne!(sum_a_tech.summary_markdown, sum_b_tech.summary_markdown);
    assert!(sum_a_tech.summary_markdown.contains("1. "));
    assert!(sum_a_tech.summary_markdown.contains("ScyllaDB"));
    assert!(sum_b_tech.summary_markdown.contains("1. "));
    assert!(sum_b_tech.summary_markdown.contains("camera kill switch"));

    // 5. Verify Custom template incorporates transcript dynamically
    let custom_prompt = "Custom Audit Summary:\n{{transcript}}";
    let sum_custom = service.generate_summary(transcript_a, SummaryTemplate::Custom, Some(custom_prompt))
        .expect("generate custom summary");
    assert!(sum_custom.summary_markdown.contains("Custom Audit Summary"));
    assert!(sum_custom.summary_markdown.contains("ScyllaDB"));

    // 6. Verify generated_at timestamp is dynamic (not static '2026-10-14T01:00:00Z')
    let parsed_ts = chrono::DateTime::parse_from_rfc3339(&sum_a_exec.generated_at);
    assert!(
        parsed_ts.is_ok(),
        "generated_at '{}' must be valid RFC 3339 timestamp",
        sum_a_exec.generated_at
    );
    let now = chrono::Utc::now();
    let gen_utc = parsed_ts.unwrap().with_timezone(&chrono::Utc);
    let diff_secs = (now - gen_utc).num_seconds().abs();
    assert!(
        diff_secs < 60,
        "generated_at '{}' must be dynamic (within 60s of current time), diff was {}s",
        sum_a_exec.generated_at,
        diff_secs
    );
}

#[test]
fn test_adversarial_export_summary_format_validation() {
    // 1. Valid string formats parse cleanly
    assert_eq!(ExportFormat::from_str("Markdown").unwrap(), ExportFormat::Markdown);
    assert_eq!(ExportFormat::from_str("markdown").unwrap(), ExportFormat::Markdown);
    assert_eq!(ExportFormat::from_str("md").unwrap(), ExportFormat::Markdown);
    assert_eq!(ExportFormat::from_str("PDF").unwrap(), ExportFormat::Pdf);
    assert_eq!(ExportFormat::from_str("pdf").unwrap(), ExportFormat::Pdf);
    assert_eq!(ExportFormat::from_str("DOCX").unwrap(), ExportFormat::Docx);
    assert_eq!(ExportFormat::from_str("docx").unwrap(), ExportFormat::Docx);

    // 2. Invalid string formats (numeric, boolean strings, unsupported extensions) return InvalidFormat
    let invalid_format_strings = [
        "1234",
        "false",
        "true",
        "null",
        "exe",
        "json",
        "yaml",
        "csv",
        "xlsx",
        "txt",
        "",
        "   ",
        "UNKNOWN_FORMAT",
    ];
    for fmt_str in &invalid_format_strings {
        let res = ExportFormat::from_str(fmt_str);
        assert!(
            res.is_err(),
            "Format '{}' must be rejected, got: {:?}",
            fmt_str,
            res
        );
        let err_msg = res.unwrap_err();
        assert!(
            err_msg.contains("Unsupported format"),
            "Error message for '{}' must contain 'Unsupported format', got: {}",
            fmt_str,
            err_msg
        );
    }
}


