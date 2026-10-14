use super::metrics::VisionGestureHealthMetrics;
use airctrl_core::dag::{
    GestureRecognizedPayload, InferenceEvent, SensorEvent, ToggleOverlayPayload,
    OverlayType, VideoFramePayload, VideoPixelFormat, SinkAction,
};
use airctrl_core::events::{DomainEvent, EventBus};
use airctrl_core::types::gesture::{
    ActiveGesturesSnapshot, PinchData, PointingRayData, TwoFingerData,
};
use airctrl_gestures::{
    compute_pointing_ray, create_gesture_classifier, create_landmark_engine, is_pinch_at_threshold,
    pinch_distance, FrameBuffer, GestureClassifier, HandLandmarkEngine, PixelFormat,
    SpatialCoordinateMapper, TwoFingerGestureEvent, INDEX_TIP, THUMB_TIP,
};
use airctrl_meetings::CameraKillSwitch;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct VisionGestureConfig {
    pub target_fps: u32,
    pub sector_count: usize,
    pub deadzone_radius: f32,
}

impl Default for VisionGestureConfig {
    fn default() -> Self {
        Self {
            target_fps: 30,
            sector_count: 6,
            deadzone_radius: 0.10,
        }
    }
}

pub struct VisionGestureLoop {
    pub config: VisionGestureConfig,
    pub engine: Arc<dyn HandLandmarkEngine>,
    pub classifier: GestureClassifier,
    pub spatial_mapper: SpatialCoordinateMapper,
    pub camera_kill_switch: Arc<CameraKillSwitch>,
    pub event_bus: EventBus,
    pub metrics: Arc<RwLock<VisionGestureHealthMetrics>>,
    pub frame_rx: mpsc::Receiver<FrameBuffer>,
    pub virtual_camera_splitter: Option<Arc<airctrl_meetings::VirtualCameraSplitter>>,
    pub multi_hand_state: Arc<RwLock<airctrl_core::types::gesture::MultiHandLandmarks>>,
    pub active_gestures: Arc<RwLock<airctrl_core::types::gesture::ActiveGesturesSnapshot>>,
    pub two_finger_detector: airctrl_gestures::TwoFingerDetector,
}

impl VisionGestureLoop {
    pub fn new(
        config: VisionGestureConfig,
        camera_kill_switch: Arc<CameraKillSwitch>,
        event_bus: EventBus,
        metrics: Arc<RwLock<VisionGestureHealthMetrics>>,
        frame_rx: mpsc::Receiver<FrameBuffer>,
    ) -> Self {
        let engine = create_landmark_engine();
        let classifier = create_gesture_classifier();
        let spatial_mapper = SpatialCoordinateMapper::new(
            config.sector_count,
            (0.5, 0.5),
            config.deadzone_radius,
        );
        let multi_hand_state = Arc::new(RwLock::new(airctrl_core::types::gesture::MultiHandLandmarks::default()));
        let active_gestures = Arc::new(RwLock::new(airctrl_core::types::gesture::ActiveGesturesSnapshot::default()));
        let two_finger_detector = airctrl_gestures::TwoFingerDetector::new();

        Self {
            config,
            engine,
            classifier,
            spatial_mapper,
            camera_kill_switch,
            event_bus,
            metrics,
            frame_rx,
            virtual_camera_splitter: None,
            multi_hand_state,
            active_gestures,
            two_finger_detector,
        }
    }

    pub fn with_splitter(mut self, splitter: Arc<airctrl_meetings::VirtualCameraSplitter>) -> Self {
        self.virtual_camera_splitter = Some(splitter);
        self
    }

    pub fn with_multi_hand_state(mut self, state: Arc<RwLock<airctrl_core::types::gesture::MultiHandLandmarks>>) -> Self {
        self.multi_hand_state = state;
        self
    }

    pub fn with_active_gestures(mut self, gestures: Arc<RwLock<airctrl_core::types::gesture::ActiveGesturesSnapshot>>) -> Self {
        self.active_gestures = gestures;
        self
    }

    pub async fn run(mut self, token: CancellationToken) {
        let frame_interval_ms = (1000 / self.config.target_fps).max(1) as u64;
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(frame_interval_ms));
        let mut last_fps_check = tokio::time::Instant::now();
        let mut frames_in_window: u64 = 0;
        let mut was_camera_active = self.camera_kill_switch.is_camera_active();

        self.metrics.write().is_running = true;

