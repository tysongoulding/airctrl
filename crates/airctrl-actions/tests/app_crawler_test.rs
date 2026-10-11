use airctrl_actions::*;
use airctrl_core::platform::create_mock_platform;
use airctrl_core::types::{WindowDetails, WindowInfo, WindowRect};


#[test]
fn test_crawl_running_applications_function() {
    let apps = crawl_running_applications().expect("crawl_running_applications failed");
    assert!(apps.len() >= 2, "Expected at least 2 windows/applications");

    // All windows must have positive geometry and valid PIDs
    for app in &apps {
        assert!(app.id > 0);
        assert!(app.process_id > 0);
        assert!(!app.process_name.is_empty());
        assert!(!app.title.is_empty());
        assert!(app.bounds.width > 0);
        assert!(app.bounds.height > 0);
    }

    // Exactly one window must be focused
    let focused_count = apps.iter().filter(|a| a.is_focused).count();
    assert_eq!(focused_count, 1, "Expected exactly 1 focused window");
}

#[tokio::test]
async fn test_window_process_crawler_with_mock_platform() {
    let (platform, controllers) = create_mock_platform();
    controllers.window.add_window(WindowInfo {
        id: 101,
        process_id: 1234,
        process_name: "cursor.exe".into(),
        title: "airctrl - Cursor".into(),
        bounds: WindowRect { x: 0, y: 0, width: 1280, height: 720 },
        is_focused: true,
        is_minimized: false,
    });
    controllers.window.add_window(WindowInfo {
        id: 102,
        process_id: 5678,
        process_name: "chrome.exe".into(),
        title: "Google Chrome".into(),
        bounds: WindowRect { x: 100, y: 100, width: 1024, height: 768 },
        is_focused: false,
        is_minimized: false,
    });
    let crawler = WindowProcessCrawler::new(platform.window.clone(), CrawlerConfig::default());

    let windows = crawler.get_open_windows().await.expect("Failed to get open windows");
    assert!(!windows.is_empty());

    let active_win = windows.iter().find(|w| w.is_focused);
    assert!(active_win.is_some());

    // Test find_window
    let match_win = crawler.find_window("cursor").await.expect("find_window failed");
    assert!(match_win.is_some());
    let matched = match_win.unwrap();
    assert_eq!(matched.window.process_name, "cursor.exe");

    // Test focus_by_query
    let focused = crawler.focus_by_query("cursor").await.expect("focus_by_query failed");
    assert_eq!(focused.process_name, "cursor.exe");
}

#[test]
fn test_window_matcher_ranking() {
    let matcher = WindowMatcher::new(0.65);
    let candidates = vec![
        WindowDetails {
            id: 1,
            process_id: 100,
            process_name: "chrome.exe".to_string(),
            title: "Google Chrome - New Tab".to_string(),
            class_name: "Chrome_WidgetWin_1".to_string(),
            bounds: WindowRect { x: 0, y: 0, width: 1920, height: 1080 },
            is_focused: false,
            is_minimized: false,
            is_maximized: true,
        },
        WindowDetails {
            id: 2,
            process_id: 200,
            process_name: "Code.exe".to_string(),
            title: "airctrl - Visual Studio Code".to_string(),
            class_name: "Chrome_WidgetWin_1".to_string(),
            bounds: WindowRect { x: 0, y: 0, width: 1280, height: 720 },
            is_focused: true,
            is_minimized: false,
            is_maximized: false,
        },
    ];

    // Exact process match
    let matches = matcher.match_windows("Code", &candidates);
    assert!(!matches.is_empty());
    assert_eq!(matches[0].window.id, 2);

    // Exact title match / substring
    let matches_chrome = matcher.match_windows("Chrome", &candidates);
    assert!(!matches_chrome.is_empty());
    assert_eq!(matches_chrome[0].window.id, 1);
}

#[test]
fn test_window_details_conversions() {
    let info = WindowInfo {
        id: 42,
        process_id: 999,
        process_name: "app.exe".to_string(),
        title: "Test App".to_string(),
        bounds: WindowRect { x: 10, y: 20, width: 800, height: 600 },
        is_focused: true,
        is_minimized: false,
    };

    let details = WindowDetails::from(info.clone());
    assert_eq!(details.id, 42);
    assert_eq!(details.process_id, 999);
    assert_eq!(details.class_name, "");
    assert!(!details.is_maximized);

    let roundtrip = WindowInfo::from(details);
    assert_eq!(roundtrip, info);
}
