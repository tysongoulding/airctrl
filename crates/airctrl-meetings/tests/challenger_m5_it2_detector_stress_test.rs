use airctrl_meetings::detector::{DefaultMeetingDetector, MeetingDetector};
use airctrl_meetings::types::{MeetingApp, ProcessScanItem};

// ============================================================================
// PART 1: Path and Casing Edge Attacks
// ============================================================================

#[test]
fn test_nested_process_paths_with_mixed_separators() {
    let detector = DefaultMeetingDetector::new();

    // 1. Windows path with spaces and backslashes
    let win_zoom = vec![ProcessScanItem::new(
        1001,
        r"C:\Program Files (x86)\Zoom\bin\zoom.exe",
        "Zoom Meeting",
    )];
    let res_win_zoom = detector.detect_meeting(&win_zoom);
    assert!(
        res_win_zoom.is_some(),
        "Nested Windows path with backslashes for Zoom must be detected"
    );
    assert_eq!(res_win_zoom.unwrap().app, MeetingApp::Zoom);

    // 2. Linux nested path with forward slashes
    let linux_zoom = vec![ProcessScanItem::new(
        1002,
        "/opt/zoom/zoom",
        "Zoom",
    )];
    let res_linux_zoom = detector.detect_meeting(&linux_zoom);
    assert!(
        res_linux_zoom.is_some(),
        "Linux nested path for Zoom must be detected"
    );
    assert_eq!(res_linux_zoom.unwrap().app, MeetingApp::Zoom);

    // 3. UNC network share path with uppercase extension
    let unc_teams = vec![ProcessScanItem::new(
        1003,
        r"\\network\share\teams.EXE",
        "Microsoft Teams",
    )];
    let res_unc_teams = detector.detect_meeting(&unc_teams);
    assert!(
        res_unc_teams.is_some(),
        "UNC network share path with uppercase .EXE for Teams must be detected"
    );
    assert_eq!(res_unc_teams.unwrap().app, MeetingApp::Teams);

    // 4. Mixed forward and backward slashes
    let mixed_slashes = vec![ProcessScanItem::new(
        1004,
        r"C:/Program Files (x86)\Zoom/bin\zoom.exe",
        "Zoom Meeting",
    )];
    let res_mixed = detector.detect_meeting(&mixed_slashes);
    assert!(
        res_mixed.is_some(),
        "Mixed slash separators must be correctly resolved"
    );
    assert_eq!(res_mixed.unwrap().app, MeetingApp::Zoom);

    // 5. Casing variations in path and binary
    let upper_unc = vec![ProcessScanItem::new(
        1005,
        r"\\SERVER\SHARE\ZOOM.EXE",
        "Meeting",
    )];
    let res_upper = detector.detect_meeting(&upper_unc);
    assert!(
        res_upper.is_some(),
        "Uppercase UNC path must be detected"
    );
    assert_eq!(res_upper.unwrap().app, MeetingApp::Zoom);
}

#[test]
fn test_decoy_variations_categorically_rejected() {
    let detector = DefaultMeetingDetector::new();

    // 1. zoombot_service.exe
    let zoombot_service = vec![ProcessScanItem::new(
        2001,
        "zoombot_service.exe",
        "ZoomBot Background Service",
    )];
    assert!(
        detector.detect_meeting(&zoombot_service).is_none(),
        "zoombot_service.exe must return None"
    );

    // 2. teamspeak3.exe
    let teamspeak3 = vec![ProcessScanItem::new(
        2002,
        "teamspeak3.exe",
        "TeamSpeak 3 Client",
    )];
    assert!(
        detector.detect_meeting(&teamspeak3).is_none(),
        "teamspeak3.exe must return None"
    );

    // 3. webex_uninstall.exe
    let webex_uninstall = vec![ProcessScanItem::new(
        2003,
        "webex_uninstall.exe",
        "Cisco Webex Uninstaller",
    )];
    assert!(
        detector.detect_meeting(&webex_uninstall).is_none(),
        "webex_uninstall.exe must return None"
    );

    // 4. discord_rpc_server.exe
    let discord_rpc = vec![ProcessScanItem::new(
        2004,
        "discord_rpc_server.exe",
        "Discord RPC Local Server",
    )];
    assert!(
        detector.detect_meeting(&discord_rpc).is_none(),
        "discord_rpc_server.exe must return None"
    );

    // 5. Additional decoy variations
    let decoys = vec![
        ProcessScanItem::new(2005, "zoombot.exe", "ZoomBot"),
        ProcessScanItem::new(2006, "zoom_updater.exe", "Zoom Updater"),
        ProcessScanItem::new(2007, "pan_zoom.exe", "Pan & Zoom Tool"),
        ProcessScanItem::new(2008, "slack_helper.exe", "Slack Helper"),
        ProcessScanItem::new(2009, "teamviewer.exe", "TeamViewer Session"),
        ProcessScanItem::new(2010, "discordbot.exe", "Discord Bot Daemon"),
        ProcessScanItem::new(2011, "webextra.exe", "Web Extra Tool"),
    ];
    for decoy in &decoys {
        assert!(
            detector.detect_meeting(std::slice::from_ref(decoy)).is_none(),
            "Decoy process {} must return None",
            decoy.process_name
        );
    }
}

