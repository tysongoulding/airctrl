use airctrl_shell::ipc::{IpcBridge, IpcError};
use serde_json::json;
use std::sync::Arc;
use std::thread;

// ============================================================================
// Milestone M12 Adversarial Stress Test Suite
// ============================================================================

/// Group 1: Audio Mixer Volumes Boundaries, Hostile Inputs & 0-Side-Effect Rollback
#[test]
fn test_adversarial_audio_mixer_volumes_boundaries_and_zero_side_effect() {
    let bridge = IpcBridge::default();

    // Establish a known baseline
    let baseline_res = bridge
        .invoke(
            "setAudioMixerVolumes",
            Some(json!({ "mic_volume": 0.75, "system_volume": 1.25 })),
        )
        .expect("Baseline set must succeed");
    assert_eq!(baseline_res["mic_volume"], 0.75);
    assert_eq!(baseline_res["system_volume"], 1.25);

    // 1. Boundary Values Exactly at 0.0 and 2.0
    let bounds_0 = bridge
        .invoke(
            "setAudioMixerVolumes",
            Some(json!({ "mic_volume": 0.0, "system_volume": 0.0 })),
        )
        .expect("0.0 volumes must succeed");
    assert_eq!(bounds_0["mic_volume"], 0.0);
    assert_eq!(bounds_0["system_volume"], 0.0);

    let bounds_2 = bridge
        .invoke(
            "set_audio_mixer_volumes",
            Some(json!({ "mic_volume": 2.0, "system_volume": 2.0 })),
        )
        .expect("2.0 volumes must succeed");
    assert_eq!(bounds_2["mic_volume"], 2.0);
    assert_eq!(bounds_2["system_volume"], 2.0);

    // Reset to baseline for error rejection tests
    bridge
        .invoke(
            "setAudioMixerVolumes",
            Some(json!({ "mic_volume": 0.75, "system_volume": 1.25 })),
        )
        .unwrap();

    // 2. Negative Volumes
    let negative_payloads = [
        json!({ "mic_volume": -0.5, "system_volume": 1.0 }),
        json!({ "mic_volume": -0.0001, "system_volume": 1.0 }),
        json!({ "mic_volume": -100.0, "system_volume": 1.0 }),
        json!({ "mic_volume": 1.0, "system_volume": -0.5 }),
        json!({ "mic_volume": 1.0, "system_volume": -0.0001 }),
        json!({ "mic_volume": 1.0, "system_volume": -100.0 }),
        json!({ "mic_volume": -1.0, "system_volume": -1.0 }),
    ];

    for p in negative_payloads {
        let err = bridge
            .invoke("setAudioMixerVolumes", Some(p.clone()))
            .unwrap_err();
        assert!(
            matches!(err, IpcError::InvalidArguments(_)),
            "Negative payload {:?} must fail with InvalidArguments, got {:?}",
            p,
            err
        );

        // Strict 0-side-effect state preservation check
        let curr = bridge.invoke("getAudioMixerVolumes", None).unwrap();
        assert_eq!(
            curr["mic_volume"], 0.75,
            "mic_volume must remain unchanged after rejected negative volume"
        );
        assert_eq!(
            curr["system_volume"], 1.25,
            "system_volume must remain unchanged after rejected negative volume"
        );
    }

    // 3. Excessive Volumes (> 2.0)
    let excessive_payloads = [
        json!({ "mic_volume": 2.0001, "system_volume": 1.0 }),
        json!({ "mic_volume": 2.1, "system_volume": 1.0 }),
        json!({ "mic_volume": 100.0, "system_volume": 1.0 }),
        json!({ "mic_volume": 1000.0, "system_volume": 1.0 }),
        json!({ "mic_volume": 1.0, "system_volume": 2.0001 }),
        json!({ "mic_volume": 1.0, "system_volume": 2.1 }),
        json!({ "mic_volume": 1.0, "system_volume": 100.0 }),
        json!({ "mic_volume": 1.0, "system_volume": 1000.0 }),
    ];

    for p in excessive_payloads {
        let err = bridge
            .invoke("setAudioMixerVolumes", Some(p.clone()))
            .unwrap_err();
        assert!(
            matches!(err, IpcError::InvalidArguments(_)),
            "Excessive payload {:?} must fail with InvalidArguments, got {:?}",
            p,
            err
        );

        // Verify zero side-effects
        let curr = bridge.invoke("getAudioMixerVolumes", None).unwrap();
        assert_eq!(curr["mic_volume"], 0.75);
        assert_eq!(curr["system_volume"], 1.25);
    }

    // 4. Non-Numeric Types & Malformed Payloads
    let non_numeric_payloads = [
        json!({ "mic_volume": "1.0", "system_volume": 1.0 }),
        json!({ "mic_volume": "NaN", "system_volume": 1.0 }),
        json!({ "mic_volume": true, "system_volume": 1.0 }),
        json!({ "mic_volume": null, "system_volume": 1.0 }),
        json!({ "mic_volume": [1.0], "system_volume": 1.0 }),
        json!({ "mic_volume": { "val": 1.0 }, "system_volume": 1.0 }),
        json!({ "mic_volume": 1.0, "system_volume": "2.0" }),
        json!({ "mic_volume": 1.0, "system_volume": "NaN" }),
        json!({ "mic_volume": 1.0, "system_volume": false }),
        json!({ "mic_volume": 1.0, "system_volume": null }),
        json!({ "mic_volume": 1.0, "system_volume": [2.0] }),
        json!({ "mic_volume": 1.0, "system_volume": { "val": 2.0 } }),
    ];

    for p in non_numeric_payloads {
        let err = bridge
            .invoke("set_audio_mixer_volumes", Some(p.clone()))
            .unwrap_err();
        assert!(
            matches!(err, IpcError::InvalidArguments(_)),
            "Non-numeric payload {:?} must fail with InvalidArguments, got {:?}",
            p,
            err
        );

        let curr = bridge.invoke("get_audio_mixer_volumes", None).unwrap();
        assert_eq!(curr["mic_volume"], 0.75);
        assert_eq!(curr["system_volume"], 1.25);
    }

    // 5. Missing Fields
    let missing_payloads = [
        None,
        Some(json!({})),
        Some(json!({ "mic_volume": 1.0 })),
        Some(json!({ "system_volume": 1.0 })),
        Some(json!({ "other_field": 1.0 })),
    ];

    for p in missing_payloads {
        let err = bridge
            .invoke("setAudioMixerVolumes", p.clone())
            .unwrap_err();
        assert!(
            matches!(err, IpcError::InvalidArguments(_)),
            "Missing fields payload {:?} must fail with InvalidArguments",
            p
        );

        let curr = bridge.invoke("getAudioMixerVolumes", None).unwrap();
        assert_eq!(curr["mic_volume"], 0.75);
        assert_eq!(curr["system_volume"], 1.25);
    }

    // 6. Transactional Partial Failure: Valid mic (1.8) but invalid sys (2.5)
    // Must NOT mutate mic_volume to 1.8!
    let partial_err = bridge
        .invoke(
            "setAudioMixerVolumes",
            Some(json!({ "mic_volume": 1.8, "system_volume": 2.5 })),
        )
        .unwrap_err();
    assert!(matches!(partial_err, IpcError::InvalidArguments(_)));

    let curr_after_partial = bridge.invoke("getAudioMixerVolumes", None).unwrap();
    assert_eq!(
        curr_after_partial["mic_volume"], 0.75,
        "mic_volume must not be mutated when system_volume validation fails"
    );
    assert_eq!(curr_after_partial["system_volume"], 1.25);
}

