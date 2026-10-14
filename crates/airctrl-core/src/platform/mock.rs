use super::traits::{
    PhysicalCameraStream, PlatformAudioOps, PlatformCameraOps, PlatformInputOps, PlatformProvider,
    PlatformWindowOps, VirtualCameraDeviceSink,
};
use crate::error::CoreError;
use crate::types::*;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};


#[derive(Debug)]
struct MockWindowState {
    active_window: Option<WindowInfo>,
    windows: Vec<WindowInfo>,
    focus_history: Vec<u64>,
    bounds_history: Vec<(u64, WindowRect)>,
    work_area: WindowRect,
    z_order: Vec<u64>,
}

impl Default for MockWindowState {
    fn default() -> Self {
        Self {
            active_window: None,
            windows: Vec::new(),
            focus_history: Vec::new(),
            bounds_history: Vec::new(),
            work_area: WindowRect { x: 0, y: 0, width: 1920, height: 1080 },
            z_order: Vec::new(),
        }
    }
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
        if !state.z_order.contains(&window.id) {
            state.z_order.push(window.id);
        }
        state.windows.push(window);
    }

    /// Sets the active focused window directly.
    pub fn set_active_window(&self, window: WindowInfo) {
        let mut state = self.state.write().unwrap();
        if !state.windows.iter().any(|w| w.id == window.id) {
            state.windows.push(window.clone());
            state.z_order.push(window.id);
        }
        state.active_window = Some(window);
    }

    /// Injects multiple windows in z-order for cycling tests.
    pub fn set_windows(&self, windows: Vec<WindowInfo>) {
        let mut state = self.state.write().unwrap();
        state.z_order = windows.iter().map(|w| w.id).collect();
        state.active_window = windows.iter().find(|w| w.is_focused).cloned();
        state.windows = windows;
    }

    /// Configures the simulated display work area.
    pub fn set_work_area(&self, area: WindowRect) {
        let mut state = self.state.write().unwrap();
        state.work_area = area;
    }

    /// Returns the recorded chronological history of window IDs passed to `focus_window`.
    pub fn get_focus_history(&self) -> Vec<u64> {
        self.state.read().unwrap().focus_history.clone()
    }

    /// Retrieves recorded bounds mutations for assertion verification.
    pub fn get_bounds_history(&self) -> Vec<(u64, WindowRect)> {
        self.state.read().unwrap().bounds_history.clone()
    }

    /// Returns the current z-order of window handles.
    pub fn get_z_order(&self) -> Vec<u64> {
        self.state.read().unwrap().z_order.clone()
    }

    /// Clears all mock windows and focus history.
    pub fn clear(&self) {
        let mut state = self.state.write().unwrap();
        state.windows.clear();
        state.active_window = None;
        state.focus_history.clear();
        state.bounds_history.clear();
        state.z_order.clear();
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
        let found = if let Some(w) = state.windows.iter_mut().find(|w| w.id == id) {
            w.is_minimized = true;
            w.bounds = WindowRect { x: 0, y: 0, width: 0, height: 0 };
            true
        } else {
            false
        };
        if found {
            state.bounds_history.push((id, WindowRect { x: 0, y: 0, width: 0, height: 0 }));
            Ok(())
        } else {
            Err(CoreError::WindowNotFound(id))
        }
    }

    fn close_window(&self, id: u64) -> Result<(), CoreError> {
        let mut state = self.state.write().unwrap();
        let initial_len = state.windows.len();
        state.windows.retain(|w| w.id != id);
        state.z_order.retain(|&win_id| win_id != id);
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
        let work = state.work_area;
        let updated = if let Some(w) = state.windows.iter_mut().find(|w| w.id == id) {
            let win_w = (work.width * 7) / 10;
            let win_h = (work.height * 7) / 10;
            w.bounds.width = win_w;
            w.bounds.height = win_h;
            w.bounds.x = work.x + ((work.width - win_w) / 2) as i32;
            w.bounds.y = work.y + ((work.height - win_h) / 2) as i32;
            w.is_focused = true;
            w.is_minimized = false;
            Some(w.clone())
        } else {
            None
        };
        if let Some(w) = updated {
            state.bounds_history.push((id, w.bounds));
            state.active_window = Some(w);
            Ok(())
        } else {
            Err(CoreError::WindowNotFound(id))
        }
    }

    fn set_window_bounds(&self, id: u64, bounds: WindowRect) -> Result<(), CoreError> {
        let mut state = self.state.write().unwrap();
        let updated = if let Some(w) = state.windows.iter_mut().find(|w| w.id == id) {
            w.bounds = bounds;
            w.is_minimized = bounds.width == 0 && bounds.height == 0;
            Some(w.clone())
        } else {
            None
        };
        if let Some(w) = updated {
            if state.active_window.as_ref().map(|a| a.id) == Some(id) {
                state.active_window = Some(w);
            }
            state.bounds_history.push((id, bounds));
            Ok(())
        } else {
            Err(CoreError::WindowNotFound(id))
        }
    }

    fn maximize_window(&self, id: u64) -> Result<(), CoreError> {
        let work = self.get_work_area()?;
        self.set_window_bounds(id, work)
    }

    fn restore_window(&self, id: u64) -> Result<(), CoreError> {
        let work = self.get_work_area()?;
        let restore_rect = WindowRect {
            x: work.x + 100,
            y: work.y + 100,
            width: 1024.min(work.width),
            height: 768.min(work.height),
        };
        self.set_window_bounds(id, restore_rect)
    }

    fn set_fullscreen(&self, id: u64, _fullscreen: bool) -> Result<(), CoreError> {
        let work = self.get_work_area()?;
        self.set_window_bounds(id, work)
    }

    fn get_work_area(&self) -> Result<WindowRect, CoreError> {
        let state = self.state.read().unwrap();
        Ok(state.work_area)
    }

    fn get_display_for_window(&self, id: u64) -> Result<WindowRect, CoreError> {
        let state = self.state.read().unwrap();
        if state.windows.iter().any(|w| w.id == id) {
            Ok(state.work_area)
        } else {
            Err(CoreError::WindowNotFound(id))
        }
    }

    fn cycle_window(&self, forward: bool) -> Result<WindowInfo, CoreError> {
        let mut state = self.state.write().unwrap();
        if state.windows.is_empty() {
            return Err(CoreError::Window("No windows available in mock environment".to_string()));
        }
        if state.windows.len() == 1 {
            let win = state.windows[0].clone();
            state.active_window = Some(win.clone());
            return Ok(win);
        }

        let cur_id = state.active_window.as_ref().map(|w| w.id).unwrap_or(0);
        let cur_idx = state.windows.iter().position(|w| w.id == cur_id).unwrap_or(0);
        let target_idx = if forward {
            (cur_idx + 1) % state.windows.len()
        } else {
            (cur_idx + state.windows.len() - 1) % state.windows.len()
        };

        let target_id = state.windows[target_idx].id;
        for w in state.windows.iter_mut() {
            w.is_focused = w.id == target_id;
        }
        let focused = state.windows[target_idx].clone();
        state.active_window = Some(focused.clone());
        state.focus_history.push(target_id);
        Ok(focused)
    }
}