#[test]
fn test_text_and_code_files_open_in_editors_return_none() {
    let detector = DefaultMeetingDetector::new();

    // 1. zoom.txt - Notepad
    let notepad_zoom = vec![ProcessScanItem::new(
        3001,
        "notepad.exe",
        "zoom.txt - Notepad",
    )];
    assert!(
        detector.detect_meeting(&notepad_zoom).is_none(),
        "zoom.txt in Notepad must return None"
    );

    // 2. README.md - Visual Studio Code (both Code.exe and electron.exe)
    let vscode_readme = vec![ProcessScanItem::new(
        3002,
        "Code.exe",
        "README.md - Visual Studio Code",
    )];
    assert!(
        detector.detect_meeting(&vscode_readme).is_none(),
        "README.md in Code.exe must return None"
    );

    let electron_readme = vec![ProcessScanItem::new(
        3003,
        "electron.exe",
        "README.md - Visual Studio Code",
    )];
    assert!(
        detector.detect_meeting(&electron_readme).is_none(),
        "README.md in electron.exe must return None"
    );

    // 3. teams_spec.docx - Word
    let word_teams = vec![ProcessScanItem::new(
        3004,
        "WINWORD.EXE",
        "teams_spec.docx - Word",
    )];
    assert!(
        detector.detect_meeting(&word_teams).is_none(),
        "teams_spec.docx in Word must return None"
    );

    let word_lower = vec![ProcessScanItem::new(
        3005,
        "word.exe",
        "teams_spec.docx - Microsoft Word",
    )];
    assert!(
        detector.detect_meeting(&word_lower).is_none(),
        "teams_spec.docx in word.exe must return None"
    );

    // 4. Editor process where process name itself resembles document or editor
    let doc_procs = vec![
        ProcessScanItem::new(3006, "zoom.txt", "zoom.txt"),
        ProcessScanItem::new(3007, "README.md", "README.md"),
        ProcessScanItem::new(3008, "teams_spec.docx", "teams_spec.docx"),
        ProcessScanItem::new(3009, "sublime_text.exe", "zoom_meeting_notes.py - Sublime Text"),
        ProcessScanItem::new(3010, "vim", "teams_architecture.rs - VIM"),
    ];
    for item in &doc_procs {
        assert!(
            detector.detect_meeting(std::slice::from_ref(item)).is_none(),
            "Editor/document item {} must return None",
            item.process_name
        );
    }
}

// ============================================================================
// PART 2: Valid Meeting Detection Preservation
// ============================================================================

