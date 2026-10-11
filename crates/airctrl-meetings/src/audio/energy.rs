/// Computes the Root Mean Square (RMS) energy of an audio buffer: $\sqrt{\frac{1}{N} \sum_{i=1}^N s_i^2}$.
pub fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    let mean_sq = sum_sq / (samples.len() as f64);
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
    samples
        .iter()
        .fold(0.0_f32, |max, &s| max.max(s.abs()))
}
