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

    /// Mutates the rectangular bounds (x, y, width, height) of target window.
    fn set_window_bounds(&self, id: u64, bounds: WindowRect) -> Result<(), CoreError> {
        let _ = (id, bounds);
        Ok(())
    }

    /// Maximizes the target window to occupy the monitor work area.
    fn maximize_window(&self, id: u64) -> Result<(), CoreError> {
        let work = self.get_work_area()?;
        self.set_window_bounds(id, work)
    }

    /// Restores the target window from maximized or minimized state.
    fn restore_window(&self, id: u64) -> Result<(), CoreError> {
        self.set_window_bounds(id, WindowRect { x: 100, y: 100, width: 1024, height: 768 })
    }

    /// Sets or unsets borderless fullscreen state.
    fn set_fullscreen(&self, id: u64, fullscreen: bool) -> Result<(), CoreError> {
        let _ = fullscreen;
        let work = self.get_work_area()?;
        self.set_window_bounds(id, work)
    }

    /// Returns the active monitor work area (excluding taskbar/dock reservations).
    fn get_work_area(&self) -> Result<WindowRect, CoreError> {
        Ok(WindowRect { x: 0, y: 0, width: 1920, height: 1080 })
    }

    /// Returns the monitor work area containing the specified window.
    fn get_display_for_window(&self, id: u64) -> Result<WindowRect, CoreError> {
        let _ = id;
        self.get_work_area()
    }

    /// Cycles focus through top-level windows in desktop z-order.
    fn cycle_window(&self, forward: bool) -> Result<WindowInfo, CoreError> {
        let windows = self.enumerate_windows()?;
        if windows.is_empty() {
            return Err(CoreError::Window("No windows available to cycle".into()));
        }
        if windows.len() == 1 {
            return Ok(windows[0].clone());
        }
        let cur_id = self.get_active_window().map(|w| w.id).unwrap_or(0);
        let cur_idx = windows.iter().position(|w| w.id == cur_id).unwrap_or(0);
        let target_idx = if forward {
            (cur_idx + 1) % windows.len()
        } else {
            (cur_idx + windows.len() - 1) % windows.len()
        };
        let target = &windows[target_idx];
        self.focus_window(target.id)?;
        Ok(target.clone())
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

    /// Retrieves current text content from the system clipboard.
    fn get_clipboard_text(&self) -> Result<Option<String>, CoreError> {
        Ok(None)
    }

    /// Sets current text content in the system clipboard without simulating paste.
    fn set_clipboard_text(&self, text: &str) -> Result<(), CoreError> {
        let _ = text;
        Ok(())
    }

    /// Clears the system clipboard.
    fn clear_clipboard(&self) -> Result<(), CoreError> {
        self.set_clipboard_text("")
    }

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

    /// Simulates directional scrolling with configurable wheel tick amount.
    fn scroll(&self, direction: ScrollDirection, amount: u32) -> Result<(), CoreError> {
        let _ = (direction, amount);
        Ok(())
    }

    /// Queries the current absolute desktop coordinates of the mouse cursor.
    fn get_pointer_position(&self) -> Result<PointerPosition, CoreError> {
        Ok(PointerPosition { x: 0, y: 0 })
    }
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

    /// Captures a raw chunk of system loopback audio from active output devices.
    /// Default implementation returns a silent 100ms buffer at 16kHz.
    fn capture_loopback_chunk(&self) -> Result<Vec<f32>, CoreError> {
        Ok(vec![0.0f32; 1600])
    }

    /// Captures synchronized microphone and system loopback chunks simultaneously.
    fn capture_dual_audio_chunk(&self) -> Result<(Vec<f32>, Vec<f32>), CoreError> {
        let mic = self.capture_audio_chunk()?;
        let loopback = self.capture_loopback_chunk()?;
        Ok((mic, loopback))
    }
}

/// Physical camera capture stream abstraction.
pub trait PhysicalCameraStream: Send + Sync {
    /// Reads the next frame from the camera stream, or returns None if no frame is ready.
    fn read_frame(&mut self) -> Result<Option<FrameBuffer>, CoreError>;
    /// Stops the capture stream and releases OS device handles.
    fn stop(&mut self) -> Result<(), CoreError>;
}

/// Virtual camera device sink abstraction ("AirCtrl Virtual Camera").
pub trait VirtualCameraDeviceSink: Send + Sync {
    /// Writes a frame buffer to the OS virtual camera device.
    fn write_frame(&mut self, frame: &FrameBuffer) -> Result<(), CoreError>;
    /// Returns the number of active video consumers connected to the virtual sink.
    fn consumer_count(&self) -> usize {
        1
    }
}

/// Low-level operating system camera capture and virtual device operations.
pub trait PlatformCameraOps: Send + Sync {
    /// Opens the physical camera capture stream.
    fn open_physical_camera_stream(
        &self,
        device_id: Option<&str>,
        resolution: CameraResolution,
        fps: u32,
    ) -> Result<Box<dyn PhysicalCameraStream>, CoreError>;

    /// Registers/creates the OS virtual camera device sink ("AirCtrl Virtual Camera").
    fn create_virtual_camera_sink(
        &self,
        config: &VirtualCameraConfig,
    ) -> Result<Box<dyn VirtualCameraDeviceSink>, CoreError>;

    /// Enumerates available camera capture devices.
    fn list_cameras(&self) -> Result<Vec<String>, CoreError> {
        Ok(vec!["Default Camera".to_string()])
    }
}

/// Aggregate container holding all platform abstraction providers.
#[derive(Clone)]
pub struct PlatformProvider {
    pub window: Arc<dyn PlatformWindowOps>,
    pub input: Arc<dyn PlatformInputOps>,
    pub audio: Arc<dyn PlatformAudioOps>,
    pub camera: Arc<dyn PlatformCameraOps>,
}