#[test]
fn test_valid_meeting_detection_preservation_all_clients() {
    let detector = DefaultMeetingDetector::new();

    // 1. Genuine Zoom: Zoom.exe and zoom
    let zoom_win = vec![ProcessScanItem::new(4001, "Zoom.exe", "Zoom Meeting")];
    let res_zoom_win = detector.detect_meeting(&zoom_win);
    assert!(
        res_zoom_win.is_some(),
        "Genuine Zoom.exe must return Some"
    );
    assert_eq!(res_zoom_win.unwrap().app, MeetingApp::Zoom);

    let zoom_linux = vec![ProcessScanItem::new(4002, "zoom", "Zoom")];
    let res_zoom_linux = detector.detect_meeting(&zoom_linux);
    assert!(
        res_zoom_linux.is_some(),
        "Genuine zoom (Linux) must return Some"
    );
    assert_eq!(res_zoom_linux.unwrap().app, MeetingApp::Zoom);

    // 2. Genuine Teams: ms-teams.exe and teams
    let teams_win = vec![ProcessScanItem::new(4003, "ms-teams.exe", "Teams Call")];
    let res_teams_win = detector.detect_meeting(&teams_win);
    assert!(
        res_teams_win.is_some(),
        "Genuine ms-teams.exe must return Some"
    );
    assert_eq!(res_teams_win.unwrap().app, MeetingApp::Teams);

    let teams_linux = vec![ProcessScanItem::new(4004, "teams", "Teams Meeting")];
    let res_teams_linux = detector.detect_meeting(&teams_linux);
    assert!(
        res_teams_linux.is_some(),
        "Genuine teams (Linux) must return Some"
    );
    assert_eq!(res_teams_linux.unwrap().app, MeetingApp::Teams);

    // 3. Genuine Webex: atmgr.exe and webex.exe
    let webex_atmgr = vec![ProcessScanItem::new(4005, "atmgr.exe", "Cisco Webex Meeting")];
    let res_webex_atmgr = detector.detect_meeting(&webex_atmgr);
    assert!(
        res_webex_atmgr.is_some(),
        "Genuine atmgr.exe must return Some"
    );
    assert_eq!(res_webex_atmgr.unwrap().app, MeetingApp::Webex);

    let webex_win = vec![ProcessScanItem::new(4006, "webex.exe", "Webex Meeting")];
    let res_webex_win = detector.detect_meeting(&webex_win);
    assert!(
        res_webex_win.is_some(),
        "Genuine webex.exe must return Some"
    );
    assert_eq!(res_webex_win.unwrap().app, MeetingApp::Webex);

    // 4. Genuine Discord: discord.exe
    let discord = vec![ProcessScanItem::new(4007, "discord.exe", "General Voice")];
    let res_discord = detector.detect_meeting(&discord);
    assert!(
        res_discord.is_some(),
        "Genuine discord.exe must return Some"
    );
    assert_eq!(res_discord.unwrap().app, MeetingApp::Discord);

    // 5. Genuine Slack Huddle: slack.exe with title "Huddle"
    let slack_huddle = vec![ProcessScanItem::new(4008, "slack.exe", "Huddle")];
    let res_slack_huddle = detector.detect_meeting(&slack_huddle);
    assert!(
        res_slack_huddle.is_some(),
        "slack.exe with title 'Huddle' must return Some"
    );
    assert_eq!(res_slack_huddle.unwrap().app, MeetingApp::SlackHuddle);

    // 6. Genuine Google Meet: chrome.exe with title "Meet - Daily Standup"
    let google_meet = vec![ProcessScanItem::new(4009, "chrome.exe", "Meet - Daily Standup")];
    let res_google_meet = detector.detect_meeting(&google_meet);
    assert!(
        res_google_meet.is_some(),
        "chrome.exe with title 'Meet - Daily Standup' must return Some"
    );
    assert_eq!(res_google_meet.unwrap().app, MeetingApp::GoogleMeet);
}

// ============================================================================
// PART 3: Comprehensive Stress & Combined Matrix
// ============================================================================

#[test]
fn test_mixed_table_stress_with_decoys_editors_and_real_meeting() {
    let detector = DefaultMeetingDetector::new();

    let mut procs = Vec::new();

    // Add 100 benign processes
    for i in 0..100 {
        procs.push(ProcessScanItem::new(
            5000 + i,
            format!("system_service_{i}.exe"),
            format!("Service {i}"),
        ));
    }

    // Add decoys
    procs.push(ProcessScanItem::new(6001, "zoombot_service.exe", "ZoomBot Worker"));
    procs.push(ProcessScanItem::new(6002, "teamspeak3.exe", "TeamSpeak Client"));
    procs.push(ProcessScanItem::new(6003, "webex_uninstall.exe", "Uninstall Wizard"));
    procs.push(ProcessScanItem::new(6004, "discord_rpc_server.exe", "RPC Server"));

    // Add editor / documents
    procs.push(ProcessScanItem::new(6005, "notepad.exe", "zoom.txt - Notepad"));
    procs.push(ProcessScanItem::new(6006, "Code.exe", "README.md - Visual Studio Code"));
    procs.push(ProcessScanItem::new(6007, "WINWORD.EXE", "teams_spec.docx - Word"));

    // First assertion: without any genuine meeting, table MUST evaluate to None
    let res_none = detector.detect_meeting(&procs);
    assert!(
        res_none.is_none(),
        "Process table with decoys and text editor documents must return None"
    );

    // Now inject genuine meeting: nested path Zoom
    procs.push(ProcessScanItem::new(
        7001,
        r"C:\Program Files (x86)\Zoom\bin\zoom.exe",
        "Zoom Meeting - Engineering",
    ));

    let res_some = detector.detect_meeting(&procs);
    assert!(
        res_some.is_some(),
        "Table containing genuine Zoom among decoys and editors must detect Zoom"
    );
    let meeting = res_some.unwrap();
    assert_eq!(meeting.app, MeetingApp::Zoom);
    assert_eq!(meeting.pid, 7001);
}

