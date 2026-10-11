use crate::config::VadConfig;
use serde::{Deserialize, Serialize};

/// Discard reasons when audio is rejected by the VAD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscardReason {
    EmptyBuffer,
    TooShort,
    BelowEnergyThreshold,
}

/// Metadata describing silence trimming results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrimMetadata {
    pub original_samples: usize,
    pub trimmed_samples: usize,
    pub leading_trimmed_samples: usize,
    pub trailing_trimmed_samples: usize,
    pub average_rms: f32,
    pub peak_dbfs: f32,
}

/// Categorized decision produced by the VAD.
#[derive(Debug, Clone, PartialEq)]
pub enum VadDecision {
    Speech {
        trimmed_samples: Vec<f32>,
        metadata: TrimMetadata,
    },
    Silence {
        average_rms: f32,
        peak_dbfs: f32,
    },
    Discarded {
        reason: DiscardReason,
        sample_count: usize,
    },
}

/// State of the streaming VAD hangover state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamVadState {
    Silence,
    Speech,
    Hangover,
}

/// Voice Activity Detector (VAD) evaluating energy, dBFS, and utterance boundaries.
pub struct VoiceActivityDetector {
    pub config: VadConfig,
    state: StreamVadState,
    hangover_frames_remaining: usize,
}

impl Default for VoiceActivityDetector {
    fn default() -> Self {
        Self::new(VadConfig::default())
    }
}

impl VoiceActivityDetector {
    pub fn new(config: VadConfig) -> Self {
        Self {
            config,
            state: StreamVadState::Silence,
            hangover_frames_remaining: 0,
        }
    }

    /// Computes Root-Mean-Square (RMS) energy: sqrt(sum(s^2) / N).
    /// Returns 0.0 for empty slices.
    pub fn calculate_rms_energy(&self, samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
        (sum_sq / (samples.len() as f64)).sqrt() as f32
    }

    /// Computes Decibels relative to Full Scale: 20 * log10(RMS + 1e-12).
    pub fn calculate_dbfs(&self, samples: &[f32]) -> f32 {
        let rms = self.calculate_rms_energy(samples);
        if rms <= 1e-9 {
            return -120.0;
        }
        (20.0 * (rms as f64 + 1e-12).log10()) as f32
    }

    /// Evaluates if the buffer contains active speech.
    ///
    /// Invariants:
    /// - Returns false if len < min_samples (default: 1600 samples).
    /// - Returns true if RMS >= energy_threshold (default: 0.05).
    pub fn is_speech(&self, samples: &[f32]) -> bool {
        if samples.len() < self.config.min_samples {
            return false;
        }
        let energy = self.calculate_rms_energy(samples);
        energy >= self.config.energy_threshold
    }

    /// Trims leading and trailing silence while preserving pre-roll and post-roll samples.
    pub fn trim_silence(&self, samples: &[f32]) -> (Vec<f32>, TrimMetadata) {
        let original_len = samples.len();
        if original_len == 0 {
            return (
                Vec::new(),
                TrimMetadata {
                    original_samples: 0,
                    trimmed_samples: 0,
                    leading_trimmed_samples: 0,
                    trailing_trimmed_samples: 0,
                    average_rms: 0.0,
                    peak_dbfs: -120.0,
                },
            );
        }

        let frame_size = self.config.frame_size.max(1);
        let hop_size = self.config.hop_size.max(1);
        let threshold = self.config.energy_threshold;

        // 1. Scan forward for speech onset
        let mut first_speech_idx = None;
        let mut idx = 0;
        while idx + frame_size <= original_len {
            let frame = &samples[idx..idx + frame_size];
            if self.calculate_rms_energy(frame) >= threshold {
                first_speech_idx = Some(idx);
                break;
            }
            idx += hop_size;
        }

        let start_frame = match first_speech_idx {
            Some(i) => i,
            None => {
                return (
                    Vec::new(),
                    TrimMetadata {
                        original_samples: original_len,
                        trimmed_samples: 0,
                        leading_trimmed_samples: original_len,
                        trailing_trimmed_samples: 0,
                        average_rms: self.calculate_rms_energy(samples),
                        peak_dbfs: self.calculate_dbfs(samples),
                    },
                );
            }
        };

        // 2. Scan backward for speech offset
        let mut last_speech_idx = original_len;
        let mut r_idx = original_len.saturating_sub(frame_size);
        while r_idx >= start_frame {
            let frame = &samples[r_idx..r_idx + frame_size];
            if self.calculate_rms_energy(frame) >= threshold {
                last_speech_idx = r_idx + frame_size;
                break;
            }
            if r_idx < hop_size {
                break;
            }
            r_idx -= hop_size;
        }

        // 3. Apply pre-roll and post-roll margins
        let trim_start = start_frame.saturating_sub(self.config.pre_roll_samples);
        let trim_end = (last_speech_idx + self.config.post_roll_samples).min(original_len);

        let trimmed = if trim_start < trim_end {
            samples[trim_start..trim_end].to_vec()
        } else {
            Vec::new()
        };

        let trimmed_len = trimmed.len();
        let metadata = TrimMetadata {
            original_samples: original_len,
            trimmed_samples: trimmed_len,
            leading_trimmed_samples: trim_start,
            trailing_trimmed_samples: original_len.saturating_sub(trim_end),
            average_rms: self.calculate_rms_energy(&trimmed),
            peak_dbfs: self.calculate_dbfs(&trimmed),
        };

        (trimmed, metadata)
    }

