use airctrl_shell::ipc::{IpcBridge, IpcError};
use airctrl_stt::{ModelInfo, SpeechToTextEngine, TranscriptionResult};
use hound::{SampleFormat, WavSpec, WavWriter};
use serde_json::json;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::thread;
use tempfile::tempdir;

#[test]
fn test_adversarial_ipc_import_and_enhance_audio_fuzzing() {
    let bridge = IpcBridge::default();

    // 1. Missing payloads or empty argument maps
    assert!(matches!(
        bridge.invoke("importAndEnhanceAudio", None),
        Err(IpcError::InvalidArguments(_))
    ));
    assert!(matches!(
        bridge.invoke("importAndEnhanceAudio", Some(json!({}))),
        Err(IpcError::InvalidArguments(_))
    ));

    // 2. Non-string file_path types
    let non_string_payloads = [
        json!({ "file_path": 12345 }),
        json!({ "file_path": true }),
        json!({ "file_path": ["audio.wav"] }),
        json!({ "file_path": { "path": "audio.wav" } }),
        json!({ "file_path": null }),
    ];
    for p in non_string_payloads {
        assert!(
            matches!(bridge.invoke("importAndEnhanceAudio", Some(p)), Err(IpcError::InvalidArguments(_))),
            "Non-string file_path must fail with InvalidArguments"
        );
    }

    // 3. Unsupported audio extensions
    let unsupported_extensions = [
        "/audio/payload.exe",
        "/audio/script.sh",
        "/audio/data.bin",
        "/audio/notes.txt",
        "/audio/source.rs",
        "/audio/config.json",
        "/audio/archive.zip",
        "/audio/library.so",
        "/audio/no_extension",
        "/audio/trailing_dot.",
    ];
    for path in unsupported_extensions {
        let res = bridge.invoke("importAndEnhanceAudio", Some(json!({ "file_path": path })));
        assert!(
            matches!(res, Err(IpcError::InvalidArguments(_))),
            "Path '{}' with unsupported extension must return InvalidArguments",
            path
        );
    }

    // 4. Valid extensions when file does not exist on disk must return ExecutionFailed
    let valid_extensions = ["wav", "mp3", "m4a", "aac", "flac", "ogg"];
    for ext in valid_extensions {
        let nonexistent_path = format!("/audio/recording_session.{}", ext);
        let res = bridge.invoke(
            "importAndEnhanceAudio",
            Some(json!({ "file_path": nonexistent_path })),
        );
        assert!(
            matches!(res, Err(IpcError::ExecutionFailed(_))),
            "Non-existent file '{}' must fail with ExecutionFailed, got: {:?}",
            nonexistent_path,
            res
        );
    }

    // 5. Zero side-effects: meeting recording state must remain false
    assert!(!bridge.state.is_meeting_recording.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn test_adversarial_ipc_generate_summary_fuzzing() {
    let bridge = IpcBridge::default();

    // 1. Missing or empty transcript payloads
    let empty_transcript_payloads = [
        None,
        Some(json!({})),
        Some(json!({ "transcript": "" })),
        Some(json!({ "transcript": "   " })),
        Some(json!({ "transcript": "\t\n\r  \n  " })),
        Some(json!({ "transcript": 12345 })),
        Some(json!({ "transcript": true })),
        Some(json!({ "transcript": null })),
    ];

    for p in empty_transcript_payloads {
        let res = bridge.invoke("generateSummary", p);
        assert!(
            matches!(res, Err(IpcError::InvalidArguments(_))),
            "Empty or missing transcript must be rejected with InvalidArguments"
        );
    }

    // 2. Invalid template names
    let invalid_templates = [
        "NonExistentTemplate",
        "MALICIOUS_INJECTION",
        "XML_TEMPLATE",
        "123",
    ];
    for tmpl in invalid_templates {
        let res = bridge.invoke(
            "generateSummary",
            Some(json!({
                "transcript": "Discussion about backend database migrations and latency.",
                "template": tmpl
            })),
        );
        assert!(
            matches!(res, Err(IpcError::InvalidArguments(_))),
            "Invalid template '{}' must return InvalidArguments",
            tmpl
        );
    }

    // 3. Invalid provider names
    let invalid_providers = [
        "InvalidProviderXYZ",
        "ChatGPT4",
        "LocalFake",
    ];
    for prov in invalid_providers {
        let res = bridge.invoke(
            "generateSummary",
            Some(json!({
                "transcript": "Discussion about backend database migrations and latency.",
                "provider": prov
            })),
        );
        assert!(
            matches!(res, Err(IpcError::InvalidArguments(_))),
            "Invalid provider '{}' must return InvalidArguments",
            prov
        );
    }

    // 4. Valid templates & providers
    let valid_templates = ["ExecutiveSummary", "ActionItems", "TechnicalDecisions", "Custom"];
    let valid_providers = ["Ollama", "Claude", "Groq", "OpenRouter", "CustomOpenAI"];

    for tmpl in valid_templates {
        for prov in valid_providers {
            let res = bridge
                .invoke(
                    "generateSummary",
                    Some(json!({
                        "transcript": "Team decided to upgrade PostgreSQL to version 16.",
                        "template": tmpl,
                        "provider": prov
                    })),
                )
                .unwrap_or_else(|e| panic!("generateSummary for {} / {} failed: {:?}", tmpl, prov, e));

            assert_eq!(res["status"], "success");
            let summary = &res["summary"];
            assert_eq!(summary["template"], tmpl);
            assert_eq!(summary["provider"], prov);
            assert!(!summary["summary_markdown"].as_str().unwrap().is_empty());
            assert!(summary["token_count"].as_u64().unwrap() > 0);
        }
    }
}

#[test]
fn test_adversarial_ipc_export_summary_fuzzing() {
    let bridge = IpcBridge::default();

    // 1. Invalid format strings must return InvalidArguments
    let invalid_formats = ["EXE", "JSON", "YAML", "TXT", "UNKNOWN", "INVALID", ""];
    for fmt in invalid_formats {
        let res = bridge.invoke(
            "exportSummary",
            Some(json!({
                "content": "# Test content",
                "format": fmt
            })),
        );
        assert!(
            matches!(res, Err(IpcError::InvalidArguments(_))),
            "Invalid format '{}' must return InvalidArguments",
            fmt
        );
    }

    // 2. Non-string format types must return IpcError::InvalidArguments
    let non_string_format_payloads = [
        json!({ "format": 1234 }),
        json!({ "format": false }),
        json!({ "format": true }),
        json!({ "format": [1, 2, 3] }),
        json!({ "format": { "nested": "Markdown" } }),
    ];
    for payload in non_string_format_payloads {
        let non_str_res = bridge.invoke("exportSummary", Some(payload.clone()));
        assert!(
            matches!(non_str_res, Err(IpcError::InvalidArguments(_))),
            "exportSummary with non-string format '{:?}' must return InvalidArguments, got: {:?}",
            payload,
            non_str_res
        );
    }

    // 3. Valid exports to destination files with magic bytes verification
    let temp_dir = std::env::temp_dir().join(format!("airctrl_test_export_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&temp_dir).expect("create temp dir");

    let test_cases = [
        ("Markdown", "export.md", &b"# "[..]),
        ("PDF", "export.pdf", &b"%PDF-1.4\n"[..]),
        ("DOCX", "export.docx", &b"PK\x03\x04"[..]),
    ];

    for (fmt, filename, expected_magic) in test_cases {
        let dest_path = temp_dir.join(filename);
        let res = bridge
            .invoke(
                "exportSummary",
                Some(json!({
                    "content": "# Meeting Summary Title\n- Discussion bullet point\n",
                    "format": fmt,
                    "destination_path": dest_path.to_str().unwrap()
                })),
            )
            .unwrap_or_else(|e| panic!("exportSummary for {} failed: {:?}", fmt, e));

        assert_eq!(res["status"], "success");
        assert_eq!(res["format"], fmt);
        let byte_count = res["bytes_count"].as_u64().unwrap();
        assert!(byte_count > 0);

        // Verify written file on disk matches expected magic bytes
        assert!(dest_path.exists(), "Destination file must be written to disk");
        let file_bytes = std::fs::read(&dest_path).expect("read exported file");
        assert_eq!(file_bytes.len() as u64, byte_count);
        assert!(
            file_bytes.starts_with(expected_magic),
            "Exported {} file must start with expected magic bytes",
            fmt
        );
    }
}

#[test]
fn test_adversarial_ipc_seek_audio_offset_fuzzing() {
    let bridge = IpcBridge::default();

    // 1. Missing or non-string meeting_id
    let invalid_meeting_id_payloads = [
        None,
        Some(json!({})),
        Some(json!({ "chunk_id": "chk_01" })),
        Some(json!({ "meeting_id": null, "chunk_id": "chk_01" })),
        Some(json!({ "meeting_id": 12345, "chunk_id": "chk_01" })),
        Some(json!({ "meeting_id": ["meet_01"], "chunk_id": "chk_01" })),
    ];
    for p in invalid_meeting_id_payloads {
        let res = bridge.invoke("seekAudioOffset", p);
        assert!(
            matches!(res, Err(IpcError::InvalidArguments(_))),
            "Missing/non-string meeting_id must return InvalidArguments"
        );
    }

    // 2. Missing both chunk_id and offset_ms
    let missing_both_payloads = [
        json!({ "meeting_id": "meet_101" }),
        json!({ "meeting_id": "meet_101", "chunk_id": null, "offset_ms": null }),
        json!({ "meeting_id": "meet_101", "chunk_id": 123, "offset_ms": "not_a_number" }),
        json!({ "meeting_id": "meet_101", "offset_ms": -500 }),
    ];
    for p in missing_both_payloads {
        let res = bridge.invoke("seekAudioOffset", Some(p));
        assert!(
            matches!(res, Err(IpcError::InvalidArguments(_))),
            "Missing both chunk_id and offset_ms must return InvalidArguments"
        );
    }

    // 3. Non-existent meeting lookup
    let res_missing_meet_chunk = bridge
        .invoke(
            "seekAudioOffset",
            Some(json!({
                "meeting_id": "non_existent_meeting",
                "chunk_id": "chk_any"
            })),
        )
        .expect("seekAudioOffset on non-existent meeting should return NotFound response");
    assert_eq!(res_missing_meet_chunk["status"], "NotFound");
    assert_eq!(res_missing_meet_chunk["offset_ms"], serde_json::Value::Null);

    let res_missing_meet_time = bridge
        .invoke(
            "seekAudioOffset",
            Some(json!({
                "meeting_id": "non_existent_meeting",
                "offset_ms": 1500
            })),
        )
        .expect("seekAudioOffset on non-existent meeting with offset_ms should return success with null chunk");
    assert_eq!(res_missing_meet_time["status"], "success");
    assert_eq!(res_missing_meet_time["chunk"], serde_json::Value::Null);
    assert_eq!(res_missing_meet_time["byte_offset"], 44 + 48000); // 1500ms * 16000 * 2 / 1000 = 48000

    // 4. Seed a valid meeting entry into archive
    let chunks = vec![
        airctrl_meetings::LiveTranscriptChunk {
            id: "chk_ipc_1".to_string(),
            start_time_ms: 0,
            end_time_ms: 2500,
            speaker: "Host".to_string(),
            text: "Sprint retrospective start.".to_string(),
            confidence: 0.98,
        },
        airctrl_meetings::LiveTranscriptChunk {
            id: "chk_ipc_2".to_string(),
            start_time_ms: 2500,
            end_time_ms: 6000,
            speaker: "Participant".to_string(),
            text: "CI/CD pipeline test runs.".to_string(),
            confidence: 0.95,
        },
    ];

    let entry = airctrl_meetings::MeetingArchiveEntry::new(
        "meet_ipc_active",
        "Active IPC Test Meeting",
        "2026-10-14T10:00:00Z",
        6,
        chunks,
        "Meeting summary notes",
        "/audio/active.wav",
        vec!["Host".to_string(), "Participant".to_string()],
    );

    bridge.state.meeting_archive.save_meeting(entry).expect("save meeting");

    // Seek by chunk_id
    let res_chk1 = bridge
        .invoke(
            "seekAudioOffset",
            Some(json!({
                "meeting_id": "meet_ipc_active",
                "chunk_id": "chk_ipc_1"
            })),
        )
        .expect("seekAudioOffset by chunk_id");
    assert_eq!(res_chk1["status"], "success");
    assert_eq!(res_chk1["offset_ms"], 0);

    let res_chk2 = bridge
        .invoke(
            "seekAudioOffset",
            Some(json!({
                "meeting_id": "meet_ipc_active",
                "chunk_id": "chk_ipc_2"
            })),
        )
        .expect("seekAudioOffset by chunk_id");
    assert_eq!(res_chk2["status"], "success");
    assert_eq!(res_chk2["offset_ms"], 2500);

    // Seek by offset_ms
    let res_ms = bridge
        .invoke(
            "seekAudioOffset",
            Some(json!({
                "meeting_id": "meet_ipc_active",
                "offset_ms": 3200
            })),
        )
        .expect("seekAudioOffset by offset_ms");
    assert_eq!(res_ms["status"], "success");
    assert_eq!(res_ms["chunk"]["id"], "chk_ipc_2");
    assert_eq!(res_ms["byte_offset"], 44 + 102400); // 3200 * 16000 * 2 / 1000 = 102400
}

#[test]
fn test_adversarial_ipc_concurrency_and_transactional_safety() {
    let bridge = Arc::new(IpcBridge::default());
    let num_threads = 16;
    let iterations = 20;
    let mut handles = Vec::new();

    for t_id in 0..num_threads {
        let bridge_clone = Arc::clone(&bridge);
        let handle = thread::spawn(move || {
            for i in 0..iterations {
                // 1. generateSummary valid call
                let sum_ok = bridge_clone.invoke(
                    "generateSummary",
                    Some(json!({
                        "transcript": format!("Thread {} iter {} transcript", t_id, i),
                        "template": "ExecutiveSummary"
                    })),
                );
                assert!(sum_ok.is_ok());

                // 2. generateSummary invalid call
                let sum_bad = bridge_clone.invoke("generateSummary", Some(json!({ "transcript": "   " })));
                assert!(matches!(sum_bad, Err(IpcError::InvalidArguments(_))));

                // 3. exportSummary valid call
                let exp_ok = bridge_clone.invoke(
                    "exportSummary",
                    Some(json!({
                        "content": "Concurrent summary export",
                        "format": "Markdown"
                    })),
                );
                assert!(exp_ok.is_ok());

                // 4. exportSummary invalid call
                let exp_bad = bridge_clone.invoke(
                    "exportSummary",
                    Some(json!({
                        "content": "Test",
                        "format": "BAD_FORMAT"
                    })),
                );
                assert!(matches!(exp_bad, Err(IpcError::InvalidArguments(_))));

                // 5. importAndEnhanceAudio non-existent file call correctly returns ExecutionFailed
                let imp_missing = bridge_clone.invoke(
                    "importAndEnhanceAudio",
                    Some(json!({ "file_path": format!("/audio/t{}_i{}.wav", t_id, i) })),
                );
                assert!(matches!(imp_missing, Err(IpcError::ExecutionFailed(_))));

                // 6. importAndEnhanceAudio invalid call
                let imp_bad = bridge_clone.invoke(
                    "importAndEnhanceAudio",
                    Some(json!({ "file_path": format!("/audio/t{}_i{}.bad_ext", t_id, i) })),
                );
                assert!(matches!(imp_bad, Err(IpcError::InvalidArguments(_))));

                // 7. seekAudioOffset invalid call
                let seek_bad = bridge_clone.invoke("seekAudioOffset", Some(json!({})));
                assert!(matches!(seek_bad, Err(IpcError::InvalidArguments(_))));

                // 8. getMeetingArchive call
                let arch_ok = bridge_clone.invoke("getMeetingArchive", None);
                assert!(arch_ok.is_ok());
            }
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().expect("Thread panicked during concurrent IPC bridge hammer");
    }

    // Verify system state remains stable
    assert!(!bridge.state.is_meeting_recording.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn test_adversarial_ipc_import_zero_byte_and_truncated_files() {
    let bridge = IpcBridge::default();
    let temp = tempdir().expect("create tempdir");

    // 1. Zero-byte files across supported extensions (.wav, .mp3, .flac, .ogg)
    for ext in &["wav", "mp3", "flac", "ogg"] {
        let zero_file = temp.path().join(format!("empty_audio.{}", ext));
        File::create(&zero_file).expect("create zero-byte file");

        let res = bridge.invoke(
            "importAndEnhanceAudio",
            Some(json!({ "file_path": zero_file.to_str().unwrap() })),
        );
        match res {
            Err(IpcError::ExecutionFailed(msg)) => {
                assert!(
                    msg.contains("Empty audio file"),
                    "Zero-byte .{} error message must indicate empty audio file, got: {}",
                    ext,
                    msg
                );
            }
            other => panic!(
                "Zero-byte .{} must return IpcError::ExecutionFailed with empty file message, got: {:?}",
                ext, other
            ),
        }
    }

    // 2. Corrupt/Truncated WAV files
    // 2.1 Only 4 bytes (RIFF magic)
    let trunc_4 = temp.path().join("trunc_4.wav");
    let mut f4 = File::create(&trunc_4).expect("create trunc_4");
    f4.write_all(b"RIFF").expect("write trunc_4");
    drop(f4);

    let res_t4 = bridge.invoke(
        "importAndEnhanceAudio",
        Some(json!({ "file_path": trunc_4.to_str().unwrap() })),
    );
    assert!(
        matches!(res_t4, Err(IpcError::ExecutionFailed(_))),
        "4-byte WAV must fail with ExecutionFailed, got: {:?}",
        res_t4
    );

    // 2.2 Incomplete header (20 bytes)
    let trunc_20 = temp.path().join("trunc_20.wav");
    let mut f20 = File::create(&trunc_20).expect("create trunc_20");
    f20.write_all(b"RIFF\x24\x00\x00\x00WAVEfmt \x10\x00\x00\x00").expect("write trunc_20");
    drop(f20);

    let res_t20 = bridge.invoke(
        "importAndEnhanceAudio",
        Some(json!({ "file_path": trunc_20.to_str().unwrap() })),
    );
    assert!(
        matches!(res_t20, Err(IpcError::ExecutionFailed(_))),
        "Incomplete 20-byte WAV header must fail with ExecutionFailed, got: {:?}",
        res_t20
    );

    // 2.3 Non-audio garbage disguised as WAV
    let garbage_wav = temp.path().join("garbage.wav");
    let mut fg = File::create(&garbage_wav).expect("create garbage_wav");
    fg.write_all(b"Plain text payload pretending to be a RIFF audio container").expect("write garbage");
    drop(fg);

    let res_g = bridge.invoke(
        "importAndEnhanceAudio",
        Some(json!({ "file_path": garbage_wav.to_str().unwrap() })),
    );
    assert!(
        matches!(res_g, Err(IpcError::ExecutionFailed(_))),
        "Garbage text disguised as WAV must fail with ExecutionFailed, got: {:?}",
        res_g
    );
}

struct ProbeSttEngine {
    expected_transcript: String,
    confidence: f32,
}

impl SpeechToTextEngine for ProbeSttEngine {
    fn transcribe(&self, samples: &[f32]) -> airctrl_stt::Result<TranscriptionResult> {
        Ok(TranscriptionResult {
            text: self.expected_transcript.clone(),
            duration_ms: (samples.len() as u64 * 1000) / 16000,
            confidence: self.confidence,
            sample_count: samples.len(),
            language: Some("en".to_string()),
            segments: vec![],
        })
    }

    fn is_loaded(&self) -> bool {
        true
    }

    fn model_info(&self) -> Option<ModelInfo> {
        Some(ModelInfo {
            variant: airctrl_stt::WhisperModelVariant::SmallEn,
            file_path: std::path::PathBuf::from("/models/probe-model.bin"),
            size_bytes: 1024,
            format: airctrl_stt::ModelFormat::GgmlBin,
            sha256: "probe_sha256".to_string(),
            is_verified: true,
        })
    }
}

#[test]
fn test_adversarial_ipc_import_real_wav_uses_app_state_engine() {
    let probe_text = "EMPIRICAL_PROBE_STT_ENGINE_TRANSCRIPT_XYZ987";
    let probe_engine = ProbeSttEngine {
        expected_transcript: probe_text.to_string(),
        confidence: 0.97,
    };
    let bridge = IpcBridge::with_engine(Box::new(probe_engine));

    let temp = tempdir().expect("create tempdir");
    let wav_path = temp.path().join("genuine_probe_test.wav");

    // Synthesize valid 16kHz mono 16-bit WAV containing 1.0 second of audio
    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(&wav_path, spec).expect("create WavWriter");
    for t in 0..16_000 {
        let sample = (30000.0 * (t as f32 * 0.05).sin()) as i16;
        writer.write_sample(sample).expect("write sample");
    }
    writer.finalize().expect("finalize WAV");

    let res = bridge
        .invoke(
            "importAndEnhanceAudio",
            Some(json!({ "file_path": wav_path.to_str().unwrap() })),
        )
        .expect("importAndEnhanceAudio should succeed for genuine WAV");

    assert_eq!(res["status"], "success");
    assert_eq!(res["format"], "wav");
    assert_eq!(res["sample_rate"], 16000);
    assert!(res["duration_secs"].as_u64().unwrap() >= 1);

    // Verify the transcript matches the ProbeSttEngine output exactly
    assert_eq!(
        res["transcript"], probe_text,
        "IPC handler must use AppState.stt_engine rather than hardcoded mock strings"
    );

    let segments = res["segments"].as_array().expect("segments array");
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0]["text"], probe_text);
    assert!((segments[0]["confidence"].as_f64().unwrap() - 0.97).abs() < 1e-4);
}

#[test]
fn test_adversarial_ipc_import_multi_bit_depth_and_stereo_wav() {
    let bridge = IpcBridge::default();
    let temp = tempdir().expect("create tempdir");

    // 1. Stereo 16-bit PCM WAV (should be downmixed to mono)
    let stereo_path = temp.path().join("stereo_test.wav");
    let stereo_spec = WavSpec {
        channels: 2,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut stereo_writer = WavWriter::create(&stereo_path, stereo_spec).expect("stereo writer");
    for t in 0..16_000 {
        let l = (25000.0 * (t as f32 * 0.03).sin()) as i16;
        let r = (25000.0 * (t as f32 * 0.03).cos()) as i16;
        stereo_writer.write_sample(l).expect("write l");
        stereo_writer.write_sample(r).expect("write r");
    }
    stereo_writer.finalize().expect("finalize stereo");

    let res_stereo = bridge
        .invoke(
            "importAndEnhanceAudio",
            Some(json!({ "file_path": stereo_path.to_str().unwrap() })),
        )
        .expect("Stereo WAV import must succeed");
    assert_eq!(res_stereo["status"], "success");
    assert_eq!(res_stereo["format"], "wav");
    assert_eq!(res_stereo["sample_rate"], 16000);

    // 2. 24-bit Integer PCM WAV
    let wav24_path = temp.path().join("wav24_test.wav");
    let wav24_spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 24,
        sample_format: SampleFormat::Int,
    };
    let mut w24 = WavWriter::create(&wav24_path, wav24_spec).expect("24-bit writer");
    for t in 0..16_000 {
        let val = (7_000_000.0 * (t as f32 * 0.05).sin()) as i32;
        w24.write_sample(val).expect("write 24-bit");
    }
    w24.finalize().expect("finalize 24-bit");

    let res_24 = bridge
        .invoke(
            "importAndEnhanceAudio",
            Some(json!({ "file_path": wav24_path.to_str().unwrap() })),
        )
        .expect("24-bit WAV import must succeed");
    assert_eq!(res_24["status"], "success");
    assert_eq!(res_24["format"], "wav");

    // 3. 32-bit Float WAV
    let float_path = temp.path().join("float32_test.wav");
    let float_spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut wf = WavWriter::create(&float_path, float_spec).expect("float writer");
    for t in 0..16_000 {
        let val = 0.8 * (t as f32 * 0.05).sin();
        wf.write_sample(val).expect("write float");
    }
    wf.finalize().expect("finalize float");

    let res_float = bridge
        .invoke(
            "importAndEnhanceAudio",
            Some(json!({ "file_path": float_path.to_str().unwrap() })),
        )
        .expect("32-bit float WAV import must succeed");
    assert_eq!(res_float["status"], "success");
    assert_eq!(res_float["format"], "wav");
}

