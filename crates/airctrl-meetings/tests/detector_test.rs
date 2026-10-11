use airctrl_meetings::detector::{DefaultMeetingDetector, MeetingDetector};
use airctrl_meetings::types::{MeetingApp, ProcessScanItem};
use std::time::Instant;

#[test]
fn test_zoom_meeting_detected() {
    let detector = DefaultMeetingDetector::new();
    let procs = vec![ProcessScanItem::new(100, "zoom.exe", "Zoom Meeting")];
    let meeting = detector.detect_meeting(&procs);
    assert!(meeting.is_some());
    let m = meeting.unwrap();
    assert_eq!(m.app, MeetingApp::Zoom);
    assert_eq!(m.pid, 100);
}

#[test]
fn test_teams_meeting_detected() {
    let detector = DefaultMeetingDetector::new();
    let procs = vec![ProcessScanItem::new(200, "teams.exe", "Teams Call")];
    let meeting = detector.detect_meeting(&procs);
    assert!(meeting.is_some());
    let m = meeting.unwrap();
    assert_eq!(m.app, MeetingApp::Teams);
    assert_eq!(m.pid, 200);
}

#[test]
fn test_google_meet_in_browser_detected() {
    let detector = DefaultMeetingDetector::new();
    let procs = vec![ProcessScanItem::new(
        300,
        "chrome.exe",
        "Daily Standup - Google Meet",
    )];
    let meeting = detector.detect_meeting(&procs);
    assert!(meeting.is_some());
    let m = meeting.unwrap();
    assert_eq!(m.app, MeetingApp::GoogleMeet);
    assert_eq!(m.pid, 300);
}

#[test]
fn test_webex_and_discord_detected() {
    let detector = DefaultMeetingDetector::new();

    let procs_webex = vec![ProcessScanItem::new(400, "ciscowebex.exe", "Webex")];
    let m_webex = detector.detect_meeting(&procs_webex);
    assert!(m_webex.is_some());
    assert_eq!(m_webex.unwrap().app, MeetingApp::Webex);

    let procs_discord = vec![ProcessScanItem::new(500, "Discord.exe", "General Voice")];
    let m_discord = detector.detect_meeting(&procs_discord);
    assert!(m_discord.is_some());
    assert_eq!(m_discord.unwrap().app, MeetingApp::Discord);
}

#[test]
fn test_slack_huddle_vs_normal_slack_chat() {
    let detector = DefaultMeetingDetector::new();

    // Normal Slack text chat: NOT a meeting
    let normal_slack = vec![ProcessScanItem::new(600, "slack.exe", "#general - Acme Corp")];
    assert!(detector.detect_meeting(&normal_slack).is_none());

    // Slack with active Huddle: IS a meeting
    let huddle_slack = vec![ProcessScanItem::new(601, "slack.exe", "Slack | Huddle")];
    let m_huddle = detector.detect_meeting(&huddle_slack);
    assert!(m_huddle.is_some());
    assert_eq!(m_huddle.unwrap().app, MeetingApp::SlackHuddle);
}

#[test]
fn test_case_insensitive_detection() {
    let detector = DefaultMeetingDetector::new();

    let uppercase_zoom = vec![ProcessScanItem::new(101, "ZOOM.EXE", "MEETING")];
    let m = detector.detect_meeting(&uppercase_zoom);
    assert!(m.is_some());
    assert_eq!(m.unwrap().app, MeetingApp::Zoom);

    let uppercase_meet = vec![ProcessScanItem::new(102, "CHROME.EXE", "STANDUP - GOOGLE MEET")];
    let m2 = detector.detect_meeting(&uppercase_meet);
    assert!(m2.is_some());
    assert_eq!(m2.unwrap().app, MeetingApp::GoogleMeet);
}

#[test]
fn test_substring_match_on_electron() {
    let detector = DefaultMeetingDetector::new();
    let procs = vec![ProcessScanItem::new(
        103,
        "electron.exe",
        "project teams review",
    )];
    let m = detector.detect_meeting(&procs);
    assert!(m.is_some());
    assert_eq!(m.unwrap().app, MeetingApp::Teams);
}

#[test]
fn test_non_meeting_processes_ignored() {
    let detector = DefaultMeetingDetector::new();
    let procs = vec![
        ProcessScanItem::new(10, "Code.exe", "Visual Studio Code"),
        ProcessScanItem::new(11, "notepad.exe", "Untitled"),
        ProcessScanItem::new(12, "bash", "terminal"),
        ProcessScanItem::new(13, "cargo", "cargo test"),
    ];
    assert!(detector.detect_meeting(&procs).is_none());
}

#[test]
fn test_empty_process_table_returns_none() {
    let detector = DefaultMeetingDetector::new();
    assert!(detector.detect_meeting(&[]).is_none());
}

#[test]
fn test_large_process_table_performance() {
    let detector = DefaultMeetingDetector::new();
    let mut procs = Vec::with_capacity(501);
    for i in 0..500 {
        procs.push(ProcessScanItem::new(
            i,
            format!("proc_{i}.exe"),
            format!("Window Title {i}"),
        ));
    }
    procs.push(ProcessScanItem::new(999, "webex.exe", "Webex Meeting"));

    let start = Instant::now();
    let meeting = detector.detect_meeting(&procs);
    let duration = start.elapsed();

    assert!(meeting.is_some());
    assert_eq!(meeting.unwrap().app, MeetingApp::Webex);
    assert!(
        duration.as_millis() < 50,
        "Scanning 500 processes took too long: {:?}",
        duration
    );
}
