use crate::error::MeetingError;
use std::f32::consts::PI;
use std::io::Cursor;

/// Synthesizes acoustic disclosure announcements into PCM buffers and WAV byte streams.
pub struct ComplianceAudioGenerator;

impl ComplianceAudioGenerator {
    /// Generates normalized float PCM samples for a compliance disclosure announcement.
    ///
    /// For default parameters (16kHz, 1450ms), produces exactly 23,200 mono samples.
    pub fn generate_pcm_buffer(sample_rate: u32, duration_ms: u32, volume: f32) -> Vec<f32> {
        let sample_count = ((sample_rate as u64 * duration_ms as u64) / 1000) as usize;
        let mut samples = Vec::with_capacity(sample_count);

        let vol = volume.clamp(0.0, 1.0);
        let chime_midpoint = sample_count / 2;

        for i in 0..sample_count {
            let t = i as f32 / sample_rate as f32;
            let progress = i as f32 / sample_count as f32;

            // Dual-tone earcon frequencies: 587.33 Hz (D5) -> 880.0 Hz (A5)
            let freq = if i < chime_midpoint {
                587.33
            } else {
                880.0
            };

            // Carrier sine + second harmonic
            let tone = (2.0 * PI * freq * t).sin() * 0.7 + (2.0 * PI * (freq * 2.0) * t).sin() * 0.3;

            // Speech-cadence modulation envelope (~4Hz syllable modulation)
            let cadence = 0.8 + 0.2 * (2.0 * PI * 4.0 * t).sin();

            // Smooth onset (50ms) and decay (100ms) envelope to prevent clicks
            let attack_samples = ((sample_rate as f32 * 0.05) as usize).max(1);
            let release_samples = ((sample_rate as f32 * 0.10) as usize).max(1);

            let envelope = if i < attack_samples {
                i as f32 / attack_samples as f32
            } else if i > sample_count - release_samples {
                (sample_count - i) as f32 / release_samples as f32
            } else {
                1.0 - 0.2 * progress
            };

            let sample = tone * cadence * envelope * vol * 0.8;
            samples.push(sample.clamp(-1.0, 1.0));
        }

        samples
    }

    /// Encodes float PCM samples into 16-bit PCM WAV byte buffer via hound.
    pub fn encode_wav_bytes(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>, MeetingError> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = Cursor::new(Vec::new());
        let mut writer = hound::WavWriter::new(&mut cursor, spec)
            .map_err(|e| MeetingError::ComplianceGenerationFailed(e.to_string()))?;

        for &sample in samples {
            let clamped = sample.clamp(-1.0, 1.0);
            let sample_i16 = (clamped * 32767.0).round() as i16;
            writer
                .write_sample(sample_i16)
                .map_err(|e| MeetingError::ComplianceGenerationFailed(e.to_string()))?;
        }

        writer
            .finalize()
            .map_err(|e| MeetingError::ComplianceGenerationFailed(e.to_string()))?;

        Ok(cursor.into_inner())
    }

    /// Convenience method synthesizing WAV bytes for a given duration and sample rate.
    pub fn generate_wav_bytes(
        sample_rate: u32,
        duration_ms: u32,
        volume: f32,
    ) -> Result<Vec<u8>, MeetingError> {
        let samples = Self::generate_pcm_buffer(sample_rate, duration_ms, volume);
        Self::encode_wav_bytes(&samples, sample_rate)
    }
}
