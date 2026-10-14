//! Adversarial stress-test suite for Milestone M8: Handy 1:1 Parity
//! Specifically targeting DictionaryEngine and TranscriptHistoryStore under
//! hostile inputs, massive concurrency, boundary limits, and crash conditions.

use airctrl_stt::dictionary::DictionaryEngine;
use airctrl_stt::history::TranscriptHistoryStore;
use airctrl_stt::types::{DictionaryRule, TranscriptItem};
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

// ============================================================================
// 1. DictionaryEngine: Complex Regex Patterns & Capture Groups
// ============================================================================

#[test]
fn test_adversarial_dictionary_nested_regex_and_reordered_capture_groups() {
    let rules = vec![
        // Rule with nested capture groups and reordering: "Smith, John" -> "John Smith"
        DictionaryRule::regex(r"\b([A-Z][a-z]+),\s*([A-Z][a-z]+)\b", "$2 $1"),
        // Rule with multiple capture groups and numeric formatting: "issue 1234 on github" -> "GH-1234 (github)"
        DictionaryRule::regex(r"\bissue\s+#?(\d+)\s+on\s+([a-z]+)\b", "$2-issue-$1"),
        // Rule with dollar signs and literal dollar substitution: "50 dollars" -> "$50"
        DictionaryRule::regex(r"\b(\d+)\s+dollars?\b", "$$$1"),
    ];

    let engine = DictionaryEngine::new(rules, false);

    assert_eq!(
        engine.replace_words("Author: Goulding, Tyson was the lead"),
        "Author: Tyson Goulding was the lead"
    );

    assert_eq!(
        engine.replace_words("Check issue 42 on github now"),
        "Check github-issue-42 now"
    );

    assert_eq!(
        engine.replace_words("That coffee costs 5 dollars yesterday"),
        "That coffee costs $5 yesterday"
    );
}

#[test]
fn test_adversarial_dictionary_malformed_regex_graceful_degradation() {
    let rules = vec![
        // Unclosed paren
        DictionaryRule::regex(r"(unclosed group", "replacement"),
        // Unclosed character class
        DictionaryRule::regex(r"[a-z", "replacement"),
        // Valid rule should still work!
        DictionaryRule::literal("valid trigger", "good"),
    ];

    // Engine creation must NOT panic
    let engine = DictionaryEngine::new(rules, false);

    // Malformed regexes must be safely skipped, while valid rule applies
    let res = engine.replace_words("this has a valid trigger and (unclosed group");
    assert_eq!(res, "this has a good and (unclosed group");
}

#[test]
fn test_adversarial_dictionary_overlapping_rules_and_topological_sort() {
    // Deliberately insert in reverse/conflicting order:
    // Shortest trigger first, then sub-phrases, then longest
    let rules = vec![
        DictionaryRule::literal("United", "Unified"),
        DictionaryRule::literal("United States", "US"),
        DictionaryRule::literal("United States of America", "USA"),
        DictionaryRule::literal("air", "oxygen"),
        DictionaryRule::literal("air control", "airctrl"),
        DictionaryRule::literal("air controller", "flight-director"),
    ];

    let engine = DictionaryEngine::new(rules, false);

    // Longest trigger must match first:
    // "United States of America" -> "USA" (not "Unified States of America" or "US of America")
    assert_eq!(
        engine.replace_words("Welcome to the United States of America today"),
        "Welcome to the USA today"
    );

    // Intermediate trigger:
    // "United States" -> "US" (not "Unified States")
    assert_eq!(
        engine.replace_words("Travelling across the United States this summer"),
        "Travelling across the US this summer"
    );

    // Shortest trigger alone:
    assert_eq!(
        engine.replace_words("All nations are United in peace"),
        "All nations are Unified in peace"
    );

    // Sub-token prefix isolation:
    assert_eq!(
        engine.replace_words("The air controller works on air control using fresh air"),
        "The flight-director works on airctrl using fresh oxygen"
    );
}

// ============================================================================
// 2. DictionaryEngine: Non-ASCII Unicode, Diacritics, Accents, and Emojis
// ============================================================================

#[test]
fn test_adversarial_dictionary_non_ascii_unicode_accents_and_scripts() {
    let rules = vec![
        DictionaryRule::literal("café", "coffee shop"),
        DictionaryRule::literal("résumé", "curriculum vitae"),
        DictionaryRule::literal("Über", "Above"),
        DictionaryRule::literal("привет", "hello"),
        DictionaryRule::literal("спасибо", "thank you"),
        DictionaryRule::literal("naïve", "gullible"),
    ];

    let engine = DictionaryEngine::new(rules, false);

    // Accented French words
    assert_eq!(
        engine.replace_words("Let's meet at the café to review your résumé"),
        "Let's meet at the coffee shop to review your curriculum vitae"
    );

    // Cyrillic script
    assert_eq!(
        engine.replace_words("Он сказал привет и большое спасибо"),
        "Он сказал hello и большое thank you"
    );

    // Diacritic preservation
    assert_eq!(
        engine.replace_words("Do not be so naïve"),
        "Do not be so gullible"
    );
}