/// Group 2: Focus Window Adversarial & Boundary Edge Cases
#[test]
fn test_adversarial_focus_window_edge_cases() {
    let bridge = IpcBridge::default();

    // 1. Missing payload or empty object
    assert!(matches!(
        bridge.invoke("focusWindow", None),
        Err(IpcError::InvalidArguments(_))
    ));
    assert!(matches!(
        bridge.invoke("focus_window", Some(json!({}))),
        Err(IpcError::InvalidArguments(_))
    ));

    // 2. Empty string & whitespace strings
    let invalid_strings = [
        "",
        " ",
        "   ",
        "\t",
        "\n",
        "\r\n",
        " \t\n\r ",
    ];

    for s in invalid_strings {
        let err1 = bridge
            .invoke("focusWindow", Some(json!({ "app": s })))
            .unwrap_err();
        assert!(
            matches!(err1, IpcError::InvalidArguments(_)),
            "Whitespace string {:?} must fail with InvalidArguments",
            s
        );

        let err2 = bridge
            .invoke("focus_window", Some(json!({ "name": s })))
            .unwrap_err();
        assert!(
            matches!(err2, IpcError::InvalidArguments(_)),
            "Whitespace string {:?} must fail with InvalidArguments on focus_window",
            s
        );
    }

    // 3. Non-string types
    let non_string_payloads = [
        json!({ "app": 12345 }),
        json!({ "app": true }),
        json!({ "app": false }),
        json!({ "app": null }),
        json!({ "app": ["Slack"] }),
        json!({ "app": { "target": "Slack" } }),
        json!({ "name": 999 }),
        json!({ "name": true }),
        json!({ "name": null }),
    ];

    for p in non_string_payloads {
        let err = bridge.invoke("focusWindow", Some(p.clone())).unwrap_err();
        assert!(
            matches!(err, IpcError::InvalidArguments(_)),
            "Non-string payload {:?} must fail with InvalidArguments",
            p
        );
    }

    // 4. Parameter aliases under valid calls
    let valid_param_aliases = [
        json!({ "app": "Visual Studio Code" }),
        json!({ "name": "Google Chrome" }),
        json!({ "app_name": "Terminal" }),
        json!({ "query": "Slack" }),
    ];

    for p in valid_param_aliases {
        let res_camel = bridge.invoke("focusWindow", Some(p.clone())).expect("Valid focusWindow should succeed");
        assert_eq!(res_camel["status"], "success");
        assert_eq!(res_camel["action"], "SwitchFocus");

        let res_snake = bridge.invoke("focus_window", Some(p.clone())).expect("Valid focus_window should succeed");
        assert_eq!(res_snake["status"], "success");
        assert_eq!(res_snake["action"], "SwitchFocus");
    }
}

