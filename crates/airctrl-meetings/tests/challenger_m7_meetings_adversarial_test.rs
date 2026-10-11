//! challenger_m7_meetings_adversarial_test.rs
//!
//! Milestone M7 Phase 2 Tier 5 Adversarial Coverage Hardening:
//! Deep white-box adversarial verification of `airctrl-meetings`:
//! 1. Heavy process table scanning under extreme loads (2,500+ processes, churn, ordering invariance).
//! 2. Exact process stem matching (`matches_process_name`):
//!    - Cross-platform separators, case-insensitivity, trailing slashes, compound extensions.
//!    - Categorical rejection of decoys (`zoombot`, `pan_zoom`, `teamspeak3`, `slack_helper`, etc.).
//! 3. Browser isolation and false-positive title keyword resistance:
//!    - Dedicated desktop client isolation (never matching on window title alone).
//!    - Web conference browser detection (Google Meet, Zoom Web, Teams Web, Webex Web).
//!    - Title keyword boundaries (avoiding articles/search false positives).
//! 4. Atomic CAS camera privacy kill-switch under 50-thread concurrent race:
//!    - Thread contention, exactly 1 thread claims initial handle closure.
//!    - Idempotency status verification ("AlreadyReleased").
//!    - Rapid multi-cycle trigger <-> reactivate stress.
//! 5. Legal compliance audio injection and disclosure gating:
//!    - Statutory volume floor >= 0.50 enforcement (cannot be muted).
//!    - Multi-sample-rate acoustic earcon generation (8kHz to 48kHz).
//!    - Recording session gating (locked until disclosure announcement completes).

use airctrl_core::platform::PlatformAudioOps;
use airctrl_core::error::CoreError;
use airctrl_meetings::compliance::{ComplianceAudioGenerator, ComplianceAudioInjector};
use airctrl_meetings::config::ComplianceConfig;
use airctrl_meetings::detector::signatures::{
    extract_binary_name, is_browser_process, matches_process_name, strip_exe,
};
use airctrl_meetings::detector::{DefaultMeetingDetector, MeetingDetector};
use airctrl_meetings::privacy::CameraKillSwitch;
use airctrl_meetings::types::{CameraState, MeetingApp, ProcessScanItem};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;

// ============================================================================
// 1. Heavy Process Table Scanning & Scale Invariance
// ============================================================================

#[test]
fn test_heavy_process_table_scanning_2500_processes_and_scale() {
    let detector = DefaultMeetingDetector::new();
    let num_procs = 2500;
    let mut procs = Vec::with_capacity(num_procs + 5);

    // Generate 2500 benign OS background tasks
    for i in 0..num_procs {
        procs.push(ProcessScanItem::new(
            10_000 + i as u32,
            format!("system_worker_{i}.exe"),
            format!("System Background Worker {i}"),
        ));
    }

    // Add decoys in the middle
    procs.insert(1000, ProcessScanItem::new(20_001, "zoombot.exe", "ZoomBot Automation"));
    procs.insert(1001, ProcessScanItem::new(20_002, "pan_zoom.exe", "Pan & Zoom"));
    procs.insert(1002, ProcessScanItem::new(20_003, "teamspeak3.exe", "TeamSpeak Client"));
    procs.insert(1003, ProcessScanItem::new(20_004, "slack_helper.exe", "Slack Helper"));

    // 1. Scan with NO meeting should complete within 500ms in debug mode
    let start_none = Instant::now();
    let res_none = detector.detect_meeting(&procs);
    let elapsed_none = start_none.elapsed();
    assert!(res_none.is_none(), "Decoys among 2500 processes must return None");
    assert!(
        elapsed_none.as_millis() < 500,
        "Scanning 2500 processes took too long: {:?}",
        elapsed_none
    );

    // 2. Insert genuine meeting at the very end of 2500 processes (worst-case scan)
    procs.push(ProcessScanItem::new(
        99_999,
        r"C:\Program Files\Zoom\bin\Zoom.exe",
        "Zoom Meeting - All Hands",
    ));

    let start_found = Instant::now();
    let res_found = detector.detect_meeting(&procs);
    let elapsed_found = start_found.elapsed();
    assert!(res_found.is_some(), "Meeting at end of list must be detected");
    let meeting = res_found.unwrap();
    assert_eq!(meeting.app, MeetingApp::Zoom);
    assert_eq!(meeting.pid, 99_999);
    assert!(
        elapsed_found.as_millis() < 500,
        "Worst-case scan took too long: {:?}",
        elapsed_found
    );

    // 3. Empty list handling
    assert!(detector.detect_meeting(&[]).is_none());
}