// ============================================================================
// PART 4: Deep Adversarial Path & Extension Fuzzing
// ============================================================================

#[test]
fn test_extended_decoy_extensions_and_relative_paths() {
    let detector = DefaultMeetingDetector::new();

    // 1. Files ending with non-exe suffixes or backup extensions
    let backup_files = vec![
        ProcessScanItem::new(8001, "zoom.exe.bak", "Zoom Meeting"),
        ProcessScanItem::new(8002, "teams.exe.old", "Teams Call"),
        ProcessScanItem::new(8003, "webex.exe.tmp", "Webex Meeting"),
        ProcessScanItem::new(8004, "discord.exe.lnk", "Discord Shortcut"),
        ProcessScanItem::new(8005, "zoom.sh", "Zoom Script"),
        ProcessScanItem::new(8006, "teams.py", "Teams Automation"),
    ];
    for item in &backup_files {
        assert!(
            detector.detect_meeting(std::slice::from_ref(item)).is_none(),
            "Non-binary extension {} must return None",
            item.process_name
        );
    }

    // 2. Relative paths with dot traversal
    let rel_zoom = vec![ProcessScanItem::new(
        8010,
        r"..\..\Program Files\Zoom\bin\zoom.exe",
        "Zoom Meeting",
    )];
    let res_rel = detector.detect_meeting(&rel_zoom);
    assert!(
        res_rel.is_some(),
        "Relative dot-traversal path for zoom.exe must be detected"
    );
    assert_eq!(res_rel.unwrap().app, MeetingApp::Zoom);

    // 3. Leading slashes and complex UNC
    let complex_unc = vec![ProcessScanItem::new(
        8011,
        r"\\?\UNC\storage-srv\apps\conferencing\teams.exe",
        "Microsoft Teams",
    )];
    let res_unc = detector.detect_meeting(&complex_unc);
    assert!(
        res_unc.is_some(),
        "Complex UNC path for teams.exe must be detected"
    );
    assert_eq!(res_unc.unwrap().app, MeetingApp::Teams);

    // 4. Dedicated clients with empty window title still detected
    let empty_title_zoom = vec![ProcessScanItem::new(8012, "Zoom.exe", "")];
    assert_eq!(
        detector.detect_meeting(&empty_title_zoom).unwrap().app,
        MeetingApp::Zoom
    );

    let empty_title_teams = vec![ProcessScanItem::new(8013, "ms-teams.exe", "")];
    assert_eq!(
        detector.detect_meeting(&empty_title_teams).unwrap().app,
        MeetingApp::Teams
    );

    let empty_title_webex = vec![ProcessScanItem::new(8014, "atmgr.exe", "")];
    assert_eq!(
        detector.detect_meeting(&empty_title_webex).unwrap().app,
        MeetingApp::Webex
    );

    let empty_title_discord = vec![ProcessScanItem::new(8015, "discord.exe", "")];
    assert_eq!(
        detector.detect_meeting(&empty_title_discord).unwrap().app,
        MeetingApp::Discord
    );

    // Slack with empty title MUST NOT be detected because it requires title match
    let empty_title_slack = vec![ProcessScanItem::new(8016, "slack.exe", "")];
    assert!(
        detector.detect_meeting(&empty_title_slack).is_none(),
        "slack.exe with empty title must return None"
    );
}

