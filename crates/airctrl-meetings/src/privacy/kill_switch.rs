use crate::types::{CameraState, KillSwitchStatus};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, RwLock};

const STATE_ACTIVE: u8 = 0;
const STATE_KILLED_BY_MEETING: u8 = 1;
const STATE_PAUSED_BY_USER: u8 = 2;

type ReleaseHook = Box<dyn Fn() + Send + Sync + 'static>;

/// Thread-safe camera privacy kill switch providing atomic state transitions,
/// sub-millisecond execution (<1.0ms), and release hook dispatch.
pub struct CameraKillSwitch {
    state: AtomicU8,
    release_hooks: Arc<RwLock<Vec<ReleaseHook>>>,
}

impl Default for CameraKillSwitch {
    fn default() -> Self {
        Self::new()
    }
}

impl CameraKillSwitch {
    /// Creates a new `CameraKillSwitch` initialized to `CameraState::Active`.
    pub fn new() -> Self {
        Self {
            state: AtomicU8::new(STATE_ACTIVE),
            release_hooks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Registers a closure to be executed when the camera kill switch is engaged.
    pub fn register_release_hook<F>(&self, hook: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        if let Ok(mut hooks) = self.release_hooks.write() {
            hooks.push(Box::new(hook));
        }
    }

    /// Triggers the camera kill switch upon meeting discovery or privacy demand.
    ///
    /// Returns `KillSwitchStatus`:
    /// - `status: "KilledByMeeting"`, `camera_handle_closed: true` on first transition.
    /// - `status: "AlreadyReleased"`, `camera_handle_closed: false` on repeated invocations (idempotent).
    pub fn trigger(&self) -> KillSwitchStatus {
        match self
            .state
            .compare_exchange(STATE_ACTIVE, STATE_KILLED_BY_MEETING, Ordering::SeqCst, Ordering::SeqCst)
        {
            Ok(_) => {
                // Execute registered handle release hooks
                if let Ok(hooks) = self.release_hooks.read() {
                    for hook in hooks.iter() {
                        hook();
                    }
                }
                KillSwitchStatus {
                    status: "KilledByMeeting".to_string(),
                    camera_handle_closed: true,
                }
            }
            Err(_) => KillSwitchStatus {
                status: "AlreadyReleased".to_string(),
                camera_handle_closed: false,
            },
        }
    }

    /// Triggers camera privacy shutdown by explicit user command.
    pub fn pause_by_user(&self) -> KillSwitchStatus {
        match self
            .state
            .compare_exchange(STATE_ACTIVE, STATE_PAUSED_BY_USER, Ordering::SeqCst, Ordering::SeqCst)
        {
            Ok(_) => {
                if let Ok(hooks) = self.release_hooks.read() {
                    for hook in hooks.iter() {
                        hook();
                    }
                }
                KillSwitchStatus {
                    status: "PausedByUser".to_string(),
                    camera_handle_closed: true,
                }
            }
            Err(_) => KillSwitchStatus {
                status: "AlreadyReleased".to_string(),
                camera_handle_closed: false,
            },
        }
    }

    /// Reactivates the camera capture state, resetting to `Active`.
    pub fn reactivate(&self) {
        self.state.store(STATE_ACTIVE, Ordering::SeqCst);
    }

    /// Resets the camera state to `Active` (alias for reactivate).
    pub fn reset(&self) {
        self.reactivate();
    }

    /// Returns `true` if camera capture is currently active.
    pub fn is_camera_active(&self) -> bool {
        self.state.load(Ordering::SeqCst) == STATE_ACTIVE
    }

    /// Returns the current domain `CameraState`.
    pub fn get_camera_state(&self) -> CameraState {
        match self.state.load(Ordering::SeqCst) {
            STATE_ACTIVE => CameraState::Active,
            STATE_KILLED_BY_MEETING => CameraState::KilledByMeeting,
            _ => CameraState::PausedByUser,
        }
    }
}