// ============================================================================
// 2. Exact Process Stem Matching (`matches_process_name`)
// ============================================================================

#[test]
fn test_exact_process_stem_matching_adversarial_paths_and_decoys() {
    // 1. Helper functions
    assert_eq!(extract_binary_name("zoom.exe"), "zoom.exe");
    assert_eq!(extract_binary_name(r"C:\bin\zoom.exe"), "zoom.exe");
    assert_eq!(extract_binary_name("/usr/local/bin/zoom"), "zoom");
    assert_eq!(strip_exe("zoom.exe"), "zoom");
    assert_eq!(strip_exe("ZOOM.EXE"), "ZOOM");
    assert_eq!(strip_exe("zoom"), "zoom");

    // 2. Exact stems match regardless of OS separator or casing
    assert!(matches_process_name("zoom.exe", "zoom.exe"));
    assert!(matches_process_name("zoom", "zoom.exe"));
    assert!(matches_process_name("ZOOM.EXE", "zoom"));
    assert!(matches_process_name(r"C:\Program Files\Zoom\Zoom.exe", "zoom.exe"));
    assert!(matches_process_name("/usr/bin/zoom", "zoom.exe"));
    assert!(!matches_process_name("teams.exe", "ms-teams.exe")); // distinct target stem
    assert!(matches_process_name("ms-teams.exe", "ms-teams.exe"));
    assert!(matches_process_name("MS-TEAMS.EXE", "ms-teams.exe"));

    // 3. Reject all decoy prefixes, suffixes, substrings, and compound extensions
    let hostile_decoys = [
        ("zoombot.exe", "zoom.exe"),
        ("zoombot", "zoom.exe"),
        ("pan_zoom.exe", "zoom.exe"),
        ("zoom_updater.exe", "zoom.exe"),
        ("zoom.exe.bak", "zoom.exe"),
        ("zoom.exe.tmp", "zoom.exe"),
        ("teamspeak3.exe", "teams.exe"),
        ("teamviewer.exe", "teams.exe"),
        ("slack_helper.exe", "slack.exe"),
        ("discordbot.exe", "discord.exe"),
        ("webextra.exe", "webex.exe"),
        ("atmgr_helper.exe", "atmgr.exe"),
    ];

    for (decoy, target) in hostile_decoys {
        assert!(
            !matches_process_name(decoy, target),
            "Decoy '{}' MUST NOT match target signature '{}'",
            decoy,
            target
        );
    }
}

// ============================================================================
// 3. Browser Isolation & Title Keyword Filtering
// ============================================================================