    /// Evaluates the entire buffer and categorizes the decision.
    pub fn evaluate(&self, samples: &[f32]) -> VadDecision {
        if samples.is_empty() {
            return VadDecision::Discarded {
                reason: DiscardReason::EmptyBuffer,
                sample_count: 0,
            };
        }

        if samples.len() < self.config.min_samples {
            return VadDecision::Discarded {
                reason: DiscardReason::TooShort,
                sample_count: samples.len(),
            };
        }

        let rms = self.calculate_rms_energy(samples);
        if rms < self.config.energy_threshold {
            return VadDecision::Silence {
                average_rms: rms,
                peak_dbfs: self.calculate_dbfs(samples),
            };
        }

        let (trimmed, metadata) = self.trim_silence(samples);
        if trimmed.is_empty() {
            return VadDecision::Silence {
                average_rms: rms,
                peak_dbfs: self.calculate_dbfs(samples),
            };
        }

        VadDecision::Speech {
            trimmed_samples: trimmed,
            metadata,
        }
    }

    /// Processes a single streaming frame with hangover state machine smoothing.
    pub fn process_stream_frame(&mut self, frame: &[f32]) -> StreamVadState {
        let frame_rms = self.calculate_rms_energy(frame);
        let is_active = frame_rms >= self.config.energy_threshold;

        // Approx number of frames in hangover duration:
        // hangover_ms * (16 samples/ms) / frame_len
        let frame_len = frame.len().max(1);
        let hangover_frames =
            (self.config.hangover_duration_ms as usize * 16) / frame_len;

        if is_active {
            self.state = StreamVadState::Speech;
            self.hangover_frames_remaining = hangover_frames;
        } else if self.hangover_frames_remaining > 0 {
            self.hangover_frames_remaining -= 1;
            self.state = StreamVadState::Hangover;
        } else {
            self.state = StreamVadState::Silence;
        }

        self.state
    }

    /// Resets streaming state to silence.
    pub fn reset_stream(&mut self) {
        self.state = StreamVadState::Silence;
        self.hangover_frames_remaining = 0;
    }

    pub fn current_state(&self) -> StreamVadState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rms_calculation() {
        let vad = VoiceActivityDetector::default();
        assert_eq!(vad.calculate_rms_energy(&[]), 0.0);

        let samples = vec![0.5; 100];
        let rms = vad.calculate_rms_energy(&samples);
        assert!((rms - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_is_speech_boundary_conditions() {
        let vad = VoiceActivityDetector::default(); // min_samples = 1600, threshold = 0.05

        // Exactly 1599 samples rejected
        let short_buffer = vec![1.0; 1599];
        assert!(!vad.is_speech(&short_buffer));

        // Exactly 1600 samples with high energy accepted
        let valid_buffer = vec![1.0; 1600];
        assert!(vad.is_speech(&valid_buffer));

        // Energy threshold boundaries at 1600 samples
        let low_energy = vec![0.049; 1600];
        assert!(!vad.is_speech(&low_energy));

        let exact_energy = vec![0.050; 1600];
        assert!(vad.is_speech(&exact_energy));

        let high_energy = vec![0.051; 1600];
        assert!(vad.is_speech(&high_energy));
    }

    #[test]
    fn test_hangover_state_machine() {
        let mut vad = VoiceActivityDetector::default();
        let loud_frame = vec![0.2; 320];
        let silent_frame = vec![0.0; 320];

        // Loud frame triggers speech
        assert_eq!(vad.process_stream_frame(&loud_frame), StreamVadState::Speech);

        // Silent frame enters hangover
        assert_eq!(vad.process_stream_frame(&silent_frame), StreamVadState::Hangover);
    }
}
