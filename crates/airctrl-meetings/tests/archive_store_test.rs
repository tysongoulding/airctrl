use airctrl_meetings::{LiveTranscriptChunk, MeetingArchiveEntry, MeetingArchiveStore};
use tempfile::tempdir;

#[test]
fn test_archive_store_save_get_list_delete() {
    let temp = tempdir().expect("tempdir");
    let store = MeetingArchiveStore::new(Some(temp.path().to_path_buf()));

    let chunks = vec![
        LiveTranscriptChunk {
            id: "chk_01".to_string(),
            start_time_ms: 0,
            end_time_ms: 2500,
            speaker: "Host".to_string(),
            text: "Sprint planning kickoff.".to_string(),
            confidence: 0.98,
        },
        LiveTranscriptChunk {
            id: "chk_02".to_string(),
            start_time_ms: 2500,
            end_time_ms: 6000,
            speaker: "Participant".to_string(),
            text: "Discussing tickets and blockers.".to_string(),
            confidence: 0.94,
        },
    ];

    let entry = MeetingArchiveEntry::new(
        "meet_retro_01",
        "Bi-weekly Retrospective",
        "2026-10-14T09:00:00Z",
        6,
        chunks,
        "# Retrospective Notes\nGood momentum on the roadmap.",
        "/path/to/audio.wav",
        vec!["Host".to_string(), "Participant".to_string()],
    );

    // Save
    store.save_meeting(entry).expect("save_meeting should succeed");

    // Get
    let fetched = store.get_meeting("meet_retro_01").expect("meeting should be found");
    assert_eq!(fetched.id, "meet_retro_01");
    assert_eq!(fetched.title, "Bi-weekly Retrospective");
    assert_eq!(fetched.transcript_chunks.len(), 2);

    // List
    let all = store.list_meetings();
    assert_eq!(all.len(), 1);

    // Delete
    let deleted = store.delete_meeting("meet_retro_01").expect("delete_meeting should succeed");
    assert!(deleted);
    assert!(store.get_meeting("meet_retro_01").is_none());
    assert_eq!(store.list_meetings().len(), 0);
}

#[test]
fn test_archive_crash_consistency_and_startup_reconciliation() {
    let temp = tempdir().expect("tempdir");
    let base_dir = temp.path().to_path_buf();

    // 1. Create a store and save an entry
    {
        let store = MeetingArchiveStore::new(Some(base_dir.clone()));
        let entry = MeetingArchiveEntry::new(
            "meet_persist_01",
            "Architecture Review",
            "2026-10-14T11:00:00Z",
            10,
            vec![LiveTranscriptChunk {
                id: "chk_p1".to_string(),
                start_time_ms: 0,
                end_time_ms: 10000,
                speaker: "Host".to_string(),
                text: "Reviewed caching layer.".to_string(),
                confidence: 0.99,
            }],
            "Caching layer approved",
            "/audio/arch.wav",
            vec!["Host".to_string()],
        );
        store.save_meeting(entry).expect("save should succeed");
    }

    // 2. Simulate a crash that left an orphaned .tmp file
    let orphaned_tmp = base_dir.join("meetings").join("meet_persist_01").join("entry.json.tmp.orphaned123");
    std::fs::write(&orphaned_tmp, b"incomplete write data").expect("write orphaned tmp");
    assert!(orphaned_tmp.exists());

    // 3. Corrupt or delete index.json to simulate an index-out-of-sync crash
    let index_file = base_dir.join("index.json");
    if index_file.exists() {
        std::fs::remove_file(&index_file).expect("remove index.json");
    }

    // 4. Restart store: load_and_reconcile should clean orphaned .tmp and reconstruct index from meetings/
    let recovered_store = MeetingArchiveStore::new(Some(base_dir.clone()));
    let recovered_entries = recovered_store.list_meetings();
    assert_eq!(recovered_entries.len(), 1, "Should reconcile and restore meeting from disk");
    assert_eq!(recovered_entries[0].id, "meet_persist_01");
    assert!(!orphaned_tmp.exists(), "Orphaned .tmp file should be cleaned up during reconciliation");
}

#[test]
fn test_archive_search_and_playback_seeking() {
    let store = MeetingArchiveStore::default();

    let chunks = vec![
        LiveTranscriptChunk {
            id: "chk_seek_1".to_string(),
            start_time_ms: 0,
            end_time_ms: 4000,
            speaker: "Host".to_string(),
            text: "First topic is database indexing.".to_string(),
            confidence: 0.96,
        },
        LiveTranscriptChunk {
            id: "chk_seek_2".to_string(),
            start_time_ms: 4000,
            end_time_ms: 8500,
            speaker: "Participant".to_string(),
            text: "Second topic is Kubernetes orchestration.".to_string(),
            confidence: 0.91,
        },
    ];

    let entry = MeetingArchiveEntry::new(
        "meet_seek_01",
        "Infrastructure Sync",
        "2026-10-14T14:00:00Z",
        8,
        chunks,
        "Covered PostgreSQL indexes and Kubernetes pods",
        "/audio/infra.wav",
        vec!["Host".to_string(), "Participant".to_string()],
    );
    store.save_meeting(entry).expect("save");

    // Full-text search
    let matches_k8s = store.search("Kubernetes");
    assert_eq!(matches_k8s.len(), 1);
    assert_eq!(matches_k8s[0].id, "meet_seek_01");

    let matches_none = store.search("nonexistent_keyword_xyz");
    assert_eq!(matches_none.len(), 0);

    // Playback seek by chunk ID
    let offset_chunk2 = store.seek_audio_offset("meet_seek_01", "chk_seek_2");
    assert_eq!(offset_chunk2, Some(4000));

    let offset_invalid = store.seek_audio_offset("meet_seek_01", "chk_missing");
    assert_eq!(offset_invalid, None);

    // Reverse lookup: chunk at offset 5500ms
    let chunk_at_5500 = store.get_chunk_at_offset("meet_seek_01", 5500);
    assert!(chunk_at_5500.is_some());
    assert_eq!(chunk_at_5500.unwrap().id, "chk_seek_2");

    // WAV byte offset calculation: 16kHz, mono (1 channel), 16-bit (2 bytes per sample)
    // 1000ms = 16,000 samples * 2 bytes = 32,000 bytes + 44 RIFF header = 32,044 bytes
    let byte_offset_1s = MeetingArchiveStore::compute_wav_byte_offset(1000, 16000, 1, 16);
    assert_eq!(byte_offset_1s, 44 + 32000);
}
