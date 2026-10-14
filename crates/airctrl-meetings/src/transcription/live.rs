use crate::audio::energy::compute_rms;
use crate::audio::mixer::DualAudioMixer;
use crate::error::{MeetingError, Result};
pub use crate::types::LiveTranscriptChunk;
use airctrl_stt::SpeechToTextEngine;

/// Real-time live meeting transcription engine with energy-ratio speaker attribution.
#[derive(Debug, Clone)]
pub struct LiveMeetingTranscriber {
    chunks: Vec<LiveTranscriptChunk>,
    pub current_time_ms: u64,
    default_confidence: f32,
}

impl Default for LiveMeetingTranscriber {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveMeetingTranscriber {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            current_time_ms: 0,
            default_confidence: 0.95,
        }
    }

    /// Evaluates speech energies, determines speaker attribution, advances monotonic
    /// timestamps, and records a transcript chunk.
    pub fn feed_audio(
        &mut self,
        mic_energy: f32,
        loopback_energy: f32,
        duration_ms: u64,
        transcribed_text: &str,
    ) -> LiveTranscriptChunk {
        let eps = 1e-6_f32;
        let mic_ratio = mic_energy / (loopback_energy + eps);
        let loop_ratio = loopback_energy / (mic_energy + eps);

        let speaker = if mic_ratio > 2.0 {
            "Host"
        } else if loop_ratio > 2.0 {
            "Participant"
        } else {
            "Mixed"
        };

        let start_time_ms = self.current_time_ms;
        let end_time_ms = start_time_ms + duration_ms;
        self.current_time_ms = end_time_ms;

        let chunk = LiveTranscriptChunk {
            id: format!("chunk_{}", self.chunks.len() + 1),
            speaker: speaker.to_string(),
            text: transcribed_text.to_string(),
            start_time_ms,
            end_time_ms,
            confidence: self.default_confidence,
        };

        self.chunks.push(chunk.clone());
        chunk
    }

    /// Convenience method matching default testing parameters (1000ms duration, default text).
    pub fn feed_audio_default(
        &mut self,
        mic_energy: f32,
        loopback_energy: f32,
    ) -> LiveTranscriptChunk {
        self.feed_audio(mic_energy, loopback_energy, 1000, "meeting notes chunk")
    }

    /// End-to-end processing: mixes raw PCM streams, runs Whisper inference,
    /// attributes speaker, and outputs a live transcript chunk.
    pub fn feed_pcm_frames(
        &mut self,
        mic_samples: &[f32],
        loopback_samples: &[f32],
        mixer: &mut DualAudioMixer,
        engine: &dyn SpeechToTextEngine,
    ) -> Result<Option<LiveTranscriptChunk>> {
        if mic_samples.is_empty() && loopback_samples.is_empty() {
            return Ok(None);
        }

        let mic_rms = compute_rms(mic_samples);
        let loop_rms = compute_rms(loopback_samples);

        // Mix audio with ducking and tanh limiter
        let mixed = mixer.mix(mic_samples, loopback_samples);
        if mixed.is_empty() {
            return Ok(None);
        }

        let duration_ms = ((mixed.len() as f64 / mixer.config().sample_rate as f64) * 1000.0) as u64;

        // Offline Whisper transcription
        let result = engine.transcribe(&mixed).map_err(|e| {
            MeetingError::AudioCaptureFailed(format!("STT failed: {e}"))
        })?;

        let chunk = self.feed_audio(mic_rms, loop_rms, duration_ms, &result.text);
        Ok(Some(chunk))
    }

    pub fn chunks(&self) -> &[LiveTranscriptChunk] {
        &self.chunks
    }

    pub fn current_time_ms(&self) -> u64 {
        self.current_time_ms
    }

    pub fn set_current_time_ms(&mut self, time_ms: u64) {
        self.current_time_ms = time_ms;
    }

    pub fn clear(&mut self) {
        self.chunks.clear();
        self.current_time_ms = 0;
    }
}
