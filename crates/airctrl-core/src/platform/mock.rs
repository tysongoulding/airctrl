use super::traits::{PlatformAudioOps, PlatformInputOps, PlatformProvider, PlatformWindowOps};
use crate::error::CoreError;
use crate::types::*;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};

#[derive(Debug, Default)]
struct MockWindowState {
    active_window: Option<WindowInfo>,
    windows: Vec<WindowInfo>,
    focus_history: Vec<u64>,
}

/// Thread-safe in-memory mock window operations provider.
#[derive(Clone, Default)]
pub struct MockWindowOps {
    state: Arc<RwLock<MockWindowState>>,
}

impl MockWindowOps {
    pub fn new() -> Self {
        Self::default()
    }

    /// Injects a window into the mock window manager table.
    pub fn add_window(&self, window: WindowInfo) {
        let mut state = self.state.write().unwrap();
        if state.active_window.is_none() && window.is_focused {
            state.active_window = Some(window.clone());
        }
        state.windows.push(window);
    }

    /// Sets the active focused window directly.
    pub fn set_active_window(&self, window: WindowInfo) {
        let mut state = self.state.write().unwrap();
        state.active_window = Some(window);
    }

    /// Returns the recorded chronological history of window IDs passed to `focus_window`.
    pub fn get_focus_history(&self) -> Vec<u64> {
        self.state.read().unwrap().focus_history.clone()
    }

    /// Clears all mock windows and focus history.
    pub fn clear(&self) {
        let mut state = self.state.write().unwrap();
        state.windows.clear();
        state.active_window = None;
        state.focus_history.clear();
    }
}

impl PlatformWindowOps for MockWindowOps {
    fn get_active_window(&self) -> Result<WindowInfo, CoreError> {
        let state = self.state.read().unwrap();
        state
            .active_window
            .clone()
            .ok_or_else(|| CoreError::Window("No active window in mock environment".to_string()))
    }

    fn focus_window(&self, id: u64) -> Result<(), CoreError> {
        let mut state = self.state.write().unwrap();

        // 1. Verify window exists before mutating state or history
        if !state.windows.iter().any(|w| w.id == id) {
            return Err(CoreError::WindowNotFound(id));
        }

        // 2. Ensure only the target window has is_focused == true, clearing all others
        let mut focused_window = None;
        for w in state.windows.iter_mut() {
            if w.id == id {
                w.is_focused = true;
                focused_window = Some(w.clone());
            } else {
                w.is_focused = false;
            }
        }

        // 3. Update active window and record in focus_history ONLY on success
        state.active_window = focused_window;
        state.focus_history.push(id);
        Ok(())
    }

    fn enumerate_windows(&self) -> Result<Vec<WindowInfo>, CoreError> {
        let state = self.state.read().unwrap();
        Ok(state.windows.clone())
    }

    fn minimize_window(&self, id: u64) -> Result<(), CoreError> {
        let mut state = self.state.write().unwrap();
        if let Some(w) = state.windows.iter_mut().find(|w| w.id == id) {
            w.is_minimized = true;
            Ok(())
        } else {
            Err(CoreError::WindowNotFound(id))
        }
    }

    fn close_window(&self, id: u64) -> Result<(), CoreError> {
        let mut state = self.state.write().unwrap();
        let initial_len = state.windows.len();
        state.windows.retain(|w| w.id != id);
        if state.windows.len() < initial_len {
            if state.active_window.as_ref().map(|w| w.id) == Some(id) {
                state.active_window = None;
            }
            Ok(())
        } else {
            Err(CoreError::WindowNotFound(id))
        }
    }

    fn center_window(&self, id: u64) -> Result<(), CoreError> {
        let mut state = self.state.write().unwrap();
        if let Some(w) = state.windows.iter_mut().find(|w| w.id == id) {
            let screen_w = 1920;
            let screen_h = 1080;
            let win_w = if w.bounds.width > 0 && w.bounds.width < screen_w {
                w.bounds.width
            } else {
                (screen_w * 7) / 10
            };
            let win_h = if w.bounds.height > 0 && w.bounds.height < screen_h {
                w.bounds.height
            } else {
                (screen_h * 8) / 10
            };
            w.bounds.width = win_w;
            w.bounds.height = win_h;
            w.bounds.x = ((screen_w - win_w) / 2) as i32;
            w.bounds.y = ((screen_h - win_h) / 2) as i32;
            w.is_focused = true;
            w.is_minimized = false;
            let updated = w.clone();
            state.active_window = Some(updated);
            Ok(())
        } else {
            Err(CoreError::WindowNotFound(id))
        }
    }
}

#[derive(Debug, Default)]
struct MockInputState {
    actions: Vec<RecordedInputAction>,
    pointer: PointerPosition,
}

/// Thread-safe in-memory mock input simulation provider.
#[derive(Clone, Default)]
pub struct MockInputOps {
    state: Arc<Mutex<MockInputState>>,
}

impl MockInputOps {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns all recorded input operations.
    pub fn get_recorded_actions(&self) -> Vec<RecordedInputAction> {
        self.state.lock().unwrap().actions.clone()
    }

    /// Returns all recorded strings sent via `type_text`.
    pub fn get_typed_strings(&self) -> Vec<String> {
        self.state
            .lock()
            .unwrap()
            .actions
            .iter()
            .filter_map(|a| match a {
                RecordedInputAction::TypedText(t) => Some(t.clone()),
                _ => None,
            })
            .collect()
    }

