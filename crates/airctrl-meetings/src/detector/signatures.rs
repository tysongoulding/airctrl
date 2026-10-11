use crate::types::{MeetingApp, MeetingSignature};

/// Static list of signatures for dedicated desktop conferencing applications.
pub static DEDICATED_SIGNATURES: &[MeetingSignature] = &[
    MeetingSignature {
        app: MeetingApp::Zoom,
        process_names: &["zoom.exe", "zoom", "zoomvideowherenext.exe", "cptapp.exe"],
        title_keywords: &["zoom meeting", "zoom webinar", "zoom cloud meetings"],
        requires_title_match: false,
    },
    MeetingSignature {
        app: MeetingApp::Teams,
        process_names: &["teams.exe", "ms-teams.exe", "msteams.exe", "teams"],
        title_keywords: &["teams call", "teams meeting", "microsoft teams"],
        requires_title_match: false,
    },
    MeetingSignature {
        app: MeetingApp::Webex,
        process_names: &[
            "ciscowebex.exe",
            "webex.exe",
            "webexmta.exe",
            "atmgr.exe",
            "ciscocollabhost.exe",
            "webex",
        ],
        title_keywords: &["webex meeting", "cisco webex meeting", "cisco webex"],
        requires_title_match: false,
    },
    MeetingSignature {
        app: MeetingApp::Discord,
        process_names: &["discord.exe", "discordcanary.exe", "discordptb.exe", "discord"],
        title_keywords: &["voice connected", "general voice", "voice channel"],
        requires_title_match: false,
    },
    MeetingSignature {
        app: MeetingApp::SlackHuddle,
        process_names: &["slack.exe", "slack"],
        title_keywords: &["huddle", "slack | huddle"],
        requires_title_match: true,
    },
];

/// Known web browser executable names that host web-based conferencing tabs.
pub static BROWSER_PROCESS_NAMES: &[&str] = &[
    "chrome.exe",
    "chrome",
    "google-chrome",
    "google-chrome-stable",
    "google-chrome-beta",
    "chromium.exe",
    "chromium",
    "chromium-browser",
    "msedge.exe",
    "msedge",
    "microsoft-edge",
    "microsoft-edge-stable",
    "microsoft-edge-dev",
    "firefox.exe",
    "firefox",
    "firefox-bin",
    "firefox-esr",
    "brave.exe",
    "brave",
    "brave-browser",
    "opera.exe",
    "opera",
    "opera-browser",
    "safari.exe",
    "safari",
    "electron.exe",
    "electron",
    "vivaldi.exe",
    "vivaldi",
    "arc.exe",
    "arc",
    "librewolf.exe",
    "librewolf",
];

/// Window title substring patterns for web-based conferencing in browsers.
pub static BROWSER_TITLE_PATTERNS: &[(MeetingApp, &[&str])] = &[
    (
        MeetingApp::GoogleMeet,
        &[
            "google meet",
            "meet - ",
            "meet.google.com",
            "daily standup - google meet",
        ],
    ),
    (
        MeetingApp::Zoom,
        &[
            "zoom meeting",
            "zoom web client",
            "launch meeting - zoom",
        ],
    ),
    (
        MeetingApp::Teams,
        &[
            "microsoft teams",
            "teams call",
            "project teams review",
            "teams meeting",
        ],
    ),
    (
        MeetingApp::Webex,
        &["webex meeting", "cisco webex"],
    ),
];

/// Normalized stems of known browser executables for zero-allocation lookup.
static BROWSER_STEMS: &[&str] = &[
    "chrome",
    "google-chrome",
    "google-chrome-stable",
    "google-chrome-beta",
    "chromium",
    "chromium-browser",
    "msedge",
    "microsoft-edge",
    "microsoft-edge-stable",
    "microsoft-edge-dev",
    "firefox",
    "firefox-bin",
    "firefox-esr",
    "brave",
    "brave-browser",
    "opera",
    "opera-browser",
    "safari",
    "electron",
    "vivaldi",
    "arc",
    "librewolf",
];

/// Extracts the base binary filename component from a path,
/// handling both Windows (`\`) and Unix (`/`) path separators cleanly.
#[inline]
pub fn extract_binary_name(raw_name: &str) -> &str {
    raw_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(raw_name)
}

/// Strips `.exe` suffix case-insensitively from a binary name.
#[inline]
pub fn strip_exe(s: &str) -> &str {
    if s.len() >= 4 && s[s.len() - 4..].eq_ignore_ascii_case(".exe") {
        &s[..s.len() - 4]
    } else {
        s
    }
}

/// Matches an inspected process name against a target signature executable name.
///
/// Semantics:
/// - Extracts the base binary name handling both `/` and `\` path separators.
/// - Performs case-insensitive comparison.
/// - Strips `.exe` from both sides before comparing stems.
/// - Performs exact stem equality.
/// - Categorically rejects prefix, suffix, or substring matching.
pub fn matches_process_name(proc_name: &str, target_name: &str) -> bool {
    let p_base = extract_binary_name(proc_name);
    let t_base = extract_binary_name(target_name);

    let p_stem = strip_exe(p_base);
    let t_stem = strip_exe(t_base);

    p_stem.eq_ignore_ascii_case(t_stem)
}

/// Checks if a process name matches a known web browser.
/// Normalizes paths and executable stems (`.exe` stripped) for cross-platform robustness.
pub fn is_browser_process(process_name: &str) -> bool {
    let base = extract_binary_name(process_name);
    let stem = strip_exe(base);

    BROWSER_STEMS.iter().any(|&b| stem.eq_ignore_ascii_case(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_binary_name() {
        assert_eq!(extract_binary_name("zoom.exe"), "zoom.exe");
        assert_eq!(extract_binary_name("C:\\Program Files\\Zoom\\Zoom.exe"), "Zoom.exe");
        assert_eq!(extract_binary_name("/usr/bin/zoom"), "zoom");
        assert_eq!(extract_binary_name(""), "");
    }

    #[test]
    fn test_matches_process_name_exact_and_stems() {
        assert!(matches_process_name("zoom.exe", "zoom.exe"));
        assert!(matches_process_name("ZOOM.EXE", "zoom"));
        assert!(matches_process_name("zoom", "zoom.exe"));
        assert!(matches_process_name("C:\\Tools\\zoom.exe", "zoom"));
        assert!(matches_process_name("/usr/bin/zoom", "zoom.exe"));

        // Reject prefix and substrings
        assert!(!matches_process_name("zoombot.exe", "zoom.exe"));
        assert!(!matches_process_name("zoombot.exe", "zoom"));
        assert!(!matches_process_name("zoom_updater.exe", "zoom.exe"));
        assert!(!matches_process_name("pan_zoom.exe", "zoom"));
        assert!(!matches_process_name("slack_helper.exe", "slack.exe"));
        assert!(!matches_process_name("teamviewer.exe", "teams.exe"));
    }

    #[test]
    fn test_is_browser_process() {
        assert!(is_browser_process("chrome.exe"));
        assert!(is_browser_process("CHROME.EXE"));
        assert!(is_browser_process("google-chrome"));
        assert!(is_browser_process("/usr/bin/firefox"));
        assert!(is_browser_process("C:\\Edge\\msedge.exe"));
        assert!(is_browser_process("brave-browser"));
        assert!(is_browser_process("electron.exe"));

        assert!(!is_browser_process("zoom.exe"));
        assert!(!is_browser_process("zoombot.exe"));
        assert!(!is_browser_process("slack.exe"));
    }
}
