use crate::audio::wav::{create_synthetic_wav, parse_wav_samples};
use crate::engine::SpeechToTextEngine;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Benchmark metrics detailing end-to-end audio ingestion, decoding, and STT inference latency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LatencyBenchmarkResult {
    pub duration_secs: f32,
    pub sample_count: usize,
    pub wav_parse_duration_us: u128,
    pub inference_duration_ms: u64,
    pub total_duration_ms: u64,
    pub satisfies_sub_second_budget: bool,
    pub text: String,
    pub confidence: f32,
}

/// Executes a programmatic latency benchmark feeding synthetic WAV audio through decoding and STT inference,
/// verifying the <1.0s sub-second latency budget requirement.
pub fn run_synthetic_wav_benchmark(
    engine: &dyn SpeechToTextEngine,
    duration_secs: f32,
) -> Result<LatencyBenchmarkResult> {
    let start_total = Instant::now();

    // 1. Generate synthetic PCM WAV audio buffer (440Hz tone, 16kHz mono)
    let wav_bytes = create_synthetic_wav(duration_secs, 16000, 1, 440.0, 0.5, false);

    // 2. Measure RIFF/WAVE header parsing & PCM normalization duration
    let start_parse = Instant::now();
    let samples = parse_wav_samples(&wav_bytes)?;
    let parse_us = start_parse.elapsed().as_micros();

    // 3. Measure Whisper STT inference latency
    let result = engine.transcribe(&samples)?;
    let total_ms = start_total.elapsed().as_millis() as u64;

    Ok(LatencyBenchmarkResult {
        duration_secs,
        sample_count: samples.len(),
        wav_parse_duration_us: parse_us,
        inference_duration_ms: result.duration_ms,
        total_duration_ms: total_ms,
        satisfies_sub_second_budget: total_ms < 1000,
        text: result.text,
        confidence: result.confidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::MockWhisperEngine;

    #[test]
    fn test_synthetic_wav_benchmark_sub_second_budget() {
        let engine = MockWhisperEngine::new();
        let result = run_synthetic_wav_benchmark(&engine, 2.0).unwrap();

        assert!(result.satisfies_sub_second_budget);
        assert!(result.total_duration_ms < 1000);
        assert_eq!(result.sample_count, 32000); // 2.0s * 16000
        assert!(result.confidence >= 0.40);
        assert!(!result.text.is_empty());
    }
}
