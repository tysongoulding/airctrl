use super::generator::ComplianceAudioGenerator;
use super::sink::MockAudioSink;
use crate::config::ComplianceConfig;
use crate::error::MeetingError;
use crate::types::CompliancePlaybackResult;
use airctrl_core::platform::PlatformAudioOps;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Injects compliance recording disclosure audio into the active audio stream.
///
/// Enforces a strict legal volume floor of 0.50 (cannot be muted) and gates
/// recording sessions until the disclosure announcement has finished playing.
pub struct ComplianceAudioInjector {
    audio_ops: Option<Arc<dyn PlatformAudioOps>>,
    fallback_sink: MockAudioSink,
    recording_allowed: AtomicBool,
}

impl Default for ComplianceAudioInjector {
    fn default() -> Self {
        Self::new()
    }
}

impl ComplianceAudioInjector {
    /// Creates a new injector using the internal mock audio sink.
    pub fn new() -> Self {
        Self {
            audio_ops: None,
            fallback_sink: MockAudioSink::new(),
            recording_allowed: AtomicBool::new(false),
        }
    }

    /// Creates an injector backed by a specific `PlatformAudioOps` provider.
    pub fn with_audio_ops(audio_ops: Arc<dyn PlatformAudioOps>) -> Self {
        Self {
            audio_ops: Some(audio_ops),
            fallback_sink: MockAudioSink::new(),
            recording_allowed: AtomicBool::new(false),
        }
    }

    /// Plays the compliance announcement, enforcing the volume floor $\ge 0.50$.
    pub fn play_announcement(
        &self,
        config: &ComplianceConfig,
    ) -> Result<CompliancePlaybackResult, MeetingError> {
        // Enforce legal compliance volume floor: cannot be muted below 0.50
        let effective_vol = config.effective_volume();

        let samples = ComplianceAudioGenerator::generate_pcm_buffer(
            config.sample_rate,
            config.duration_ms,
            effective_vol,
        );

        if let Some(ref ops) = self.audio_ops {
            ops.play_audio_stream(&samples, config.sample_rate)
                .map_err(|e| MeetingError::AudioPlaybackFailed(e.to_string()))?;
        } else {
            self.fallback_sink.play(&samples, config.sample_rate);
        }

        // Disclosure complete: unlock recording session
        self.recording_allowed.store(true, Ordering::SeqCst);

        Ok(CompliancePlaybackResult {
            announcement: config.message.as_str().to_string(),
            duration_ms: config.duration_ms,
            injected_into_stream: true,
            status: "Success".to_string(),
        })
    }

    /// Checks if recording is currently permitted under legal compliance gating.
    pub fn is_recording_allowed(&self) -> bool {
        self.recording_allowed.load(Ordering::SeqCst)
    }

    /// Resets the recording gate (recording disallowed until next disclosure playback).
    pub fn reset_recording_gate(&self) {
        self.recording_allowed.store(false, Ordering::SeqCst);
    }

    /// Returns a reference to the internal fallback sink for inspection in tests.
    pub fn fallback_sink(&self) -> &MockAudioSink {
        &self.fallback_sink
    }
}
