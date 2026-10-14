use crate::types::AudioCueType;
use std::f32::consts::PI;

/// Synthesizes acoustic audio feedback cues (earcons) matching Handy audio feedback cues.
pub struct AcousticCueSynthesizer;

impl AcousticCueSynthesizer {
    /// Synthesizes 16kHz float PCM samples for subtle Handy earcons:
    /// - `RecordingStart`: 440 Hz -> 880 Hz rising sweep (80ms)
    /// - `RecordingStop`: 587.33 Hz (D5) -> 880 Hz (A5) dual bell chime (120ms)
    /// - `Cancel`: 520 Hz -> 260 Hz falling blip (60ms)
    /// - `Error`: 180 Hz warning tone with 2nd harmonic (150ms)
    pub fn generate_cue_samples(cue: AudioCueType, sample_rate: u32, volume: f32) -> Vec<f32> {
        let vol = volume.clamp(0.0, 1.0);
        let sr = sample_rate as f32;

        match cue {
            AudioCueType::RecordingStart => {
                let duration_secs = 0.08;
                let total_samples = (sr * duration_secs) as usize;
                let mut samples = Vec::with_capacity(total_samples);

                for i in 0..total_samples {
                    let progress = i as f32 / total_samples as f32;
                    let freq = 440.0 * (2.0f32).powf(progress); // 440 to 880 Hz
                    let t = i as f32 / sr;
                    let envelope = (PI * progress).sin(); // Smooth half-sine bell envelope
                    let sample = (2.0 * PI * freq * t).sin() * envelope * vol * 0.25;
                    samples.push(sample.clamp(-1.0, 1.0));
                }
                samples
            }
            AudioCueType::RecordingStop => {
                let duration_secs = 0.12;
                let total_samples = (sr * duration_secs) as usize;
                let mut samples = Vec::with_capacity(total_samples);
                let mid = total_samples / 2;

                for i in 0..total_samples {
                    let t = i as f32 / sr;
                    let (freq, env) = if i < mid {
                        let p = i as f32 / mid as f32;
                        (587.33, (PI * p).sin())
                    } else {
                        let p = (i - mid) as f32 / (total_samples - mid) as f32;
                        (880.0, (PI * p).sin())
                    };
                    let tone = (2.0 * PI * freq * t).sin() * 0.7
                        + (2.0 * PI * (freq * 1.5) * t).sin() * 0.3;
                    let sample = tone * env * vol * 0.25;
                    samples.push(sample.clamp(-1.0, 1.0));
                }
                samples
            }
            AudioCueType::Cancel => {
                let duration_secs = 0.06;
                let total_samples = (sr * duration_secs) as usize;
                let mut samples = Vec::with_capacity(total_samples);

                for i in 0..total_samples {
                    let progress = i as f32 / total_samples as f32;
                    let freq = 520.0 - (260.0 * progress);
                    let t = i as f32 / sr;
                    let env = 1.0 - progress; // Linear decay
                    let sample = (2.0 * PI * freq * t).sin() * env * vol * 0.2;
                    samples.push(sample.clamp(-1.0, 1.0));
                }
                samples
            }
            AudioCueType::Error => {
                let duration_secs = 0.15;
                let total_samples = (sr * duration_secs) as usize;
                let mut samples = Vec::with_capacity(total_samples);

                for i in 0..total_samples {
                    let t = i as f32 / sr;
                    let progress = i as f32 / total_samples as f32;
                    let tone = (2.0 * PI * 180.0 * t).sin() * 0.7
                        + (2.0 * PI * 360.0 * t).sin() * 0.3;
                    let env = (1.0 - progress).powf(0.5);
                    let sample = tone * env * vol * 0.3;
                    samples.push(sample.clamp(-1.0, 1.0));
                }
                samples
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acoustic_cue_generation() {
        for cue in &[
            AudioCueType::RecordingStart,
            AudioCueType::RecordingStop,
            AudioCueType::Cancel,
            AudioCueType::Error,
        ] {
            let samples = AcousticCueSynthesizer::generate_cue_samples(*cue, 16_000, 0.8);
            assert!(!samples.is_empty());
            for &s in &samples {
                assert!((-1.0..=1.0).contains(&s));
            }
        }
    }
}