/// Group 3: Cycle Windows Stress Testing & Rapid Forward/Backward Cycling
#[test]
fn test_adversarial_cycle_windows_stress_and_rapid_cycling() {
    let bridge = IpcBridge::default();

    // 1. Rapid alternating forward and backward cycling (100 iterations)
    for i in 0..100 {
        let dir = if i % 2 == 0 { "Next" } else { "Previous" };
        let res = bridge
            .invoke("cycleWindows", Some(json!({ "direction": dir })))
            .unwrap_or_else(|e| panic!("cycleWindows failed at iteration {}: {:?}", i, e));
        assert_eq!(res["status"], "success");
        assert_eq!(res["direction"], dir);

        // Also alternate with snake_case
        let res_snake = bridge
            .invoke("cycle_windows", Some(json!({ "direction": dir })))
            .unwrap_or_else(|e| panic!("cycle_windows failed at iteration {}: {:?}", i, e));
        assert_eq!(res_snake["status"], "success");
        assert_eq!(res_snake["direction"], dir);
    }

    // 2. Case insensitivity and alias directions
    let direction_cases = [
        ("next", "Next"),
        ("NEXT", "Next"),
        ("NeXt", "Next"),
        ("previous", "Previous"),
        ("PREVIOUS", "Previous"),
        ("prev", "Previous"),
        ("PREV", "Previous"),
        ("pReV", "Previous"),
    ];

    for (input, expected) in direction_cases {
        let res = bridge
            .invoke("cycleWindows", Some(json!({ "direction": input })))
            .unwrap_or_else(|e| panic!("cycleWindows failed for direction '{}': {:?}", input, e));
        assert_eq!(res["status"], "success");
        assert_eq!(res["direction"], expected);
    }

    // 3. Invalid directions must be rejected
    let invalid_directions = [
        "up", "down", "left", "right", "forward", "backward", "sideways", "diagonal", "jump", ""
    ];
    for d in invalid_directions {
        let err = bridge
            .invoke("cycleWindows", Some(json!({ "direction": d })))
            .unwrap_err();
        assert!(
            matches!(err, IpcError::InvalidArguments(_)),
            "Invalid direction '{}' must fail with InvalidArguments",
            d
        );
    }

    // 4. Multi-threaded concurrent cycling stress
    let bridge_arc = Arc::new(bridge);
    let mut handles = Vec::new();
    for thread_idx in 0..8 {
        let b = Arc::clone(&bridge_arc);
        handles.push(thread::spawn(move || {
            for j in 0..50 {
                let dir = if (thread_idx + j) % 2 == 0 { "Next" } else { "Previous" };
                let res = b.invoke("cycleWindows", Some(json!({ "direction": dir })));
                assert!(res.is_ok(), "Concurrent cycleWindows must succeed");
            }
        }));
    }

    for h in handles {
        h.join().expect("Concurrent cycle thread must not panic");
    }
}

