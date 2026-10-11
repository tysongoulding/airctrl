use crate::error::MeetingError;
use crate::types::{ActiveMeetingInfo, MeetingStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use tokio_util::sync::CancellationToken;

/// Control handle for the asynchronous meeting watcher background task.
#[derive(Clone)]
pub struct WatcherHandle {
    cancellation_token: CancellationToken,
    is_running_flag: Arc<AtomicBool>,
    status: Arc<RwLock<MeetingStatus>>,
    active_meeting: Arc<RwLock<Option<ActiveMeetingInfo>>>,
}

impl WatcherHandle {
    pub fn new(
        cancellation_token: CancellationToken,
        is_running_flag: Arc<AtomicBool>,
        status: Arc<RwLock<MeetingStatus>>,
        active_meeting: Arc<RwLock<Option<ActiveMeetingInfo>>>,
    ) -> Self {
        Self {
            cancellation_token,
            is_running_flag,
            status,
            active_meeting,
        }
    }

    /// Returns `true` if the background watcher worker is actively running.
    pub fn is_running(&self) -> bool {
        self.is_running_flag.load(Ordering::SeqCst)
    }

    /// Stops the watcher background task and signals graceful shutdown.
    pub fn stop(&self) -> Result<(), MeetingError> {
        if !self.is_running() {
            return Err(MeetingError::NotRunning);
        }
        self.cancellation_token.cancel();
        self.is_running_flag.store(false, Ordering::SeqCst);
        if let Ok(mut status) = self.status.write() {
            *status = MeetingStatus::Inactive;
        }
        Ok(())
    }

    /// Returns the current lifecycle status of the meeting watcher.
    pub fn get_status(&self) -> MeetingStatus {
        self.status
            .read()
            .map(|s| *s)
            .unwrap_or(MeetingStatus::Inactive)
    }

    /// Returns the currently active meeting details, if any.
    pub fn get_active_meeting(&self) -> Option<ActiveMeetingInfo> {
        self.active_meeting
            .read()
            .ok()
            .and_then(|m| m.clone())
    }
}
