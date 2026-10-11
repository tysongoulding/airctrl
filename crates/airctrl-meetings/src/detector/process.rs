use super::signatures::{matches_process_name, DEDICATED_SIGNATURES};
use crate::types::{ActiveMeetingInfo, ProcessScanItem};
use chrono::Utc;

/// Scans a process scan item for dedicated conferencing desktop clients.
///
/// Dedicated desktop clients MANDATORILY require a matching process name (`proc_matches`).
/// Dedicated desktop clients can NEVER match based on window title alone without a matching process!
pub fn scan_dedicated_item(item: &ProcessScanItem) -> Option<ActiveMeetingInfo> {
    for sig in DEDICATED_SIGNATURES {
        let proc_matches = sig
            .process_names
            .iter()
            .any(|&name| matches_process_name(&item.process_name, name));

        // MANDATORY: if !proc_matches, continue to next signature.
        // Dedicated desktop clients can NEVER match based on window title alone without a matching process!
        if !proc_matches {
            continue;
        }

        // If title match is required (e.g. Slack Huddle), verify title match
        if sig.requires_title_match {
            let title_lower = item.title.to_lowercase();
            let title_matches = sig
                .title_keywords
                .iter()
                .any(|&keyword| title_lower.contains(keyword));

            if !title_matches {
                continue;
            }
        }

        return Some(ActiveMeetingInfo {
            app: sig.app,
            window_id: item.window_id,
            pid: item.process_id,
            title: item.title.clone(),
            start_time: Utc::now(),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MeetingApp;

    #[test]
    fn test_scan_dedicated_item_genuine_clients() {
        let zoom = ProcessScanItem::new(101, "Zoom.exe", "Zoom Meeting");
        assert_eq!(scan_dedicated_item(&zoom).unwrap().app, MeetingApp::Zoom);

        let teams = ProcessScanItem::new(102, "ms-teams.exe", "Teams Call");
        assert_eq!(scan_dedicated_item(&teams).unwrap().app, MeetingApp::Teams);

        let webex = ProcessScanItem::new(103, "atmgr.exe", "Webex Meeting");
        assert_eq!(scan_dedicated_item(&webex).unwrap().app, MeetingApp::Webex);

        let discord = ProcessScanItem::new(104, "discord.exe", "General Voice");
        assert_eq!(scan_dedicated_item(&discord).unwrap().app, MeetingApp::Discord);

        let slack_huddle = ProcessScanItem::new(105, "slack.exe", "Slack | Huddle - Sync");
        assert_eq!(
            scan_dedicated_item(&slack_huddle).unwrap().app,
            MeetingApp::SlackHuddle
        );
    }

    #[test]
    fn test_scan_dedicated_item_rejects_decoys_and_helpers() {
        let zoombot = ProcessScanItem::new(201, "zoombot.exe", "ZoomBot Headless Automation");
        assert!(scan_dedicated_item(&zoombot).is_none());

        let pan_zoom = ProcessScanItem::new(202, "pan_zoom.exe", "Pan and Zoom Tool");
        assert!(scan_dedicated_item(&pan_zoom).is_none());

        let zoom_updater = ProcessScanItem::new(203, "zoom_updater.exe", "Zoom");
        assert!(scan_dedicated_item(&zoom_updater).is_none());

        let slack_helper = ProcessScanItem::new(204, "slack_helper.exe", "Slack Helper");
        assert!(scan_dedicated_item(&slack_helper).is_none());

        let teamviewer = ProcessScanItem::new(205, "teamviewer.exe", "TeamViewer Session");
        assert!(scan_dedicated_item(&teamviewer).is_none());

        let slack_chat = ProcessScanItem::new(206, "slack.exe", "Slack | General");
        assert!(scan_dedicated_item(&slack_chat).is_none());

        // Title alone without matching process MUST return None
        let notepad_zoom = ProcessScanItem::new(207, "notepad.exe", "Zoom Meeting");
        assert!(scan_dedicated_item(&notepad_zoom).is_none());
    }
}