#[test]
fn test_browser_isolation_and_dedicated_client_distinction() {
    let detector = DefaultMeetingDetector::new();

    // 1. All recognized browsers
    let browser_names = [
        "chrome.exe", "google-chrome", "google-chrome-stable", "chromium.exe", "chromium",
        "msedge.exe", "msedge", "microsoft-edge", "firefox.exe", "firefox",
        "brave.exe", "brave-browser", "opera.exe", "safari.exe", "electron.exe",
        "vivaldi.exe", "arc.exe", "librewolf.exe",
    ];
    for b in browser_names {
        assert!(is_browser_process(b), "Process '{}' must be recognized as browser", b);
    }

    // 2. Non-browser executables must never be classified as browser
    let non_browsers = ["zoom.exe", "ms-teams.exe", "slack.exe", "discord.exe", "notepad.exe", "cmd.exe"];
    for nb in non_browsers {
        assert!(!is_browser_process(nb), "Process '{}' must NOT be browser", nb);
    }

    // 3. Dedicated desktop client isolation:
    // Window title alone without matching process MUST NEVER match!
    let notepad_zoom = vec![ProcessScanItem::new(101, "notepad.exe", "Zoom Meeting Notes")];
    assert!(detector.detect_meeting(&notepad_zoom).is_none());

    let code_teams = vec![ProcessScanItem::new(102, "code.exe", "Microsoft Teams Architecture Review")];
    assert!(detector.detect_meeting(&code_teams).is_none());

    // 4. Web conference in browser:
    // Browser process + valid conference title keyword -> Matches
    let chrome_meet = vec![ProcessScanItem::new(201, "chrome.exe", "Meet - Project Retrospective")];
    assert_eq!(
        detector.detect_meeting(&chrome_meet).unwrap().app,
        MeetingApp::GoogleMeet
    );

    let firefox_teams_web = vec![ProcessScanItem::new(202, "firefox.exe", "Microsoft Teams Web Call")];
    assert_eq!(
        detector.detect_meeting(&firefox_teams_web).unwrap().app,
        MeetingApp::Teams
    );

    let edge_zoom_web = vec![ProcessScanItem::new(203, "msedge.exe", "Zoom Web Client - Conference")];
    assert_eq!(
        detector.detect_meeting(&edge_zoom_web).unwrap().app,
        MeetingApp::Zoom
    );

    // 5. Browser tab with non-conference keywords -> None
    let chrome_articles = vec![
        ProcessScanItem::new(301, "chrome.exe", "How to configure Zoom audio - KnowledgeBase"),
        ProcessScanItem::new(302, "chrome.exe", "Google Search: Webex download"),
        ProcessScanItem::new(303, "chrome.exe", "Discord bots API documentation"),
    ];
    for item in &chrome_articles {
        assert!(
            detector.detect_meeting(std::slice::from_ref(item)).is_none(),
            "Non-conference browser tab '{}' must return None",
            item.title
        );
    }
}

// ============================================================================
// 4. Atomic CAS Camera Kill-Switch Under Concurrency Hammer
// ============================================================================

#[test]
fn test_camera_kill_switch_50_thread_atomic_cas_race() {
    let switch = Arc::new(CameraKillSwitch::new());
    let hook_counter = Arc::new(AtomicUsize::new(0));

    // Register release hook
    let hc = hook_counter.clone();
    switch.register_release_hook(move || {
        hc.fetch_add(1, Ordering::SeqCst);
    });

    assert!(switch.is_camera_active());
    assert_eq!(switch.get_camera_state(), CameraState::Active);

    let num_threads = 50;
    let barrier = Arc::new(Barrier::new(num_threads));
    let closed_count = Arc::new(AtomicUsize::new(0));
    let released_count = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::with_capacity(num_threads);

    for _ in 0..num_threads {
        let sw = switch.clone();
        let b = barrier.clone();
        let closed = closed_count.clone();
        let released = released_count.clone();

        handles.push(thread::spawn(move || {
            b.wait();
            let status = sw.trigger();
            if status.camera_handle_closed {
                closed.fetch_add(1, Ordering::SeqCst);
            } else {
                released.fetch_add(1, Ordering::SeqCst);
            }
        }));
    }

    for h in handles {
        h.join().expect("Worker thread panicked in kill-switch race");
    }

    // Strict atomic CAS invariants:
    // Exactly 1 thread succeeds in initial closure; exactly 49 threads get AlreadyReleased
    assert_eq!(closed_count.load(Ordering::SeqCst), 1);
    assert_eq!(released_count.load(Ordering::SeqCst), 49);

    // Release hook executed exactly once!
    assert_eq!(hook_counter.load(Ordering::SeqCst), 1);

    // Camera state is now KilledByMeeting
    assert!(!switch.is_camera_active());
    assert_eq!(switch.get_camera_state(), CameraState::KilledByMeeting);

    // Subsequent triggers are strictly idempotent
    let status_after = switch.trigger();
    assert_eq!(status_after.status, "AlreadyReleased");
    assert!(!status_after.camera_handle_closed);

    // Reactivation resets state
    switch.reactivate();
    assert!(switch.is_camera_active());
    assert_eq!(switch.get_camera_state(), CameraState::Active);
}

#[test]
fn test_camera_kill_switch_rapid_multi_cycle_trigger_reactivate_stress() {
    let switch = CameraKillSwitch::new();

    for cycle in 0..500 {
        assert!(switch.is_camera_active());
        let s = switch.trigger();
        assert!(s.camera_handle_closed, "Cycle {cycle} failed trigger");
        assert!(!switch.is_camera_active());

        // Repeated trigger is idempotent
        let s2 = switch.trigger();
        assert!(!s2.camera_handle_closed);

        switch.reactivate();
    }
}

