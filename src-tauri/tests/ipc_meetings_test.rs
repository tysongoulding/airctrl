use airctrl_shell::ipc::{IpcBridge, IpcError};
use serde_json::json;

#[test]
fn test_ipc_get_mixer_status() {
    let bridge = IpcBridge::default();
    let res = bridge.invoke("getMixerStatus", None).expect("getMixerStatus should succeed");

    assert_eq!(res["status"], "success");
    assert_eq!(res["sample_rate"], 16000);
    assert_eq!(res["ducking_gain"], 1.0);
    assert_eq!(res["current_envelope"], 0.0);
}

#[test]
fn test_ipc_start_and_stop_meeting_recording_lifecycle() {
    let bridge = IpcBridge::default();

    // Start recording with explicit meeting_id
    let start_res = bridge
        .invoke("startMeetingRecording", Some(json!({ "meeting_id": "test_meet_001" })))
        .expect("startMeetingRecording should succeed");

    assert_eq!(start_res["status"], "success");
    assert_eq!(start_res["recording_started"], true);
    assert_eq!(start_res["meeting_id"], "test_meet_001");
    assert!(bridge.state.is_meeting_recording.load(std::sync::atomic::Ordering::SeqCst));

    // Stop recording
    let stop_res = bridge
        .invoke("stopMeetingRecording", None)
        .expect("stopMeetingRecording should succeed");

    assert_eq!(stop_res["status"], "success");
    assert_eq!(stop_res["recording_stopped"], true);
    assert_eq!(stop_res["was_recording"], true);
    assert_eq!(stop_res["meeting_id"], "test_meet_001");
    assert!(!bridge.state.is_meeting_recording.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn test_ipc_generate_summary_templates_and_empty_transcript_rejection() {
    let bridge = IpcBridge::default();

    // Empty transcript must be rejected
    let empty_err = bridge.invoke("generateSummary", Some(json!({ "transcript": "   \n\t  " })));
    assert!(empty_err.is_err(), "Empty transcript should be rejected");

    // ActionItems template
    let res_action = bridge
        .invoke(
            "generateSummary",
            Some(json!({
                "transcript": "Alice agreed to deploy the service on Friday. Bob will review the PR.",
                "template": "ActionItems"
            })),
        )
        .expect("generateSummary ActionItems should succeed");

    assert_eq!(res_action["status"], "success");
    let summary_md = res_action["summary"]["summary_markdown"].as_str().unwrap();
    assert!(summary_md.contains("Action Items") || summary_md.contains("Tasks"));

    // ExecutiveSummary template
    let res_exec = bridge
        .invoke(
            "generateSummary",
            Some(json!({
                "transcript": "Q3 product review discussed revenue growth and user retention.",
                "template": "ExecutiveSummary"
            })),
        )
        .expect("generateSummary ExecutiveSummary should succeed");

    assert_eq!(res_exec["status"], "success");
    assert!(res_exec["summary"]["summary_markdown"].as_str().unwrap().contains("Executive Summary"));
}

#[test]
fn test_ipc_export_summary_formats_and_invalid_format_rejection() {
    let bridge = IpcBridge::default();

    // Markdown export
    let res_md = bridge
        .invoke(
            "exportSummary",
            Some(json!({
                "content": "# Test Meeting Summary\n- Key decision made",
                "format": "Markdown"
            })),
        )
        .expect("Markdown export should succeed");
    assert_eq!(res_md["status"], "success");
    assert_eq!(res_md["format"], "Markdown");
    assert!(res_md["bytes_count"].as_u64().unwrap() > 0);

    // PDF export
    let res_pdf = bridge
        .invoke(
            "exportSummary",
            Some(json!({
                "content": "Meeting summary text for PDF export",
                "format": "PDF"
            })),
        )
        .expect("PDF export should succeed");
    assert_eq!(res_pdf["status"], "success");
    assert_eq!(res_pdf["format"], "PDF");
    assert!(res_pdf["bytes_count"].as_u64().unwrap() > 0);

    // DOCX export
    let res_docx = bridge
        .invoke(
            "exportSummary",
            Some(json!({
                "content": "Meeting summary text for DOCX export",
                "format": "DOCX"
            })),
        )
        .expect("DOCX export should succeed");
    assert_eq!(res_docx["status"], "success");
    assert_eq!(res_docx["format"], "DOCX");
    assert!(res_docx["bytes_count"].as_u64().unwrap() > 0);

    // Invalid format must fail
    let res_err = bridge.invoke(
        "exportSummary",
        Some(json!({
            "content": "test",
            "format": "INVALID_FORMAT_XYZ"
        })),
    );
    assert!(res_err.is_err(), "Invalid export format must be rejected");
}

fn create_test_wav_file(dir: &std::path::Path, filename: &str) -> std::path::PathBuf {
    let file_path = dir.join(filename);
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&file_path, spec)
        .expect("failed to create test WAV writer");

    // Write 16,000 samples (1.0 second of 440Hz sine wave audio at 16kHz)
    for t in 0..16_000 {
        let sample = (t as f32 * 440.0 * 2.0 * std::f32::consts::PI / 16000.0).sin();
        writer
            .write_sample((sample * i16::MAX as f32) as i16)
            .expect("failed to write WAV sample");
    }
    writer.finalize().expect("failed to finalize test WAV file");
    file_path
}

#[test]
fn test_ipc_import_and_enhance_audio_formats() {
    let bridge = IpcBridge::default();
    let temp_dir = tempfile::tempdir().expect("create temp dir");

    // 1. Valid WAV audio import with genuine audio file on disk
    let wav_path = create_test_wav_file(temp_dir.path(), "test_recording.wav");
    let res = bridge
        .invoke(
            "importAndEnhanceAudio",
            Some(json!({ "file_path": wav_path.to_str().unwrap() })),
        )
        .expect("importAndEnhanceAudio for valid WAV should succeed");

    assert_eq!(res["status"], "success");
    assert_eq!(res["format"], "wav");
    assert_eq!(res["sample_rate"], 16000);
    assert!(res["duration_secs"].as_u64().unwrap() >= 1);
    assert!(res["transcript"].as_str().is_some());

    // 2. Non-existent file with valid extension must be rejected with ExecutionFailed
    let missing_path = temp_dir.path().join("non_existent_file.wav");
    let res_missing = bridge.invoke(
        "importAndEnhanceAudio",
        Some(json!({ "file_path": missing_path.to_str().unwrap() })),
    );
    assert!(
        matches!(res_missing, Err(IpcError::ExecutionFailed(_))),
        "Non-existent audio file must fail with ExecutionFailed, got: {:?}",
        res_missing
    );

    // 3. Unsupported audio extensions (.bin, .exe) must be rejected with InvalidArguments
    for invalid_name in &["payload.exe", "data.bin", "script.sh"] {
        let res_invalid = bridge.invoke(
            "importAndEnhanceAudio",
            Some(json!({ "file_path": format!("/audio/{}", invalid_name) })),
        );
        assert!(
            matches!(res_invalid, Err(IpcError::InvalidArguments(_))),
            "Unsupported audio extension {} must be rejected with InvalidArguments",
            invalid_name
        );
    }
}

#[test]
fn test_ipc_meeting_archive_and_seek_audio_offset() {
    let bridge = IpcBridge::default();

    // Initially archive should be queryable
    let initial_archive = bridge
        .invoke("getMeetingArchive", None)
        .expect("getMeetingArchive should succeed");
    assert_eq!(initial_archive["status"], "success");

    // Manually insert a meeting entry for search and seek testing
    let chunks = vec![
        airctrl_meetings::LiveTranscriptChunk {
            id: "chk_001".to_string(),
            start_time_ms: 0,
            end_time_ms: 3000,
            speaker: "Host".to_string(),
            text: "Welcome to the sprint retro.".to_string(),
            confidence: 0.95,
        },
        airctrl_meetings::LiveTranscriptChunk {
            id: "chk_002".to_string(),
            start_time_ms: 3000,
            end_time_ms: 7500,
            speaker: "Participant".to_string(),
            text: "Let's review the deployment pipeline.".to_string(),
            confidence: 0.92,
        },
    ];

    let entry = airctrl_meetings::MeetingArchiveEntry::new(
        "meet_sprint_retro",
        "Sprint 42 Retrospective",
        "2026-10-14T10:00:00Z",
        7,
        chunks,
        "Retrospective on deployment pipeline improvements",
        "/audio/sprint_retro.wav",
        vec!["Host".to_string(), "Participant".to_string()],
    );

    bridge.state.meeting_archive.save_meeting(entry).expect("save_meeting should succeed");

    // Search archive for "deployment"
    let search_res = bridge
        .invoke("getMeetingArchive", Some(json!({ "query": "deployment" })))
        .expect("search archive should succeed");

    assert_eq!(search_res["status"], "success");
    assert!(search_res["count"].as_u64().unwrap() >= 1);

    // Seek by chunk_id
    let seek_chunk = bridge
        .invoke(
            "seekAudioOffset",
            Some(json!({
                "meeting_id": "meet_sprint_retro",
                "chunk_id": "chk_002"
            })),
        )
        .expect("seekAudioOffset by chunk_id should succeed");

    assert_eq!(seek_chunk["status"], "success");
    assert_eq!(seek_chunk["offset_ms"], 3000);

    // Seek by offset_ms (reverse lookup)
    let seek_time = bridge
        .invoke(
            "seekAudioOffset",
            Some(json!({
                "meeting_id": "meet_sprint_retro",
                "offset_ms": 4500
            })),
        )
        .expect("seekAudioOffset by offset_ms should succeed");

    assert_eq!(seek_time["status"], "success");
    assert_eq!(seek_time["chunk"]["id"], "chk_002");
    assert!(seek_time["byte_offset"].as_u64().unwrap() > 44);
}
