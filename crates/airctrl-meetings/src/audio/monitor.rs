use super::energy::{compute_dbfs, compute_peak, compute_rms};
use super::hangover::HangoverStateMachine;
use crate::config::AudioMonitorConfig;
use serde::{Deserialize, Serialize};

/// Metrics computed from an evaluated audio chunk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioStreamMetrics {
    pub rms: f32,
    pub dbfs: f32,
    pub peak: f32,
    pub is_speech: bool,
    pub frame_count: u64,
}

impl Default for AudioStreamMetrics {
    fn default() -> Self {
        Self {
            rms: 0.0,
            dbfs: -120.0,
            peak: 0.0,
            is_speech: false,
            frame_count: 0,
        }
    }
}

/// Real-time audio stream monitor that computes signal metrics and tracks speech presence.
#[derive(Debug, Clone)]
pub struct AudioStreamMonitor {
    config: AudioMonitorConfig,
    hangover: HangoverStateMachine,
    frame_count: u64,
    latest_metrics: AudioStreamMetrics,
}

impl AudioStreamMonitor {
    pub fn new(config: AudioMonitorConfig) -> Self {
        let chunk_duration_ms =
            (config.chunk_size_samples as f64 / config.sample_rate as f64) * 1000.0;
        let min_speech_frames = ((config.min_speech_duration_ms as f64 / chunk_duration_ms).ceil()
            as usize)
            .max(1);
        let hangover_frames = (config.silence_timeout_ms as f64 / chunk_duration_ms).ceil() as usize;

        let hangover = HangoverStateMachine::new(min_speech_frames, hangover_frames);

        Self {
            config,
            hangover,
            frame_count: 0,
            latest_metrics: AudioStreamMetrics::default(),
        }
    }

    /// Evaluates a chunk of PCM audio samples and updates speech detection status.
    pub fn process_chunk(&mut self, samples: &[f32]) -> AudioStreamMetrics {
        self.frame_count += 1;

        let rms = compute_rms(samples);
        let dbfs = compute_dbfs(rms);
        let peak = compute_peak(samples);

        let frame_active =
            rms >= self.config.energy_threshold && dbfs >= self.config.silence_threshold_dbfs;

        let is_speech = self.hangover.update(frame_active);

        let metrics = AudioStreamMetrics {
            rms,
            dbfs,
            peak,
            is_speech,
            frame_count: self.frame_count,
        };

        self.latest_metrics = metrics.clone();
        metrics
    }

    /// Returns `true` if speech is currently active according to the hangover state machine.
    pub fn is_speech_active(&self) -> bool {
        self.hangover.is_speech_active()
    }

    /// Returns the latest computed audio metrics.
    pub fn latest_metrics(&self) -> &AudioStreamMetrics {
        &self.latest_metrics
    }

    /// Resets monitoring state and frame counter.
    pub fn reset(&mut self) {
        self.hangover.reset();
        self.frame_count = 0;
        self.latest_metrics = AudioStreamMetrics::default();
    }
}