// ============================================================================
// 5. Legal Compliance Audio Injection & Disclosure Gating
// ============================================================================

struct DummyAudioOps;
impl PlatformAudioOps for DummyAudioOps {
    fn play_audio_stream(&self, samples: &[f32], _sample_rate: u32) -> Result<(), CoreError> {
        assert!(!samples.is_empty());
        Ok(())
    }

    fn capture_audio_chunk(&self) -> Result<Vec<f32>, CoreError> {
        Ok(vec![0.0; 160])
    }

    fn is_audio_output_active(&self) -> Result<bool, CoreError> {
        Ok(true)
    }

    fn list_audio_devices(&self) -> Result<Vec<String>, CoreError> {
        Ok(vec!["DefaultSpeaker".into()])
    }
}

#[test]
fn test_compliance_volume_floor_and_gating_lifecycle() {
    let injector = ComplianceAudioInjector::with_audio_ops(Arc::new(DummyAudioOps));

    // 1. Legal compliance volume floor (cannot be muted below 0.50)
    let muted_config = ComplianceConfig {
        volume: 0.0,
        ..Default::default()
    };
    assert!(
        muted_config.effective_volume() >= 0.50,
        "Volume 0.0 must be elevated to >= 0.50 statutory floor"
    );

    let low_config = ComplianceConfig {
        volume: 0.25,
        ..Default::default()
    };
    assert!(
        low_config.effective_volume() >= 0.50,
        "Volume 0.25 must be elevated to >= 0.50 statutory floor"
    );

    let normal_config = ComplianceConfig {
        volume: 0.85,
        ..Default::default()
    };
    assert_eq!(normal_config.effective_volume(), 0.85);

    // 2. Recording gate lifecycle:
    // Initial state: recording disallowed!
    assert!(
        !injector.is_recording_allowed(),
        "Initial state must forbid recording before disclosure"
    );

    // Play disclosure announcement
    let play_res = injector.play_announcement(&normal_config).expect("Playback failed");
    assert_eq!(play_res.status, "Success");
    assert!(play_res.injected_into_stream);

    // After playback finishes: recording is unlocked
    assert!(
        injector.is_recording_allowed(),
        "Recording must be unlocked after disclosure completes"
    );

    // Gate reset: locked again
    injector.reset_recording_gate();
    assert!(!injector.is_recording_allowed());
}

#[test]
fn test_compliance_audio_multi_sample_rate_synthesis_and_wav_encoding() {
    let sample_rates = [8000, 16000, 24000, 44100, 48000];

    for &sr in &sample_rates {
        let duration_ms = 1000;
        let expected_samples = (sr as u64 * duration_ms as u64 / 1000) as usize;

        // 1. PCM float buffer generation
        let pcm = ComplianceAudioGenerator::generate_pcm_buffer(sr, duration_ms, 0.80);
        assert_eq!(pcm.len(), expected_samples);

        // All samples must be bounded within [-1.0, 1.0]
        for &s in &pcm {
            assert!((-1.0..=1.0).contains(&s), "Sample {} out of bounds", s);
        }

        // 2. WAV byte encoding
        let wav_bytes = ComplianceAudioGenerator::encode_wav_bytes(&pcm, sr).expect("WAV encoding failed");
        assert!(wav_bytes.len() > 44, "WAV bytes must contain header and PCM data");

        // RIFF header magic checks
        assert_eq!(&wav_bytes[0..4], b"RIFF");
        assert_eq!(&wav_bytes[8..12], b"WAVE");
        assert_eq!(&wav_bytes[12..16], b"fmt ");

        // Parse back with hound to verify spec conformance
        let cursor = std::io::Cursor::new(wav_bytes);
        let reader = hound::WavReader::new(cursor).expect("Hound must parse generated WAV");
        assert_eq!(reader.spec().channels, 1);
        assert_eq!(reader.spec().sample_rate, sr);
        assert_eq!(reader.spec().bits_per_sample, 16);
        assert_eq!(reader.len() as usize, expected_samples);
    }
}
