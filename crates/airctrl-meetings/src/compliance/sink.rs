use airctrl_core::error::CoreError;
use airctrl_core::platform::PlatformAudioOps;
use std::sync::{Arc, Mutex};

/// Represents a played audio stream buffer and its sample rate.
pub type AudioBufferItem = (Vec<f32>, u32);

/// In-memory mock audio sink for capturing and validating played audio buffers in test environments.
#[derive(Debug, Default, Clone)]
pub struct MockAudioSink {
    played_buffers: Arc<Mutex<Vec<AudioBufferItem>>>,
}

impl MockAudioSink {
    pub fn new() -> Self {
        Self {
            played_buffers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn play(&self, samples: &[f32], sample_rate: u32) {
        if let Ok(mut buf) = self.played_buffers.lock() {
            buf.push((samples.to_vec(), sample_rate));
        }
    }

    pub fn played_count(&self) -> usize {
        self.played_buffers.lock().map(|b| b.len()).unwrap_or(0)
    }

    pub fn last_played(&self) -> Option<(Vec<f32>, u32)> {
        self.played_buffers.lock().ok()?.last().cloned()
    }

    pub fn clear(&self) {
        if let Ok(mut b) = self.played_buffers.lock() {
            b.clear();
        }
    }
}

impl PlatformAudioOps for MockAudioSink {
    fn play_audio_stream(&self, samples: &[f32], sample_rate: u32) -> Result<(), CoreError> {
        self.play(samples, sample_rate);
        Ok(())
    }

    fn capture_audio_chunk(&self) -> Result<Vec<f32>, CoreError> {
        Ok(Vec::new())
    }

    fn is_audio_output_active(&self) -> Result<bool, CoreError> {
        Ok(self.played_count() > 0)
    }

    fn list_audio_devices(&self) -> Result<Vec<String>, CoreError> {
        Ok(vec!["MockAudioDevice".to_string()])
    }
}