#[derive(Debug, Default)]
struct MockInputState {
    actions: Vec<RecordedInputAction>,
    pointer: PointerPosition,
    clipboard: Option<String>,
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

    /// Returns the current simulated clipboard content.
    pub fn get_clipboard(&self) -> Option<String> {
        self.state.lock().unwrap().clipboard.clone()
    }

    /// Sets the simulated clipboard content directly in test setups.
    pub fn set_clipboard(&self, text: Option<String>) {
        self.state.lock().unwrap().clipboard = text;
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

    /// Returns all recorded scroll actions.
    pub fn get_scrolled_actions(&self) -> Vec<(ScrollDirection, u32)> {
        let state = self.state.lock().unwrap();
        state
            .actions
            .iter()
            .filter_map(|a| match a {
                RecordedInputAction::PointerScrolled { direction, amount } => {
                    Some((*direction, *amount))
                }
                _ => None,
            })
            .collect()
    }

    /// Returns all recorded key combinations.
    pub fn get_key_combinations(&self) -> Vec<(Vec<ModifierKey>, InputKey)> {
        let state = self.state.lock().unwrap();
        state
            .actions
            .iter()
            .filter_map(|a| match a {
                RecordedInputAction::KeyCombination { modifiers, key } => {
                    Some((modifiers.clone(), key.clone()))
                }
                _ => None,
            })
            .collect()
    }

    /// Clears the recorded input history.
    pub fn clear(&self) {
        let mut state = self.state.lock().unwrap();
        state.actions.clear();
        state.clipboard = None;
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
        state.clipboard = Some(text.to_string());
        state
            .actions
            .push(RecordedInputAction::PastedText(text.to_string()));
        Ok(())
    }

    fn get_clipboard_text(&self) -> Result<Option<String>, CoreError> {
        let state = self.state.lock().unwrap();
        Ok(state.clipboard.clone())
    }

    fn set_clipboard_text(&self, text: &str) -> Result<(), CoreError> {
        let mut state = self.state.lock().unwrap();
        state.clipboard = Some(text.to_string());
        Ok(())
    }

    fn clear_clipboard(&self) -> Result<(), CoreError> {
        let mut state = self.state.lock().unwrap();
        state.clipboard = None;
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

    fn scroll(&self, direction: ScrollDirection, amount: u32) -> Result<(), CoreError> {
        let mut state = self.state.lock().unwrap();
        state.actions.push(RecordedInputAction::PointerScrolled {
            direction,
            amount,
        });
        Ok(())
    }

    fn get_pointer_position(&self) -> Result<PointerPosition, CoreError> {
        let state = self.state.lock().unwrap();
        Ok(state.pointer)
    }
}

#[derive(Debug, Default)]
struct MockAudioState {
    played_buffers: Vec<(Vec<f32>, u32)>,
    capture_queue: VecDeque<Vec<f32>>,
    loopback_queue: VecDeque<Vec<f32>>,
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
            loopback_queue: VecDeque::new(),
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

    /// Enqueues synthetic loopback audio PCM samples to be returned by `capture_loopback_chunk`.
    pub fn enqueue_loopback_samples(&self, samples: Vec<f32>) {
        let mut state = self.state.lock().unwrap();
        state.loopback_queue.push_back(samples);
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
        state.loopback_queue.clear();
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

    fn capture_loopback_chunk(&self) -> Result<Vec<f32>, CoreError> {
        let mut state = self.state.lock().unwrap();
        state.loopback_queue.pop_front().ok_or_else(|| {
            CoreError::Audio("Mock loopback capture queue underflow (no samples available)".to_string())
        })
    }

    fn capture_dual_audio_chunk(&self) -> Result<(Vec<f32>, Vec<f32>), CoreError> {
        let mut state = self.state.lock().unwrap();
        let mic = state.capture_queue.pop_front().ok_or_else(|| {
            CoreError::Audio("Mock audio capture queue underflow (no samples available)".to_string())
        })?;
        let loopback = state.loopback_queue.pop_front().ok_or_else(|| {
            CoreError::Audio("Mock loopback capture queue underflow (no samples available)".to_string())
        })?;
        Ok((mic, loopback))
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

/// Thread-safe in-memory mock physical camera stream.
pub struct MockPhysicalCameraStream {
    frames: Arc<Mutex<VecDeque<FrameBuffer>>>,
    stopped: Arc<AtomicBool>,
}

impl MockPhysicalCameraStream {
    pub fn new(frames: Arc<Mutex<VecDeque<FrameBuffer>>>, stopped: Arc<AtomicBool>) -> Self {
        Self { frames, stopped }
    }
}

impl PhysicalCameraStream for MockPhysicalCameraStream {
    fn read_frame(&mut self) -> Result<Option<FrameBuffer>, CoreError> {
        if self.stopped.load(Ordering::SeqCst) {
            return Err(CoreError::DeviceNotFound("Camera stream stopped".into()));
        }
        let mut queue = self.frames.lock().unwrap();
        Ok(queue.pop_front())
    }

    fn stop(&mut self) -> Result<(), CoreError> {
        self.stopped.store(true, Ordering::SeqCst);
        Ok(())
    }
}

/// Thread-safe in-memory mock virtual camera sink ("AirCtrl Virtual Camera").
pub struct MockVirtualCameraSink {
    pub device_name: String,
    pub written_frames: Arc<Mutex<Vec<FrameBuffer>>>,
    pub consumer_count: Arc<AtomicUsize>,
}

impl VirtualCameraDeviceSink for MockVirtualCameraSink {
    fn write_frame(&mut self, frame: &FrameBuffer) -> Result<(), CoreError> {
        self.written_frames.lock().unwrap().push(frame.clone());
        Ok(())
    }

    fn consumer_count(&self) -> usize {
        self.consumer_count.load(Ordering::Relaxed)
    }
}

/// Thread-safe in-memory mock camera operations provider.
#[derive(Clone)]
pub struct MockCameraOps {
    capture_frames: Arc<Mutex<VecDeque<FrameBuffer>>>,
    written_frames: Arc<Mutex<Vec<FrameBuffer>>>,
    consumer_count: Arc<AtomicUsize>,
    stream_stopped: Arc<AtomicBool>,
}

impl Default for MockCameraOps {
    fn default() -> Self {
        Self {
            capture_frames: Arc::new(Mutex::new(VecDeque::new())),
            written_frames: Arc::new(Mutex::new(Vec::new())),
            consumer_count: Arc::new(AtomicUsize::new(2)),
            stream_stopped: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl MockCameraOps {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueues synthetic video frame to be read from `read_frame`.
    pub fn enqueue_frame(&self, frame: FrameBuffer) {
        self.capture_frames.lock().unwrap().push_back(frame);
    }

    /// Retrieves all frames written to the virtual camera sink.
    pub fn get_written_frames(&self) -> Vec<FrameBuffer> {
        self.written_frames.lock().unwrap().clone()
    }

    /// Sets simulated active consumer count.
    pub fn set_consumer_count(&self, count: usize) {
        self.consumer_count.store(count, Ordering::Relaxed);
    }

    /// Clears frame queues and resets stop flag.
    pub fn clear(&self) {
        self.capture_frames.lock().unwrap().clear();
        self.written_frames.lock().unwrap().clear();
        self.stream_stopped.store(false, Ordering::SeqCst);
    }
}

impl PlatformCameraOps for MockCameraOps {
    fn open_physical_camera_stream(
        &self,
        _device_id: Option<&str>,
        _resolution: CameraResolution,
        _fps: u32,
    ) -> Result<Box<dyn PhysicalCameraStream>, CoreError> {
        self.stream_stopped.store(false, Ordering::SeqCst);
        Ok(Box::new(MockPhysicalCameraStream::new(
            self.capture_frames.clone(),
            self.stream_stopped.clone(),
        )))
    }

    fn create_virtual_camera_sink(
        &self,
        config: &VirtualCameraConfig,
    ) -> Result<Box<dyn VirtualCameraDeviceSink>, CoreError> {
        Ok(Box::new(MockVirtualCameraSink {
            device_name: config.device_name.clone(),
            written_frames: self.written_frames.clone(),
            consumer_count: self.consumer_count.clone(),
        }))
    }
}

/// Controller struct exposing direct inspection hooks for unit and integration testing.
pub struct MockPlatformControllers {
    pub window: MockWindowOps,
    pub input: MockInputOps,
    pub audio: MockAudioOps,
    pub camera: MockCameraOps,
}

/// Creates a coupled `PlatformProvider` and test controller pair.
pub fn create_mock_platform() -> (PlatformProvider, MockPlatformControllers) {
    let mock_window = MockWindowOps::new();
    let mock_input = MockInputOps::new();
    let mock_audio = MockAudioOps::new();
    let mock_camera = MockCameraOps::new();

    let provider = PlatformProvider {
        window: Arc::new(mock_window.clone()),
        input: Arc::new(mock_input.clone()),
        audio: Arc::new(mock_audio.clone()),
        camera: Arc::new(mock_camera.clone()),
    };

    let controllers = MockPlatformControllers {
        window: mock_window,
        input: mock_input,
        audio: mock_audio,
        camera: mock_camera,
    };

    (provider, controllers)
}