/// Group 4: Fuzzing All 16 M12 Command Aliases
#[test]
fn test_adversarial_fuzz_all_16_m12_command_aliases() {
    let bridge = IpcBridge::default();

    // 1. discoverWhisperModels & discover_whisper_models
    for cmd in ["discoverWhisperModels", "discover_whisper_models"] {
        let res = bridge.invoke(cmd, None).expect("Should succeed with None");
        assert_eq!(res["status"], "success");
        assert!(res["models"].as_array().unwrap().len() >= 7);

        // Extraneous arbitrary JSON arguments should be safely ignored
        let res_extra = bridge
            .invoke(cmd, Some(json!({ "bogus": [1, 2, 3], "nested": { "foo": "bar" } })))
            .expect("Should succeed with extraneous args");
        assert_eq!(res_extra["status"], "success");
    }

    // 2. loadWhisperModel & load_whisper_model
    for cmd in ["loadWhisperModel", "load_whisper_model"] {
        // Missing arguments
        assert!(matches!(bridge.invoke(cmd, None), Err(IpcError::InvalidArguments(_))));
        assert!(matches!(bridge.invoke(cmd, Some(json!({}))), Err(IpcError::InvalidArguments(_))));
        assert!(matches!(bridge.invoke(cmd, Some(json!({ "model": 123 }))), Err(IpcError::InvalidArguments(_))));

        // Valid model loading
        let res = bridge.invoke(cmd, Some(json!({ "model": "whisper-turbo.en" }))).unwrap();
        assert_eq!(res["status"], "success");
        assert_eq!(res["model"], "whisper-turbo.en");
    }

    // 3. getActiveModel & get_active_model
    for cmd in ["getActiveModel", "get_active_model"] {
        let res = bridge.invoke(cmd, None).expect("getActiveModel should succeed");
        assert_eq!(res["status"], "success");
        assert_eq!(res["model_path"], "whisper-turbo.en");
    }

    // 4. updateCustomDictionary & update_custom_dictionary
    for cmd in ["updateCustomDictionary", "update_custom_dictionary"] {
        // Missing rules argument
        assert!(matches!(bridge.invoke(cmd, None), Err(IpcError::InvalidArguments(_))));
        assert!(matches!(bridge.invoke(cmd, Some(json!({}))), Err(IpcError::InvalidArguments(_))));

        // Valid map format
        let res_map = bridge
            .invoke(cmd, Some(json!({ "custom_dictionary": { "POC": "proof of concept" } })))
            .expect("updateCustomDictionary map should succeed");
        assert_eq!(res_map["status"], "success");
        assert_eq!(res_map["total_rules"], 1);

        // Valid array format
        let res_arr = bridge
            .invoke(
                cmd,
                Some(json!({
                    "rules": [
                        { "trigger": "TBD", "replacement": "to be determined" },
                        { "trigger": "ETA", "replacement": "estimated time of arrival" }
                    ]
                })),
            )
            .expect("updateCustomDictionary array should succeed");
        assert_eq!(res_arr["status"], "success");
        assert_eq!(res_arr["total_rules"], 2);
    }

    // 5. searchTranscriptHistory & search_transcript_history
    for cmd in ["searchTranscriptHistory", "search_transcript_history"] {
        let res_none = bridge.invoke(cmd, None).expect("search with None should succeed");
        assert_eq!(res_none["status"], "success");

        let res_query = bridge
            .invoke(cmd, Some(json!({ "query": "!@#$%^&*()_+", "limit": 10 })))
            .expect("search with hostile query should succeed");
        assert_eq!(res_query["status"], "success");
    }

    // 6. injectTextReliable & inject_text_reliable
    for cmd in ["injectTextReliable", "inject_text_reliable"] {
        // Empty text: ignored without firing keystrokes
        let res_empty = bridge.invoke(cmd, Some(json!({ "text": "" }))).unwrap();
        assert_eq!(res_empty["status"], "Ignored");
        assert_eq!(res_empty["success"], false);
        assert_eq!(res_empty["text_length"], 0);

        // Unicode and emoji text
        let res_unicode = bridge
            .invoke(cmd, Some(json!({ "text": "🦀 AirCtrl Rust 🚀" })))
            .unwrap();
        assert_eq!(res_unicode["success"], true);

        // Long text (> 200 chars triggers clipboard fallback)
        let long_text = "A".repeat(300);
        let res_long = bridge
            .invoke(cmd, Some(json!({ "text": long_text })))
            .unwrap();
        assert_eq!(res_long["success"], true);
    }

    // 7. executeVoiceAction & execute_voice_action
    for cmd in ["executeVoiceAction", "execute_voice_action"] {
        // Missing utterance
        assert!(matches!(bridge.invoke(cmd, None), Err(IpcError::InvalidArguments(_))));
        assert!(matches!(bridge.invoke(cmd, Some(json!({}))), Err(IpcError::InvalidArguments(_))));
        assert!(matches!(bridge.invoke(cmd, Some(json!({ "utterance": 999 }))), Err(IpcError::InvalidArguments(_))));

        // Valid actions
        let utterances = [
            ("snap window left", "SnapWindow"),
            ("snap window fullscreen", "SnapWindow"),
            ("resize window 75%", "ResizeWindow"),
            ("focus Code", "SwitchFocus"),
            ("next window", "CycleWindow"),
            ("next field", "NavigateUi"),
            ("press enter", "NavigateUi"),
            ("press escape", "NavigateUi"),
            ("scroll down", "Scroll"),
            ("click left", "MouseClick"),
        ];

        for (utt, expected_act) in utterances {
            let res = bridge
                .invoke(cmd, Some(json!({ "utterance": utt })))
                .unwrap_or_else(|e| panic!("{} failed for utterance '{}': {:?}", cmd, utt, e));
            assert_eq!(res["status"], "success");
            assert_eq!(res["action"], expected_act);
        }
    }

    // 8. resizeWindowWorkArea & resize_window_work_area
    for cmd in ["resizeWindowWorkArea", "resize_window_work_area"] {
        // Default with None
        let res_def = bridge.invoke(cmd, None).unwrap();
        assert_eq!(res_def["status"], "success");
        assert_eq!(res_def["bounds"]["width"], 1280);
        assert_eq!(res_def["bounds"]["height"], 720);

        // Explicit width/height
        let res_exp = bridge
            .invoke(cmd, Some(json!({ "width": 1600, "height": 900 })))
            .unwrap();
        assert_eq!(res_exp["bounds"]["width"], 1600);
        assert_eq!(res_exp["bounds"]["height"], 900);
    }

    // 9. resizeWindowDelta & resize_window_delta
    for cmd in ["resizeWindowDelta", "resize_window_delta"] {
        for dim in ["larger", "smaller", "wider", "narrower", "taller", "shorter"] {
            let res = bridge
                .invoke(cmd, Some(json!({ "dimension": dim, "delta_pixels": 80 })))
                .unwrap_or_else(|e| panic!("{} failed for dimension '{}': {:?}", cmd, dim, e));
            assert_eq!(res["status"], "success");
        }

        // Invalid dimension must fail
        let err = bridge
            .invoke(cmd, Some(json!({ "dimension": "unsupported_delta" })))
            .unwrap_err();
        assert!(matches!(err, IpcError::InvalidArguments(_)));
    }

    // 10. focusWindow & focus_window (already covered in Group 2)

    // 11. cycleWindows & cycle_windows (already covered in Group 3)

    // 12. navigateUiField & navigate_ui_field
    for cmd in ["navigateUiField", "navigate_ui_field"] {
        // Default with None -> NextField
        let res_def = bridge.invoke(cmd, None).unwrap();
        assert_eq!(res_def["status"], "success");
        assert_eq!(res_def["element"], "NextField");

        // Explicit PrevField
        let res_prev = bridge
            .invoke(cmd, Some(json!({ "element": "PrevField" })))
            .unwrap();
        assert_eq!(res_prev["element"], "PrevField");
    }

    // 13. simulateUiKey & simulate_ui_key
    for cmd in ["simulateUiKey", "simulate_ui_key"] {
        // Missing key must fail
        assert!(matches!(bridge.invoke(cmd, None), Err(IpcError::InvalidArguments(_))));
        assert!(matches!(bridge.invoke(cmd, Some(json!({}))), Err(IpcError::InvalidArguments(_))));

        // Valid keys
        for key in ["Enter", "Return", "Escape", "Esc", "Tab"] {
            let res = bridge.invoke(cmd, Some(json!({ "key": key }))).unwrap();
            assert_eq!(res["status"], "success");
        }

        // Invalid keys
        for key in ["Backspace", "F1", "Ctrl", "Alt", "PrintScreen", "Delete"] {
            let err = bridge.invoke(cmd, Some(json!({ "key": key }))).unwrap_err();
            assert!(
                matches!(err, IpcError::InvalidArguments(_)),
                "Invalid key '{}' must fail with InvalidArguments",
                key
            );
        }
    }

    // 14. setAudioMixerVolumes & set_audio_mixer_volumes (covered in Group 1)

    // 15. getAudioMixerVolumes & get_audio_mixer_volumes
    for cmd in ["getAudioMixerVolumes", "get_audio_mixer_volumes"] {
        let res = bridge.invoke(cmd, None).unwrap();
        assert_eq!(res["status"], "success");
        assert!(res["mic_volume"].as_f64().is_some());
        assert!(res["system_volume"].as_f64().is_some());
    }

    // 16. generateAiSummary & generate_ai_summary
    for cmd in ["generateAiSummary", "generate_ai_summary"] {
        let sample_text = "Reviewing the M12 release milestones. Everything is verified.";

        // Valid summary with all templates
        for tmpl in ["ExecutiveSummary", "ActionItems", "TechnicalDecisions"] {
            let res = bridge
                .invoke(
                    cmd,
                    Some(json!({
                        "transcript": sample_text,
                        "template": tmpl
                    })),
                )
                .unwrap_or_else(|e| panic!("{} failed for template '{}': {:?}", cmd, tmpl, e));
            assert_eq!(res["status"], "success");
            assert!(res["summary"]["summary_markdown"].as_str().is_some());
        }

        // Empty transcript must fail
        assert!(matches!(
            bridge.invoke(cmd, Some(json!({ "transcript": "" }))),
            Err(IpcError::InvalidArguments(_))
        ));
        assert!(matches!(
            bridge.invoke(cmd, Some(json!({ "transcript": "   \n\t  " }))),
            Err(IpcError::InvalidArguments(_))
        ));

        // Invalid template must fail
        assert!(matches!(
            bridge.invoke(
                cmd,
                Some(json!({
                    "transcript": sample_text,
                    "template": "InvalidTemplateName"
                }))
            ),
            Err(IpcError::InvalidArguments(_))
        ));

        // Invalid provider must fail
        assert!(matches!(
            bridge.invoke(
                cmd,
                Some(json!({
                    "transcript": sample_text,
                    "provider": "InvalidProviderName"
                }))
            ),
            Err(IpcError::InvalidArguments(_))
        ));
    }
}
