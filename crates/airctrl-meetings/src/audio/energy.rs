/// Computes the Root Mean Square (RMS) energy of an audio buffer: $\sqrt{\frac{1}{N} \sum_{i=1}^N s_i^2}$.
pub fn compute_rms(samples: &[f32]) -> f32 {
    let len = samples.len();
    if len == 0 {
        return 0.0;
    }
    let mut sum_sq = 0.0_f64;
    let mut i = 0;
    while i + 4 <= len {
        let s0 = samples[i] as f64;
        let s1 = samples[i + 1] as f64;
        let s2 = samples[i + 2] as f64;
        let s3 = samples[i + 3] as f64;
        sum_sq += s0 * s0 + s1 * s1 + s2 * s2 + s3 * s3;
        i += 4;
    }
    while i < len {
        let s = samples[i] as f64;
        sum_sq += s * s;
        i += 1;
    }
    let mean_sq = sum_sq / (len as f64);
    let rms = mean_sq.sqrt() as f32;
    if rms.is_nan() {
        0.0
    } else {
        rms
    }
}

/// Computes the decibels relative to full scale (dBFS): $20 \log_{10}(\text{RMS} + 10^{-12})$.
/// Clamped between -120.0 dBFS (digital silence) and 0.0 dBFS (full scale).
pub fn compute_dbfs(rms: f32) -> f32 {
    let epsilon = 1e-12_f64;
    let val = (rms as f64).max(0.0) + epsilon;
    let dbfs = 20.0 * val.log10();
    (dbfs as f32).clamp(-120.0, 0.0)
}

/// Computes the peak absolute amplitude of an audio buffer.
pub fn compute_peak(samples: &[f32]) -> f32 {
    let len = samples.len();
    let mut max = 0.0_f32;
    let mut i = 0;
    while i + 4 <= len {
        let a0 = samples[i].abs();
        let a1 = samples[i + 1].abs();
        let a2 = samples[i + 2].abs();
        let a3 = samples[i + 3].abs();
        max = max.max(a0).max(a1).max(a2).max(a3);
        i += 4;
    }
    while i < len {
        max = max.max(samples[i].abs());
        i += 1;
    }
    if max.is_nan() {
        0.0
    } else {
        max
    }
}
