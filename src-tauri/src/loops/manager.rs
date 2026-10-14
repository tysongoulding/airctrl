use super::audio::{AudioCaptureVadLoop, AudioVadConfig};
use super::hotkey::{GlobalHotkeyLoop, RawKeyEvent};
use super::meeting::MeetingWatcherLoop;
use super::metrics::*;
use super::vision::{VisionGestureConfig, VisionGestureLoop};
use crate::state::AppState;
use airctrl_core::events::EventBus;
use airctrl_gestures::FrameBuffer;
use airctrl_meetings::{MeetingWatcherConfig, ProcessScanItem};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

struct LoopHandles {
    root_token: CancellationToken,
    audio_token: Option<CancellationToken>,
    vision_token: Option<CancellationToken>,
    meeting_token: Option<CancellationToken>,
    hotkey_token: Option<CancellationToken>,
    audio_handle: Option<JoinHandle<()>>,
    vision_handle: Option<JoinHandle<()>>,
    meeting_handle: Option<JoinHandle<()>>,
    hotkey_handle: Option<JoinHandle<()>>,
    audio_feed_tx: Option<mpsc::Sender<Vec<f32>>>,
    frame_feed_tx: Option<mpsc::Sender<FrameBuffer>>,
    hotkey_feed_tx: Option<mpsc::Sender<RawKeyEvent>>,
    start_time: Option<Instant>,
}

pub struct WorkerLoopManager {
    inner: Mutex<LoopHandles>,
    pub is_started: AtomicBool,
    pub meeting_scan_override: Arc<RwLock<Option<Vec<ProcessScanItem>>>>,
    pub audio_metrics: Arc<RwLock<AudioVadHealthMetrics>>,
    pub vision_metrics: Arc<RwLock<VisionGestureHealthMetrics>>,
    pub meeting_metrics: Arc<RwLock<MeetingWatcherHealthMetrics>>,
    pub hotkey_metrics: Arc<RwLock<GlobalHotkeyHealthMetrics>>,
    pub event_bus: EventBus,
}

impl WorkerLoopManager {
    pub fn new(event_bus: EventBus) -> Self {
        let root_token = CancellationToken::new();
        let inner = LoopHandles {
            root_token,
            audio_token: None,
            vision_token: None,
            meeting_token: None,
            hotkey_token: None,
            audio_handle: None,
            vision_handle: None,
            meeting_handle: None,
            hotkey_handle: None,
            audio_feed_tx: None,
            frame_feed_tx: None,
            hotkey_feed_tx: None,
            start_time: None,
        };

        Self {
            inner: Mutex::new(inner),
            is_started: AtomicBool::new(false),
            meeting_scan_override: Arc::new(RwLock::new(None)),
            audio_metrics: Arc::new(RwLock::new(Default::default())),
            vision_metrics: Arc::new(RwLock::new(Default::default())),
            meeting_metrics: Arc::new(RwLock::new(Default::default())),
            hotkey_metrics: Arc::new(RwLock::new(Default::default())),
            event_bus,
        }
    }

    pub async fn start_all(&self, state: Arc<AppState>) -> Result<(), String> {
        self.stop_all().await?;

        let mut lock = self.inner.lock().await;
        let root_token = CancellationToken::new();
        lock.root_token = root_token.clone();
        lock.start_time = Some(Instant::now());

        // Pre-arm loop health metrics synchronously upon launch
        self.audio_metrics.write().is_running = true;
        self.vision_metrics.write().is_running = true;
        self.meeting_metrics.write().is_running = true;
        self.hotkey_metrics.write().is_running = true;

        // 1. Start Audio Loop
        let (audio_tx, audio_rx) = mpsc::channel(256);
        let audio_token = root_token.child_token();
        let audio_loop = AudioCaptureVadLoop::new(
            AudioVadConfig::default(),
            state.audio_buffer.clone(),
            self.event_bus.clone(),
            self.audio_metrics.clone(),
            audio_rx,
        );
        let audio_token_clone = audio_token.clone();
        let audio_handle = tokio::spawn(async move {
            audio_loop.run(audio_token_clone).await;
        });
        lock.audio_token = Some(audio_token);
        lock.audio_handle = Some(audio_handle);
        lock.audio_feed_tx = Some(audio_tx);

        // 2. Start Vision Loop
        let (frame_tx, frame_rx) = mpsc::channel(64);
        let vision_token = root_token.child_token();
        let vision_loop = VisionGestureLoop::new(
            VisionGestureConfig::default(),
            state.camera_kill_switch.clone(),
            self.event_bus.clone(),
            self.vision_metrics.clone(),
            frame_rx,
        )
        .with_splitter(state.virtual_camera_splitter.clone())
        .with_multi_hand_state(state.multi_hand_state.clone())
        .with_active_gestures(state.active_gestures.clone());
        let vision_token_clone = vision_token.clone();
        let vision_handle = tokio::spawn(async move {
            vision_loop.run(vision_token_clone).await;
        });
        lock.vision_token = Some(vision_token);
        lock.vision_handle = Some(vision_handle);
        lock.frame_feed_tx = Some(frame_tx);

        // 3. Start Meeting Watcher Loop
        let meeting_token = root_token.child_token();
        let compliance_injector = airctrl_meetings::get_global_compliance_injector();
        let meeting_loop = MeetingWatcherLoop::new(
            MeetingWatcherConfig {
                poll_interval_ms: 100,
                debounce_ticks: 2,
                ..Default::default()
            },
            state.camera_kill_switch.clone(),
            compliance_injector,
            self.event_bus.clone(),
            self.meeting_metrics.clone(),
        )
        .with_scan_override(self.meeting_scan_override.clone());
        let meeting_token_clone = meeting_token.clone();
        let meeting_handle = tokio::spawn(async move {
            meeting_loop.run(meeting_token_clone).await;
        });
        lock.meeting_token = Some(meeting_token);
        lock.meeting_handle = Some(meeting_handle);

        // 4. Start Hotkey Loop
        let (hotkey_tx, hotkey_rx) = mpsc::channel(128);
        let hotkey_token = root_token.child_token();
        let hotkey_loop = GlobalHotkeyLoop::new(
            self.event_bus.clone(),
            self.hotkey_metrics.clone(),
            hotkey_rx,
        );
        let hotkey_token_clone = hotkey_token.clone();
        let dial_overlay = state.dial_overlay.clone();
        let hotkey_handle = tokio::spawn(async move {
            hotkey_loop.run(hotkey_token_clone, dial_overlay).await;
        });
        lock.hotkey_token = Some(hotkey_token);
        lock.hotkey_handle = Some(hotkey_handle);
        lock.hotkey_feed_tx = Some(hotkey_tx);

        // Synchronously mark loop manager as started before releasing lock & returning
        self.is_started.store(true, Ordering::SeqCst);

        Ok(())
    }