// ============================================================================
// 3. DictionaryEngine: Case Preservation Permutations
// ============================================================================

#[test]
fn test_adversarial_dictionary_case_preservation_permutations() {
    let rules = vec![
        DictionaryRule::literal("air control", "airctrl"),
        DictionaryRule::literal("hyper text transfer protocol", "http"),
    ];

    let engine = DictionaryEngine::new(rules, false);

    // ALL UPPERCASE -> replacement should be UPPERCASE
    assert_eq!(
        engine.replace_words("RUNNING AIR CONTROL SYSTEM"),
        "RUNNING AIRCTRL SYSTEM"
    );
    assert_eq!(
        engine.replace_words("USING HYPER TEXT TRANSFER PROTOCOL NOW"),
        "USING HTTP NOW"
    );

    // Title Case -> replacement should be Title Case
    assert_eq!(
        engine.replace_words("Starting Air Control daemon"),
        "Starting Airctrl daemon"
    );
    assert_eq!(
        engine.replace_words("Connect over Hyper Text Transfer Protocol secure"),
        "Connect over Http secure"
    );

    // lowercase -> replacement should remain lowercase
    assert_eq!(
        engine.replace_words("starting air control daemon"),
        "starting airctrl daemon"
    );

    // When preserve_case is false: exact replacement casing is respected
    let strict_case_rules = vec![
        DictionaryRule {
            trigger: "air control".into(),
            replacement: "AirCtrl-Native".into(),
            is_regex: false,
            preserve_case: false,
        },
    ];
    let strict_engine = DictionaryEngine::new(strict_case_rules, false);

    assert_eq!(
        strict_engine.replace_words("AIR CONTROL system"),
        "AirCtrl-Native system"
    );
    assert_eq!(
        strict_engine.replace_words("air control system"),
        "AirCtrl-Native system"
    );
}

#[test]
fn test_adversarial_dictionary_post_processing_edge_cases() {
    let engine_with_post = DictionaryEngine::new(vec![], true);

    // Excessive whitespace, tabs, and newlines collapsed to single space
    assert_eq!(
        engine_with_post.replace_words("  \t\n  hello \t  world \n\n from \t airctrl  \n"),
        "Hello world from airctrl"
    );

    // Empty or whitespace-only inputs return empty string
    assert_eq!(engine_with_post.replace_words(""), "");
    assert_eq!(engine_with_post.replace_words("   \t\n   "), "");

    // Leading punctuation or symbols should not panic or corrupt leading capitalization
    assert_eq!(
        engine_with_post.replace_words("  #hashtag test  "),
        "#hashtag test"
    );
    assert_eq!(
        engine_with_post.replace_words("  123 numbers here  "),
        "123 numbers here"
    );
}

// ============================================================================
// 4. TranscriptHistoryStore: Multi-Threaded Concurrent Operations
// ============================================================================

#[test]
fn test_adversarial_transcript_history_concurrent_multi_threaded_hammer() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("transcripts_concurrent.json");
    let store = Arc::new(TranscriptHistoryStore::new(Some(file_path.clone())));

    let thread_count = 16;
    let items_per_thread = 25;
    let mut handles = Vec::new();
    let stored_counter = Arc::new(AtomicUsize::new(0));

    // Spawn concurrent writers
    for t in 0..thread_count {
        let store_clone = Arc::clone(&store);
        let counter_clone = Arc::clone(&stored_counter);

        handles.push(thread::spawn(move || {
            for i in 0..items_per_thread {
                let item = TranscriptItem {
                    id: format!("item_{}_{}", t, i),
                    timestamp: (1000 + t * 100 + i) as f64,
                    text: format!("Concurrent transcript payload {} from thread {}", i, t),
                    raw_text: format!("concurrent transcript payload {} from thread {}", i, t),
                    duration_ms: 1000 + (i as u64) * 10,
                    model_used: if t % 2 == 0 { "whisper-turbo.en".into() } else { "parakeet-v3".into() },
                    confidence: 0.95,
                    session_id: format!("session_{}", t),
                    target_app: Some(if i % 2 == 0 { "code".into() } else { "slack".into() }),
                    target_title: None,
                    injected: true,
                    starred: i == 0,
                };

                store_clone.store(item).expect("Concurrent store should never fail");
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }
        }));
    }

    // Simultaneously spawn concurrent readers executing search and count
    let reader_handles: Vec<_> = (0..4).map(|_r| {
        let store_clone = Arc::clone(&store);
        thread::spawn(move || {
            for _ in 0..50 {
                let _c = store_clone.count();
                let _res = store_clone.search("concurrent", None, 10);
                let _model_res = store_clone.search("", Some("turbo"), 10);
                thread::yield_now();
            }
        })
    }).collect();

    for h in handles {
        h.join().unwrap();
    }
    for r in reader_handles {
        r.join().unwrap();
    }

    let expected_total = thread_count * items_per_thread;
    assert_eq!(store.count(), expected_total);
    assert_eq!(stored_counter.load(Ordering::SeqCst), expected_total);

    // Verify on-disk persistence reloaded cleanly
    let reloaded = TranscriptHistoryStore::new(Some(file_path));
    assert_eq!(reloaded.count(), expected_total);

    // Verify search across populated data
    let turbo_results = reloaded.search("", Some("turbo"), expected_total);
    assert_eq!(turbo_results.len(), expected_total / 2);

    let slack_results = reloaded.search("slack", None, expected_total);
    assert!(!slack_results.is_empty());
}

