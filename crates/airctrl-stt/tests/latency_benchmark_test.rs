use airctrl_stt::{
    run_synthetic_wav_benchmark, MockWhisperEngine,
};

#[test]
fn test_latency_benchmark_1s_audio() {
    let engine = MockWhisperEngine::new();
    let res = run_synthetic_wav_benchmark(&engine, 1.0).expect("benchmark run");

    assert!(res.satisfies_sub_second_budget);
    assert!(res.total_duration_ms < 1000);
    assert_eq!(res.sample_count, 16000);
    assert!(res.confidence >= 0.40);
    assert!(!res.text.is_empty());
}

#[test]
fn test_latency_benchmark_5s_audio() {
    let engine = MockWhisperEngine::new();
    let res = run_synthetic_wav_benchmark(&engine, 5.0).expect("benchmark run");

    assert!(res.satisfies_sub_second_budget);
    assert!(res.total_duration_ms < 1000);
    assert_eq!(res.sample_count, 80000); // 5s * 16000
    assert!(res.confidence >= 0.40);
}