    pub async fn stop_all(&self) -> Result<(), String> {
        self.is_started.store(false, Ordering::SeqCst);

        let mut lock = self.inner.lock().await;
        lock.root_token.cancel();

        if let Some(t) = lock.audio_token.take() {
            t.cancel();
        }
        if let Some(t) = lock.vision_token.take() {
            t.cancel();
        }
        if let Some(t) = lock.meeting_token.take() {
            t.cancel();
        }
        if let Some(t) = lock.hotkey_token.take() {
            t.cancel();
        }

        let timeout = std::time::Duration::from_millis(500);
        let handles = vec![
            lock.audio_handle.take(),
            lock.vision_handle.take(),
            lock.meeting_handle.take(),
            lock.hotkey_handle.take(),
        ];

        for h in handles.into_iter().flatten() {
            let _ = tokio::time::timeout(timeout, h).await;
        }

        lock.audio_feed_tx = None;
        lock.frame_feed_tx = None;
        lock.hotkey_feed_tx = None;
        lock.start_time = None;

        // Reset metrics is_running flags
        self.audio_metrics.write().is_running = false;
        self.vision_metrics.write().is_running = false;
        self.meeting_metrics.write().is_running = false;
        self.hotkey_metrics.write().is_running = false;

        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.is_started.load(Ordering::SeqCst)
    }

    pub fn get_health_metrics(&self) -> LoopHealthMetrics {
        let inner_guard = self.inner.try_lock();
        let uptime_secs = match inner_guard {
            Ok(ref lock) => lock.start_time.map(|t| t.elapsed().as_secs()).unwrap_or(0),
            Err(_) => 0,
        };

        let audio = self.audio_metrics.read().clone();
        let vision = self.vision_metrics.read().clone();
        let meeting = self.meeting_metrics.read().clone();
        let hotkey = self.hotkey_metrics.read().clone();
        let all_healthy = audio.is_running && vision.is_running && meeting.is_running && hotkey.is_running;

        LoopHealthMetrics {
            uptime_secs,
            all_healthy,
            audio_vad: audio,
            vision_gesture: vision,
            meeting_watcher: meeting,
            global_hotkey: hotkey,
        }
    }

    pub async fn feed_audio(&self, samples: Vec<f32>) -> Result<(), String> {
        let lock = self.inner.lock().await;
        if let Some(ref tx) = lock.audio_feed_tx {
            tx.send(samples).await.map_err(|e| e.to_string())
        } else {
            Err("Audio loop not running".to_string())
        }
    }

    pub async fn feed_frame(&self, frame: FrameBuffer) -> Result<(), String> {
        let lock = self.inner.lock().await;
        if let Some(ref tx) = lock.frame_feed_tx {
            tx.send(frame).await.map_err(|e| e.to_string())
        } else {
            Err("Vision loop not running".to_string())
        }
    }

    pub async fn feed_hotkey(&self, event: RawKeyEvent) -> Result<(), String> {
        let lock = self.inner.lock().await;
        if let Some(ref tx) = lock.hotkey_feed_tx {
            tx.send(event).await.map_err(|e| e.to_string())
        } else {
            Err("Hotkey loop not running".to_string())
        }
    }

    pub fn set_meeting_scan_override(&self, procs: Option<Vec<ProcessScanItem>>) {
        *self.meeting_scan_override.write() = procs;
    }
}
