use serde::{Deserialize, Serialize};

/// Configuration parameters for the dual audio mixer, ducking, and anti-clipping limiter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MixerConfig {
    /// Target sample rate in Hz (default: 16,000).
    pub sample_rate: u32,
    /// Absolute mic amplitude threshold to trigger ducking (default: 0.05).
    pub ducking_threshold: f32,
    /// Attenuation gain factor applied to system loopback during speech (default: 0.25 / -12dB).
    pub ducking_gain: f32,
    /// Ducking attack time in milliseconds (default: 30.0 ms).
    pub attack_ms: f32,
    /// Ducking release time in milliseconds (default: 500.0 ms).
    pub release_ms: f32,
    /// Linear scale factor for microphone input (default: 1.0).
    pub mic_gain: f32,
    /// Linear scale factor for system loopback input (default: 1.0).
    pub loopback_gain: f32,
    /// Master toggle for voice ducking (default: true).
    pub ducking_enabled: bool,
    /// Master toggle for hyperbolic tangent limiter (default: true).
    pub anti_clipping_enabled: bool,
}

impl Default for MixerConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16_000,
            ducking_threshold: 0.05,
            ducking_gain: 0.25,
            attack_ms: 30.0,
            release_ms: 500.0,
            mic_gain: 1.0,
            loopback_gain: 1.0,
            ducking_enabled: true,
            anti_clipping_enabled: true,
        }
    }
}

/// Dynamic dual audio stream mixer with dynamic voice ducking and soft-knee limiter.
#[derive(Debug, Clone)]
pub struct DualAudioMixer {
    config: MixerConfig,
    attack_alpha: f32,
    release_alpha: f32,
    pub current_envelope: f32,
    pub current_duck_gain: f32,
    pub last_mic_energy: f32,
    pub last_loopback_energy: f32,
    total_samples_processed: u64,
}

impl Default for DualAudioMixer {
    fn default() -> Self {
        Self::new(MixerConfig::default())
    }
}

impl DualAudioMixer {
    pub fn new(config: MixerConfig) -> Self {
        let sample_rate = config.sample_rate.max(1) as f32;
        let attack_sec = (config.attack_ms / 1000.0).max(0.001);
        let release_sec = (config.release_ms / 1000.0).max(0.001);

        let attack_alpha = 1.0 - (-1.0 / (sample_rate * attack_sec)).exp();
        let release_alpha = 1.0 - (-1.0 / (sample_rate * release_sec)).exp();

        Self {
            config,
            attack_alpha,
            release_alpha,
            current_envelope: 0.0,
            current_duck_gain: 1.0,
            last_mic_energy: 0.0,
            last_loopback_energy: 0.0,
            total_samples_processed: 0,
        }
    }

    /// Simultaneously mixes microphone and loopback audio streams.
    /// Handles mismatched buffer lengths by zero-padding the shorter slice.
    /// Sanitizes non-finite IEEE 754 floats.
    pub fn mix(&mut self, mic_samples: &[f32], loopback_samples: &[f32]) -> Vec<f32> {
        let n = mic_samples.len().max(loopback_samples.len());
        if n == 0 {
            return Vec::new();
        }

        let mut mixed = Vec::with_capacity(n);
        let mut mic_sum_sq = 0.0f64;
        let mut loop_sum_sq = 0.0f64;

        for i in 0..n {
            let raw_m = mic_samples.get(i).copied().unwrap_or(0.0);
            let raw_lp = loopback_samples.get(i).copied().unwrap_or(0.0);

            // IEEE 754 float sanitization
            let m = if raw_m.is_finite() { raw_m } else { 0.0 } * self.config.mic_gain;
            let lp = if raw_lp.is_finite() { raw_lp } else { 0.0 } * self.config.loopback_gain;

            mic_sum_sq += (m * m) as f64;
            loop_sum_sq += (lp * lp) as f64;

            // 1. Envelope follower on microphone
            let rect_mic = m.abs();
            if rect_mic > self.current_envelope {
                self.current_envelope += self.attack_alpha * (rect_mic - self.current_envelope);
            } else {
                self.current_envelope += self.release_alpha * (rect_mic - self.current_envelope);
            }

            // 2. Ducking gain computation
            let target_gain = if self.config.ducking_enabled
                && self.current_envelope >= self.config.ducking_threshold
            {
                self.config.ducking_gain
            } else {
                1.0
            };

            if target_gain < self.current_duck_gain {
                self.current_duck_gain += self.attack_alpha * (target_gain - self.current_duck_gain);
            } else {
                self.current_duck_gain += self.release_alpha * (target_gain - self.current_duck_gain);
            }

            // Invariant: Ducking floor constraint
            if self.config.ducking_enabled && self.current_duck_gain < self.config.ducking_gain {
                self.current_duck_gain = self.config.ducking_gain;
            }

            // 3. Attenuation and summing
            let ducked_lp = lp * self.current_duck_gain;
            let raw_sum = m + ducked_lp;

            // 4. Anti-clipping soft-knee saturation
            let limited = if self.config.anti_clipping_enabled {
                raw_sum.tanh().clamp(-1.0, 1.0)
            } else {
                raw_sum.clamp(-1.0, 1.0)
            };

            mixed.push(limited);
        }

        self.last_mic_energy = (mic_sum_sq / n as f64).sqrt() as f32;
        self.last_loopback_energy = (loop_sum_sq / n as f64).sqrt() as f32;
        self.total_samples_processed += n as u64;

        mixed
    }

    pub fn current_duck_gain(&self) -> f32 {
        self.current_duck_gain
    }

    pub fn current_envelope(&self) -> f32 {
        self.current_envelope
    }

    pub fn is_ducking_active(&self) -> bool {
        self.current_duck_gain < 0.95
    }

    pub fn current_mic_energy(&self) -> f32 {
        self.last_mic_energy
    }

    pub fn current_loopback_energy(&self) -> f32 {
        self.last_loopback_energy
    }

    pub fn config(&self) -> &MixerConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut MixerConfig {
        &mut self.config
    }

    pub fn total_samples_processed(&self) -> u64 {
        self.total_samples_processed
    }

    pub fn reset(&mut self) {
        self.current_envelope = 0.0;
        self.current_duck_gain = 1.0;
        self.last_mic_energy = 0.0;
        self.last_loopback_energy = 0.0;
        self.total_samples_processed = 0;
    }
}
