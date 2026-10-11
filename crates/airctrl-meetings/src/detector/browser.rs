use super::signatures::{is_browser_process, BROWSER_TITLE_PATTERNS};
use crate::types::{ActiveMeetingInfo, ProcessScanItem};
use chrono::Utc;

/// Scans a process scan item for browser-based meeting sessions (Google Meet, Teams Web, etc.).
pub fn scan_browser_item(item: &ProcessScanItem) -> Option<ActiveMeetingInfo> {
    // Check if it is a known browser/electron process, or if the title explicitly mentions Google Meet
    let is_browser = is_browser_process(&item.process_name);
    let explicit_meet_title = !is_browser
        && (item.title.contains("Google Meet")
            || item.title.contains("google meet")
            || item.title.contains("meet.google.com")
            || item.title.to_lowercase().contains("google meet"));

    if !is_browser && !explicit_meet_title {
        return None;
    }

    let title_lower = item.title.to_lowercase();

    for &(app, patterns) in BROWSER_TITLE_PATTERNS {
        for &pattern in patterns {
            if title_lower.contains(pattern) {
                return Some(ActiveMeetingInfo {
                    app,
                    window_id: item.window_id,
                    pid: item.process_id,
                    title: item.title.clone(),
                    start_time: Utc::now(),
                });
            }
        }
    }

    None
}
