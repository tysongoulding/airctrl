/// Downmixes interleaved multi-channel audio samples into a single-channel mono buffer.
pub fn downmix_to_mono(interleaved: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.iter().map(|&s| s.clamp(-1.0, 1.0)).collect();
    }
    let ch = channels as usize;
    let frames = interleaved.len() / ch;
    let mut mono = Vec::with_capacity(frames);
    let inv_ch = 1.0 / channels as f32;

    for i in 0..frames {
        let mut sum = 0.0f32;
        let base = i * ch;
        for c in 0..ch {
            sum += interleaved[base + c];
        }
        mono.push((sum * inv_ch).clamp(-1.0, 1.0));
    }
    mono
}

/// Resamples a single-channel PCM audio stream to standard 16,000 Hz.
/// For 48kHz, performs exact 3:1 integer decimation with a 3-tap FIR anti-aliasing filter.
/// For other rates, applies fractional linear interpolation.
pub fn resample_to_16k(samples: &[f32], source_rate: u32) -> Vec<f32> {
    if source_rate == 16_000 || samples.is_empty() {
        return samples.to_vec();
    }

    if source_rate == 48_000 {
        // 3:1 decimation with 3-tap triangular FIR filter [0.25, 0.50, 0.25]
        let output_len = samples.len() / 3;
        let mut out = Vec::with_capacity(output_len);
        for i in 0..output_len {
            let idx = i * 3;
            let s0 = samples[idx];
            let s1 = samples.get(idx + 1).copied().unwrap_or(s0);
            let s2 = samples.get(idx + 2).copied().unwrap_or(s1);
            let filtered = 0.25 * s0 + 0.50 * s1 + 0.25 * s2;
            out.push(filtered.clamp(-1.0, 1.0));
        }
        return out;
    }

    // General fractional linear interpolation
    let ratio = source_rate as f64 / 16_000.0;
    let target_len = ((samples.len() as f64) / ratio).round() as usize;
    let mut out = Vec::with_capacity(target_len);

    for i in 0..target_len {
        let src_pos = i as f64 * ratio;
        let idx0 = src_pos.floor() as usize;
        let idx1 = (idx0 + 1).min(samples.len() - 1);
        let frac = (src_pos - idx0 as f64) as f32;

        let s0 = samples.get(idx0).copied().unwrap_or(0.0);
        let s1 = samples.get(idx1).copied().unwrap_or(s0);
        let interpolated = s0 + frac * (s1 - s0);
        out.push(interpolated.clamp(-1.0, 1.0));
    }

    out
}
