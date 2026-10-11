use crate::error::CoreError;
use crate::types::*;
use std::sync::Arc;

/// Low-level operating system window management operations.
pub trait PlatformWindowOps: Send + Sync {
    /// Retrieves metadata for the currently active/focused desktop window.
    fn get_active_window(&self) -> Result<WindowInfo, CoreError>;

    /// Activates and brings the specified window to the foreground.
    fn focus_window(&self, id: u64) -> Result<(), CoreError>;

    /// Enumerates all open top-level windows on the desktop.
    fn enumerate_windows(&self) -> Result<Vec<WindowInfo>, CoreError>;

    /// Minimizes the target window to the taskbar/dock.
    fn minimize_window(&self, id: u64) -> Result<(), CoreError>;

    /// Requests graceful closure of the target window.
    fn close_window(&self, id: u64) -> Result<(), CoreError>;

    /// Centers the target window on the display work area.
    fn center_window(&self, id: u64) -> Result<(), CoreError> {
        let _ = id;
        Ok(())
    }

    /// Extended enumeration returning comprehensive WindowDetails (default implementation).
    fn enumerate_window_details(&self) -> Result<Vec<WindowDetails>, CoreError> {
        let windows = self.enumerate_windows()?;
        Ok(windows.into_iter().map(WindowDetails::from).collect())
    }

    /// Retrieves comprehensive WindowDetails for the active focused window (default implementation).
    fn get_active_window_details(&self) -> Result<WindowDetails, CoreError> {
        let win = self.get_active_window()?;
        Ok(WindowDetails::from(win))
    }
}

/// Low-level operating system keyboard and mouse input simulation.
pub trait PlatformInputOps: Send + Sync {
    /// Simulates direct Unicode typing into the active input focus.
    fn type_text(&self, text: &str) -> Result<(), CoreError>;

    /// Sets the system clipboard and simulates a paste keystroke (Ctrl+V / Cmd+V).
    fn paste_text(&self, text: &str) -> Result<(), CoreError>;

    /// Simulates pressing a modifier combination with an input key.
    fn send_key_combination(
        &self,
        modifiers: &[ModifierKey],
        key: InputKey,
    ) -> Result<(), CoreError>;

    /// Moves the system pointer to absolute desktop coordinates.
    fn move_pointer(&self, x: i32, y: i32) -> Result<(), CoreError>;

    /// Performs a mouse click operation.
    fn click_pointer(&self, action: ClickAction) -> Result<(), CoreError>;
}

/// Low-level operating system audio output playback and input inspection.
pub trait PlatformAudioOps: Send + Sync {
    /// Plays PCM audio stream samples at the given sample rate.
    fn play_audio_stream(&self, samples: &[f32], sample_rate: u32) -> Result<(), CoreError>;

    /// Captures a raw chunk of PCM audio from the active input device.
    fn capture_audio_chunk(&self) -> Result<Vec<f32>, CoreError>;

    /// Checks if communication audio output is currently playing.
    fn is_audio_output_active(&self) -> Result<bool, CoreError>;

    /// Enumerates available audio device names.
    fn list_audio_devices(&self) -> Result<Vec<String>, CoreError>;
}

/// Aggregate container holding all platform abstraction providers.
#[derive(Clone)]
pub struct PlatformProvider {
    pub window: Arc<dyn PlatformWindowOps>,
    pub input: Arc<dyn PlatformInputOps>,
    pub audio: Arc<dyn PlatformAudioOps>,
}