        while !token.is_cancelled() {
            tokio::select! {
                _ = token.cancelled() => break,
                _ = interval.tick() => {
                    // 1. Camera Kill-Switch & Virtual Camera Splitter Safety Check
                    let camera_active = if let Some(ref splitter) = self.virtual_camera_splitter {
                        splitter.is_consumer_1_active()
                    } else {
                        self.camera_kill_switch.is_camera_active()
                    };

                    self.metrics.write().camera_active = camera_active;
                    if !camera_active {
                        // Drain and discard all pending frames to prevent stale frame
                        // accumulation while privacy kill-switch or dual mute is engaged.
                        while self.frame_rx.try_recv().is_ok() {}
                        was_camera_active = false;
                        continue;
                    }

                    // Flush boundary frames queued during the inactive-to-active transition
                    if !was_camera_active {
                        while self.frame_rx.try_recv().is_ok() {}
                        was_camera_active = true;
                        continue;
                    }

                    // 2. Frame Acquisition
                    let frame = match self.frame_rx.try_recv() {
                        Ok(f) => f,
                        Err(mpsc::error::TryRecvError::Empty) => continue,
                        Err(mpsc::error::TryRecvError::Disconnected) => break,
                    };

                    frames_in_window += 1;
                    if last_fps_check.elapsed().as_secs_f32() >= 1.0 {
                        let fps = frames_in_window as f32 / last_fps_check.elapsed().as_secs_f32();
                        self.metrics.write().actual_fps = fps;
                        frames_in_window = 0;
                        last_fps_check = tokio::time::Instant::now();
                    }

                    let now_ms = chrono::Utc::now().timestamp_millis() as u64;

                    // Publish VideoFrame sensor event to DAG
                    let format = match frame.format {
                        PixelFormat::Rgb8 => VideoPixelFormat::Rgb8,
                        PixelFormat::Rgba8 => VideoPixelFormat::Rgba8,
                        PixelFormat::Bgr8 => VideoPixelFormat::Bgr8,
                        PixelFormat::Bgra8 => VideoPixelFormat::Bgra8,
                    };
                    let _ = self.event_bus.publish_sensor(SensorEvent::VideoFrame(VideoFramePayload {
                        width: frame.width,
                        height: frame.height,
                        stride: frame.stride,
                        format,
                        data: frame.data.clone(),
                        timestamp_ns: now_ms * 1_000_000,
                    }));

                    // 3. Multi-Hand Landmark Inference
                    let multi_result = self.engine.process_frame_multi(&frame.as_ref());
                    if let Ok(multi) = multi_result {
                        *self.multi_hand_state.write() = multi.clone();
                        self.metrics.write().hands_detected_count = multi.hand_count() as u64;

                        let mut snapshot = ActiveGesturesSnapshot::default();

                        // 4. Gesture Heuristics on Primary Hand
                        if let Some(hand_landmarks) = multi.primary_hand() {
                            let ray = compute_pointing_ray(&hand_landmarks.landmarks);
                            snapshot.pointing_ray = Some(PointingRayData {
                                dx: ray.direction.x,
                                dy: ray.direction.y,
                                dz: ray.direction.z,
                                pitch_rad: ray.pitch_rad,
                                yaw_rad: ray.yaw_rad,
                            });

                            let p_dist = pinch_distance(
                                &hand_landmarks.landmarks[THUMB_TIP],
                                &hand_landmarks.landmarks[INDEX_TIP],
                            );
                            let is_pinch = is_pinch_at_threshold(p_dist, 0.050);
                            snapshot.pinch = PinchData {
                                is_pinching: is_pinch,
                                distance: p_dist,
                            };

                            let two_finger_evt = self.two_finger_detector.update(hand_landmarks, now_ms);
                            match two_finger_evt {
                                TwoFingerGestureEvent::Scroll { position, .. } => {
                                    snapshot.two_finger = TwoFingerData {
                                        scroll_position: position,
                                        is_active: true,
                                    };
                                }
                                TwoFingerGestureEvent::Resize { .. } => {
                                    snapshot.two_finger = TwoFingerData {
                                        scroll_position: 0.0,
                                        is_active: true,
                                    };
                                }
                                TwoFingerGestureEvent::None => {
                                    snapshot.two_finger = TwoFingerData {
                                        scroll_position: 0.0,
                                        is_active: false,
                                    };
                                }
                            }

                            if let Ok(recognized) = self.classifier.process_frame(hand_landmarks, now_ms) {
                                let index_tip = &hand_landmarks.landmarks[INDEX_TIP];
                                let pointer_state = self.spatial_mapper.map_landmark(index_tip);
                                let dial = pointer_state.to_dial_coordinates().unwrap_or(airctrl_core::types::gesture::DialCoordinates {
                                    angle_rad: pointer_state.angle_rad,
                                    normalized_radius: pointer_state.normalized_radius,
                                    active_sector: pointer_state.active_sector.unwrap_or(0),
                                    is_selected: pointer_state.is_selected,
                                });

                                // Update metrics
                                {
                                    let mut m = self.metrics.write();
                                    m.last_gesture = format!("{:?}", recognized.gesture);
                                    m.is_pinching = recognized.is_pinching;
                                    m.pinch_distance = recognized.pinch_distance.unwrap_or(1.0);
                                    m.active_sector = pointer_state.active_sector.unwrap_or(0);
                                    m.polar_angle_rad = pointer_state.angle_rad;
                                    m.normalized_radius = pointer_state.normalized_radius;
                                    m.total_frames_processed += 1;
                                    m.last_tick_timestamp_ms = now_ms;
                                }

                                // Publish gesture events
                                let gesture_payload = GestureRecognizedPayload {
                                    gesture: recognized.gesture,
                                    dial: dial.clone(),
                                    landmarks: Some(hand_landmarks.clone()),
                                    confidence: recognized.confidence,
                                    timestamp_ns: now_ms * 1_000_000,
                                };
                                let _ = self.event_bus.publish_inference(InferenceEvent::GestureRecognized(gesture_payload));

                                let _ = self.event_bus.publish(DomainEvent::GestureRecognized {
                                    gesture: recognized.gesture,
                                    dial: dial.clone(),
                                });

                                if recognized.is_pinching && dial.is_selected {
                                    let _ = self.event_bus.publish_sink(SinkAction::ToggleOverlay(ToggleOverlayPayload {
                                        overlay: OverlayType::RadialDial,
                                        visible: Some(true),
                                        active_sector: Some(dial.active_sector),
                                    }));
                                }
                            }
                        } else {
                            self.metrics.write().total_frames_processed += 1;
                        }

                        *self.active_gestures.write() = snapshot;
                    } else {
                        self.metrics.write().total_frames_processed += 1;
                    }
                }
            }
        }

        self.metrics.write().is_running = false;
    }
}