    /// Returns all recorded strings sent via `paste_text`.
    pub fn get_pasted_strings(&self) -> Vec<String> {
        self.state
            .lock()
            .unwrap()
            .actions
            .iter()
            .filter_map(|a| match a {
                RecordedInputAction::PastedText(t) => Some(t.clone()),
                _ => None,
            })
            .collect()
    }

    /// Clears the recorded input history.
    pub fn clear(&self) {
        let mut state = self.state.lock().unwrap();
        state.actions.clear();
    }
}

impl PlatformInputOps for MockInputOps {
    fn type_text(&self, text: &str) -> Result<(), CoreError> {
        let mut state = self.state.lock().unwrap();
        state
            .actions
            .push(RecordedInputAction::TypedText(text.to_string()));
        Ok(())
    }

    fn paste_text(&self, text: &str) -> Result<(), CoreError> {
        let mut state = self.state.lock().unwrap();
        state
            .actions
            .push(RecordedInputAction::PastedText(text.to_string()));
        Ok(())
    }

    fn send_key_combination(
        &self,
        modifiers: &[ModifierKey],
        key: InputKey,
    ) -> Result<(), CoreError> {
        let mut state = self.state.lock().unwrap();
        state.actions.push(RecordedInputAction::KeyCombination {
            modifiers: modifiers.to_vec(),
            key,
        });
        Ok(())
    }

    fn move_pointer(&self, x: i32, y: i32) -> Result<(), CoreError> {
        let mut state = self.state.lock().unwrap();
        state.pointer = PointerPosition { x, y };
        state.actions.push(RecordedInputAction::PointerMoved { x, y });
        Ok(())
    }

    fn click_pointer(&self, action: ClickAction) -> Result<(), CoreError> {
        let mut state = self.state.lock().unwrap();
        state.actions.push(RecordedInputAction::PointerClicked(action));
        Ok(())
    }
}

#[derive(Debug, Default)]
struct MockAudioState {
    played_buffers: Vec<(Vec<f32>, u32)>,
    capture_queue: VecDeque<Vec<f32>>,
    output_active: bool,
    devices: Vec<String>,
}

/// Thread-safe in-memory mock audio provider.
#[derive(Clone)]
pub struct MockAudioOps {
    state: Arc<Mutex<MockAudioState>>,
}

impl Default for MockAudioOps {
    fn default() -> Self {
        let state = MockAudioState {
            played_buffers: Vec::new(),
            capture_queue: VecDeque::new(),
            output_active: false,
            devices: vec![
                "Mock Microphone (16kHz Mono)".into(),
                "Mock Speaker (48kHz Stereo)".into(),
            ],
        };
        Self {
            state: Arc::new(Mutex::new(state)),
        }
    }
}

impl MockAudioOps {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueues synthetic audio PCM samples to be returned by `capture_audio_chunk`.
    pub fn enqueue_capture_samples(&self, samples: Vec<f32>) {
        let mut state = self.state.lock().unwrap();
        state.capture_queue.push_back(samples);
    }

    /// Returns recorded audio chunks passed to `play_audio_stream`.
    pub fn get_played_buffers(&self) -> Vec<(Vec<f32>, u32)> {
        self.state.lock().unwrap().played_buffers.clone()
    }

    /// Configures the return value of `is_audio_output_active`.
    pub fn set_output_active(&self, active: bool) {
        let mut state = self.state.lock().unwrap();
        state.output_active = active;
    }

    /// Clears played and captured audio queues.
    pub fn clear(&self) {
        let mut state = self.state.lock().unwrap();
        state.played_buffers.clear();
        state.capture_queue.clear();
    }
}

impl PlatformAudioOps for MockAudioOps {
    fn play_audio_stream(&self, samples: &[f32], sample_rate: u32) -> Result<(), CoreError> {
        let mut state = self.state.lock().unwrap();
        state.played_buffers.push((samples.to_vec(), sample_rate));
        Ok(())
    }

    fn capture_audio_chunk(&self) -> Result<Vec<f32>, CoreError> {
        let mut state = self.state.lock().unwrap();
        state.capture_queue.pop_front().ok_or_else(|| {
            CoreError::Audio("Mock audio capture queue underflow (no samples available)".to_string())
        })
    }

    fn is_audio_output_active(&self) -> Result<bool, CoreError> {
        let state = self.state.lock().unwrap();
        Ok(state.output_active)
    }

    fn list_audio_devices(&self) -> Result<Vec<String>, CoreError> {
        let state = self.state.lock().unwrap();
        Ok(state.devices.clone())
    }
}

/// Controller struct exposing direct inspection hooks for unit and integration testing.
pub struct MockPlatformControllers {
    pub window: MockWindowOps,
    pub input: MockInputOps,
    pub audio: MockAudioOps,
}

/// Creates a coupled `PlatformProvider` and test controller pair.
pub fn create_mock_platform() -> (PlatformProvider, MockPlatformControllers) {
    let mock_window = MockWindowOps::new();
    let mock_input = MockInputOps::new();
    let mock_audio = MockAudioOps::new();

    let provider = PlatformProvider {
        window: Arc::new(mock_window.clone()),
        input: Arc::new(mock_input.clone()),
        audio: Arc::new(mock_audio.clone()),
    };

    let controllers = MockPlatformControllers {
        window: mock_window,
        input: mock_input,
        audio: mock_audio,
    };

    (provider, controllers)
}
