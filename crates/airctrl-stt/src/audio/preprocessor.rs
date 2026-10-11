use crate::error::{Result, SttError};
use airctrl_core::types::AudioBuffer;

/// Audio preprocessor responsible for mono downmixing, linear resampling,
/// anti-aliasing decimation (48kHz -> 16kHz), and sample range normalization.
pub struct AudioPreprocessor {
    pub target_sample_rate: u32,
}

impl Default for AudioPreprocessor {
    fn default() -> Self {
        Self {
            target_sample_rate: 16_000,
        }
    }
}

impl AudioPreprocessor {
    pub fn new(target_sample_rate: u32) -> Self {
        Self { target_sample_rate }
    }

    /// Downmixes multi-channel interleaved float samples into single-channel mono by averaging channels.
    /// Clamps each output sample strictly within [-1.0, 1.0].
    pub fn downmix_to_mono(interleaved: &[f32], channels: u16) -> Vec<f32> {
        if channels <= 1 {
            return interleaved.iter().map(|&s| s.clamp(-1.0, 1.0)).collect();
        }

        let ch = channels as usize;
        let frames = interleaved.len() / ch;
        let mut mono = Vec::with_capacity(frames);

        for i in 0..frames {
            let mut sum = 0.0f32;
            for c in 0..ch {
                sum += interleaved[i * ch + c];
            }
            mono.push((sum / channels as f32).clamp(-1.0, 1.0));
        }

        mono
    }

    /// Resamples single-channel audio from `src_rate` to `dst_rate`.
    ///
    /// When resampling from standard 48,000 Hz to 16,000 Hz (3:1 integer decimation),
    /// applies a 3-tap anti-aliasing FIR filter (coefficients: 0.25, 0.50, 0.25).
    /// For other rate ratios, applies linear interpolation.
    pub fn resample_linear(samples: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
        if src_rate == dst_rate || samples.is_empty() {
            return samples.iter().map(|&s| s.clamp(-1.0, 1.0)).collect();
        }

        // Exact 3:1 integer decimation (48kHz -> 16kHz) with 3-tap triangle FIR filter
        if src_rate == 48_000 && dst_rate == 16_000 {
            let out_len = samples.len() / 3;
            let mut out = Vec::with_capacity(out_len);
            for i in 0..out_len {
                let idx = i * 3;
                let s0 = samples[idx];
                let s1 = samples[idx + 1];
                let s2 = samples[idx + 2];
                let filtered = 0.25 * s0 + 0.50 * s1 + 0.25 * s2;
                out.push(filtered.clamp(-1.0, 1.0));
            }
            return out;
        }

        // General fractional resampling via linear interpolation
        let ratio = src_rate as f64 / dst_rate as f64;
        let out_len = ((samples.len() as f64) / ratio).floor() as usize;
        let mut out = Vec::with_capacity(out_len);

        for k in 0..out_len {
            let src_idx = (k as f64) * ratio;
            let idx0 = src_idx.floor() as usize;
            let frac = (src_idx - (idx0 as f64)) as f32;
            let s0 = samples[idx0];
            let s1 = if idx0 + 1 < samples.len() {
                samples[idx0 + 1]
            } else {
                s0
            };
            let sample = (1.0 - frac) * s0 + frac * s1;
            out.push(sample.clamp(-1.0, 1.0));
        }

        out
    }

    /// Normalizes an `AudioBuffer` by downmixing to mono and resampling to the target sample rate.
    pub fn normalize_buffer(&self, buffer: &AudioBuffer) -> Result<Vec<f32>> {
        if buffer.channels == 0 {
            return Err(SttError::UnsupportedAudioFormat(
                "Buffer channels must be at least 1".to_string(),
            ));
        }
        let mono = Self::downmix_to_mono(&buffer.samples, buffer.channels);
        let resampled = Self::resample_linear(&mono, buffer.sample_rate, self.target_sample_rate);
        Ok(resampled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_downmix_to_mono() {
        let stereo = vec![0.5, -0.5, 1.0, 1.0, -1.0, -1.0];
        let mono = AudioPreprocessor::downmix_to_mono(&stereo, 2);
        assert_eq!(mono, vec![0.0, 1.0, -1.0]);
    }

    #[test]
    fn test_resample_48k_to_16k_decimation() {
        let input = vec![1.0, 1.0, 1.0, 0.0, 0.0, 0.0];
        let out = AudioPreprocessor::resample_linear(&input, 48_000, 16_000);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], 1.0);
        assert_eq!(out[1], 0.0);
    }

    #[test]
    fn test_resample_fractional() {
        let input = vec![0.0, 1.0];
        let out = AudioPreprocessor::resample_linear(&input, 16_000, 32_000);
        assert_eq!(out.len(), 4);
        assert_eq!(out[0], 0.0);
        assert_eq!(out[1], 0.5);
    }
}
