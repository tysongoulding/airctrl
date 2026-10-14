use super::metrics::AudioVadHealthMetrics;
use airctrl_core::dag::{InferenceEvent, RawAudioChunkPayload, SensorEvent, SpeechDetectedPayload};
use airctrl_core::events::{DomainEvent, EventBus};
use airctrl_core::types::SpeechState;
use airctrl_stt::{
    AudioRingBuffer, AudioStreamConfig, StreamVadState, VadConfig, VoiceActivityDetector,
};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct AudioVadConfig {
    pub sample_rate: u32,
    pub frame_size: usize,
    pub hop_size: usize,
    pub energy_threshold: f32,
    pub dbfs_threshold: f32,
    pub pre_roll_samples: usize,
    pub post_roll_samples: usize,
    pub hangover_duration_ms: u32,
    pub ring_buffer_capacity: usize,
    pub tick_interval_ms: u64,
    pub min_speech_samples: usize,
}

impl Default for AudioVadConfig {
    fn default() -> Self {
        let stream_cfg = AudioStreamConfig::default();
        let vad_cfg = VadConfig::default();
        Self {
            sample_rate: stream_cfg.target_sample_rate,
            frame_size: vad_cfg.frame_size,
            hop_size: vad_cfg.hop_size,
            energy_threshold: vad_cfg.energy_threshold,
            dbfs_threshold: vad_cfg.dbfs_threshold,
            pre_roll_samples: vad_cfg.pre_roll_samples,
            post_roll_samples: vad_cfg.post_roll_samples,
            hangover_duration_ms: vad_cfg.hangover_duration_ms,
            ring_buffer_capacity: stream_cfg.ring_buffer_capacity,
            tick_interval_ms: 10,
            min_speech_samples: vad_cfg.min_samples,
        }
    }
}

pub struct AudioCaptureVadLoop {
    pub config: AudioVadConfig,
    pub ring_buffer: Arc<AudioRingBuffer>,
    pub app_audio_buffer: Arc<AudioRingBuffer>,
    pub vad: VoiceActivityDetector,
    pub event_bus: EventBus,
    pub metrics: Arc<RwLock<AudioVadHealthMetrics>>,
    pub sample_rx: mpsc::Receiver<Vec<f32>>,
}

impl AudioCaptureVadLoop {
    pub fn new(
        config: AudioVadConfig,
        app_audio_buffer: Arc<AudioRingBuffer>,
        event_bus: EventBus,
        metrics: Arc<RwLock<AudioVadHealthMetrics>>,
        sample_rx: mpsc::Receiver<Vec<f32>>,
    ) -> Self {
        let ring_buffer = Arc::new(AudioRingBuffer::new(config.ring_buffer_capacity));
        let vad_cfg = VadConfig {
            frame_size: config.frame_size,
            hop_size: config.hop_size,
            energy_threshold: config.energy_threshold,
            dbfs_threshold: config.dbfs_threshold,
            pre_roll_samples: config.pre_roll_samples,
            post_roll_samples: config.post_roll_samples,
            hangover_duration_ms: config.hangover_duration_ms,
            min_samples: config.min_speech_samples,
            ..Default::default()
        };
        let vad = VoiceActivityDetector::new(vad_cfg);

        Self {
            config,
            ring_buffer,
            app_audio_buffer,
            vad,
            event_bus,
            metrics,
            sample_rx,
        }
    }

    pub async fn run(mut self, token: CancellationToken) {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(self.config.tick_interval_ms));
        let mut current_utterance: Vec<f32> = Vec::new();
        let mut previous_state = StreamVadState::Silence;

        self.metrics.write().is_running = true;

        while !token.is_cancelled() {
            tokio::select! {
                _ = token.cancelled() => break,
                Some(new_samples) = self.sample_rx.recv() => {
                    if !new_samples.is_empty() {
                        self.ring_buffer.push(&new_samples);
                        self.app_audio_buffer.push(&new_samples);

                        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
                        let now_ms = now_ns / 1_000_000;

                        let _ = self.event_bus.publish_sensor(SensorEvent::RawAudioChunk(RawAudioChunkPayload {
                            samples: new_samples.clone(),
                            sample_rate: self.config.sample_rate,
                            channels: 1,
                            timestamp_ns: now_ns,
                        }));

                        self.metrics.write().total_samples_ingested += new_samples.len() as u64;

                        // Process incoming samples in frames
                        for chunk in new_samples.chunks(self.config.frame_size) {
                            let rms = self.vad.calculate_rms_energy(chunk);
                            let dbfs = self.vad.calculate_dbfs(chunk);
                            let current_state = self.vad.process_stream_frame(chunk);

                            let is_speech = current_state == StreamVadState::Speech || current_state == StreamVadState::Hangover;
                            let _ = self.event_bus.publish_inference(InferenceEvent::SpeechDetected(SpeechDetectedPayload {
                                is_speech,
                                energy_dbfs: dbfs,
                                hangover_active: current_state == StreamVadState::Hangover,
                                samples: chunk.to_vec(),
                                timestamp_ns: now_ns,
                            }));

                            {
                                let mut m = self.metrics.write();
                                m.last_rms_energy = rms;
                                m.last_dbfs = dbfs;
                                m.current_vad_state = format!("{:?}", current_state);
                                m.ring_buffer_len = self.ring_buffer.len();
                                m.total_frames_processed += 1;
                                m.last_tick_timestamp_ms = now_ms;
                            }

                            match (previous_state, current_state) {
                                (StreamVadState::Silence, StreamVadState::Speech) => {
                                    let pre_roll = self.ring_buffer.peek(self.config.pre_roll_samples);
                                    current_utterance.clear();
                                    current_utterance.extend(pre_roll);
                                    current_utterance.extend_from_slice(chunk);
                                    let _ = self.event_bus.publish(DomainEvent::SpeechStateChanged(SpeechState::Listening));
                                }
                                (StreamVadState::Hangover, StreamVadState::Silence) => {
                                    let (trimmed, _meta) = self.vad.trim_silence(&current_utterance);
                                    if trimmed.len() >= self.config.min_speech_samples {
                                        self.metrics.write().speech_segments_detected += 1;
                                        let _ = self.event_bus.publish(DomainEvent::SpeechStateChanged(SpeechState::Transcribing));
                                    } else {
                                        let _ = self.event_bus.publish(DomainEvent::SpeechStateChanged(SpeechState::Idle));
                                    }
                                    current_utterance.clear();
                                    self.vad.reset_stream();
                                }
                                _ => {
                                    if previous_state != StreamVadState::Silence {
                                        current_utterance.extend_from_slice(chunk);
                                    }
                                }
                            }
                            previous_state = current_state;
                        }
                    }
                }
                _ = interval.tick() => {
                    let now_ms = chrono::Utc::now().timestamp_millis() as u64;
                    self.metrics.write().last_tick_timestamp_ms = now_ms;
                }
            }
        }

        self.metrics.write().is_running = false;
    }
}
