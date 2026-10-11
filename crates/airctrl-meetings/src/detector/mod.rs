pub mod browser;
pub mod process;
pub mod signatures;

use crate::error::MeetingError;
use crate::types::{ActiveMeetingInfo, ProcessScanItem};
use browser::scan_browser_item;
use process::scan_dedicated_item;
use signatures::is_browser_process;

/// Trait defining meeting detection operations over inspected process lists or live system state.
pub trait MeetingDetector: Send + Sync {
    /// Evaluates a list of process/window scan items and returns the first active meeting detected.
    fn detect_meeting(&self, items: &[ProcessScanItem]) -> Option<ActiveMeetingInfo>;

    /// Scans live system processes and returns an active meeting if found.
    fn scan_system(&self) -> Result<Option<ActiveMeetingInfo>, MeetingError>;
}

/// Default implementation of the `MeetingDetector` trait.
#[derive(Debug, Default, Clone)]
pub struct DefaultMeetingDetector;

impl DefaultMeetingDetector {
    pub fn new() -> Self {
        Self
    }

    /// Scans an individual item against browser and dedicated application signatures.
    pub fn scan_item(&self, item: &ProcessScanItem) -> Option<ActiveMeetingInfo> {
        let proc_lower = item.process_name.to_lowercase();

        // 1. Browser Process Isolation:
        // If the process is a recognized web browser, evaluate exclusively via scan_browser_item.
        // Recognized browser processes must NEVER fall through to dedicated client scanning.
        if is_browser_process(&proc_lower) {
            return scan_browser_item(item);
        }

        // 2. For non-browser processes, check browser item first (for explicit titles e.g. Google Meet)
        if let Some(meeting) = scan_browser_item(item) {
            return Some(meeting);
        }

        // 3. Dedicated desktop client scan (Zoom, Teams, Webex, Discord, Slack Huddle)
        if let Some(meeting) = scan_dedicated_item(item) {
            return Some(meeting);
        }

        None
    }
}

impl MeetingDetector for DefaultMeetingDetector {
    fn detect_meeting(&self, items: &[ProcessScanItem]) -> Option<ActiveMeetingInfo> {
        for item in items {
            if let Some(meeting) = self.scan_item(item) {
                return Some(meeting);
            }
        }
        None
    }

    fn scan_system(&self) -> Result<Option<ActiveMeetingInfo>, MeetingError> {
        let mut sys = sysinfo::System::new();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let items: Vec<ProcessScanItem> = sys
            .processes()
            .iter()
            .map(|(pid, proc)| {
                let name = proc.name().to_string_lossy().to_string();
                ProcessScanItem::new(pid.as_u32(), name.clone(), name)
            })
            .collect();

        Ok(self.detect_meeting(&items))
    }
}