// ============================================================================
// 5. TranscriptHistoryStore: Crash-Safety & Corrupted Disk Recovery
// ============================================================================

#[test]
fn test_adversarial_transcript_history_crash_safety_and_corrupt_recovery() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("corrupted_transcripts.json");

    // Write corrupted JSON to the target file simulating a hard OS crash or partial write
    fs::write(&file_path, b"{ incomplete json data [").unwrap();

    // TranscriptHistoryStore initialization must NOT panic and should fallback to empty
    let store = TranscriptHistoryStore::new(Some(file_path.clone()));
    assert_eq!(store.count(), 0);

    // Storing a new item must cleanly overwrite the corrupted file atomically
    let item = TranscriptItem {
        id: "recovered_1".into(),
        timestamp: 1000.0,
        text: "Successfully recovered after corruption".into(),
        raw_text: "successfully recovered after corruption".into(),
        duration_ms: 1500,
        model_used: "whisper-base.en".into(),
        confidence: 0.99,
        session_id: "rec_sess".into(),
        target_app: None,
        target_title: None,
        injected: false,
        starred: true,
    };
    store.store(item).unwrap();
    assert_eq!(store.count(), 1);

    // Verify reload from disk is now perfectly valid JSON
    let reloaded = TranscriptHistoryStore::new(Some(file_path));
    assert_eq!(reloaded.count(), 1);
    assert_eq!(reloaded.all()[0].id, "recovered_1");
}

// ============================================================================
// 6. TranscriptHistoryStore: Query Filtering on Large Sets & Boundary Limits
// ============================================================================

#[test]
fn test_adversarial_transcript_history_large_set_search_and_limits() {
    let store = TranscriptHistoryStore::new(None);
    let total_items = 2_000;

    for i in 0..total_items {
        let app = match i % 4 {
            0 => "vscode",
            1 => "slack",
            2 => "chrome",
            _ => "terminal",
        };
        let model = match i % 3 {
            0 => "whisper-tiny.en",
            1 => "whisper-large.en",
            _ => "parakeet-v3",
        };
        store.store(TranscriptItem {
            id: format!("tx_{:04}", i),
            timestamp: i as f64,
            text: format!("Command entry {} for testing", i),
            raw_text: format!("command entry {} for testing", i),
            duration_ms: 500,
            model_used: model.into(),
            confidence: 0.9,
            session_id: "bulk".into(),
            target_app: Some(app.into()),
            target_title: Some(format!("Window {}", app)),
            injected: true,
            starred: i % 100 == 0,
        }).unwrap();
    }

    assert_eq!(store.count(), total_items);

    // Reverse chronological ordering: latest item (tx_1999) must be first
    let latest_10 = store.search("", None, 10);
    assert_eq!(latest_10.len(), 10);
    assert_eq!(latest_10[0].id, "tx_1999");
    assert_eq!(latest_10[1].id, "tx_1998");

    // Exact limit compliance
    let limit_50 = store.search("", None, 50);
    assert_eq!(limit_50.len(), 50);

    // Filtering by target application via query
    let slack_items = store.search("slack", None, total_items);
    assert_eq!(slack_items.len(), total_items / 4);

    // Filtering by model (indices where i % 3 == 2: 2, 5, ..., 1997 -> exactly 666 items)
    let parakeet_items = store.search("", Some("parakeet"), total_items);
    assert_eq!(parakeet_items.len(), 666);

    // Non-matching query returns empty vec
    let non_existent = store.search("NonExistentString12345", None, 100);
    assert!(non_existent.is_empty());
}
