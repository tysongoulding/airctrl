//! Challenger Latency Benchmark Empirical Verification
//!
//! Objectives:
//! 1. Verify synthetic WAV processing & transcription completes strictly under 1.0s (1000ms) latency budget.
//! 2. Multi-duration benchmark scaling (0.1s, 0.5s, 1.0s, 2.0s, 5.0s, 10.0s, 30.0s).
//! 3. 100-iteration statistical profiling: min, mean, p50, p95, p99, max latency and jitter.
//! 4. End-to-end pipeline latency: WAV decode -> VAD evaluation -> STT inference -> text injection.

use airctrl_core::platform::mock::create_mock_platform;
use airctrl_core::types::WindowInfo;
use airctrl_stt::audio::wav::{create_synthetic_wav, encode_wav, parse_wav_samples};
use airctrl_stt::benchmark::run_synthetic_wav_benchmark;
use airctrl_stt::engine::{MockWhisperEngine, SpeechToTextEngine};
use airctrl_stt::injection::ActiveWindowTextInjector;
use airctrl_stt::vad::VoiceActivityDetector;
use airctrl_stt::VadConfig;
use std::time::Instant;

#[test]
fn test_latency_multi_duration_synthetic_wav() {
    let engine = MockWhisperEngine::new();

    let durations = [0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0];

    for &dur in &durations {
        let result = run_synthetic_wav_benchmark(&engine, dur)
            .unwrap_or_else(|e| panic!("Failed benchmark on {}s audio: {:?}", dur, e));

        assert!(
            result.satisfies_sub_second_budget,
            "Failed 1.0s latency budget for duration {}s: total {}ms",
            dur,
            result.total_duration_ms
        );
        assert!(
            result.total_duration_ms < 1000,
            "Total duration {}ms >= 1000ms budget",
            result.total_duration_ms
        );
        assert_eq!(result.sample_count, (dur * 16000.0).round() as usize);
        assert!(!result.text.is_empty());
        assert!(result.confidence >= 0.40 && result.confidence <= 0.99);

        // WAV header parsing & normalization should complete in microseconds or few milliseconds
        assert!(
            result.wav_parse_duration_us < 500_000,
            "WAV parse duration took {} us (> 500ms)",
            result.wav_parse_duration_us
        );
    }
}

#[test]
fn test_latency_statistical_profiling_100_runs() {
    let engine = MockWhisperEngine::new();
    let num_runs = 100;
    let mut total_latencies_us = Vec::with_capacity(num_runs);

    for _ in 0..num_runs {
        let start = Instant::now();
        let res = run_synthetic_wav_benchmark(&engine, 1.0).unwrap();
        let elapsed_us = start.elapsed().as_micros();
        assert!(res.satisfies_sub_second_budget);
        total_latencies_us.push(elapsed_us);
    }

    total_latencies_us.sort_unstable();

    let min_us = total_latencies_us[0];
    let max_us = total_latencies_us[num_runs - 1];
    let sum_us: u128 = total_latencies_us.iter().sum();
    let mean_us = sum_us / num_runs as u128;
    let p50_us = total_latencies_us[num_runs * 50 / 100];
    let p95_us = total_latencies_us[num_runs * 95 / 100];
    let p99_us = total_latencies_us[num_runs * 99 / 100];

    let p99_ms = p99_us as f64 / 1000.0;
    let max_ms = max_us as f64 / 1000.0;

    println!(
        "Latency Profile (100 runs, 1.0s WAV): min={:.2}ms, mean={:.2}ms, p50={:.2}ms, p95={:.2}ms, p99={:.2}ms, max={:.2}ms",
        min_us as f64 / 1000.0,
        mean_us as f64 / 1000.0,
        p50_us as f64 / 1000.0,
        p95_us as f64 / 1000.0,
        p99_ms,
        max_ms
    );

    // Assert strictly under 1000ms budget with enormous headroom (p99 < 50ms)
    assert!(p99_ms < 50.0, "p99 latency {:.2}ms exceeded 50ms threshold", p99_ms);
    assert!(max_ms < 100.0, "max latency {:.2}ms exceeded 100ms threshold", max_ms);
}

#[tokio::test]
async fn test_latency_end_to_end_pipeline_budget() {
    // End-to-end pipeline:
    // Raw WAV audio -> parse_wav_samples -> VAD speech check -> MockWhisperEngine -> ActiveWindowTextInjector
    let (provider, controllers) = create_mock_platform();
    controllers.window.add_window(WindowInfo {
        id: 100,
        process_id: 1,
        process_name: "test.exe".into(),
        title: "Test".into(),
        bounds: Default::default(),
        is_focused: true,
        is_minimized: false,
    });

    let engine = MockWhisperEngine::new();
    let vad = VoiceActivityDetector::new(VadConfig::default());
    let injector = ActiveWindowTextInjector::new(provider.window, provider.input, None);

    // 2.0s synthetic speech audio (440Hz tone, 0.4 amplitude)
    let wav_bytes = create_synthetic_wav(2.0, 16000, 1, 440.0, 0.4, false);

    let pipeline_start = Instant::now();

    // 1. WAV decoding
    let samples = parse_wav_samples(&wav_bytes).expect("WAV decode failed");
    assert_eq!(samples.len(), 32000);

    // 2. VAD validation
    let is_speech = vad.is_speech(&samples);
    assert!(is_speech, "Synthetic audio must be identified as speech");

    // 3. STT inference
    let stt_result = engine.transcribe(&samples).expect("STT inference failed");
    assert!(!stt_result.text.is_empty());

    // 4. Text injection
    let injection_report = injector
        .inject(&stt_result.text, Some(100))
        .await
        .expect("Text injection failed");

    let total_elapsed = pipeline_start.elapsed();
    let total_ms = total_elapsed.as_millis() as u64;

    assert!(
        total_ms < 1000,
        "End-to-end pipeline elapsed {}ms exceeded 1000ms budget",
        total_ms
    );
    assert_eq!(
        injection_report.status,
        airctrl_stt::InjectionStatus::Success
    );
    assert_eq!(
        controllers.input.get_typed_strings(),
        vec![stt_result.text]
    );
}

#[test]
fn test_latency_encode_parse_roundtrip_overhead() {
    let original_samples: Vec<f32> = (0..16000)
        .map(|i| ((i as f32 / 16000.0) * 880.0 * std::f32::consts::PI).sin())
        .collect();

    let start = Instant::now();
    let wav_bytes = encode_wav(&original_samples, 16000, 1).expect("Encode failed");
    let parsed_samples = parse_wav_samples(&wav_bytes).expect("Parse failed");
    let roundtrip_duration = start.elapsed();

    assert_eq!(original_samples.len(), parsed_samples.len());
    assert!(
        roundtrip_duration.as_millis() < 50,
        "Encode/parse roundtrip took {}ms (> 50ms)",
        roundtrip_duration.as_millis()
    );
}
