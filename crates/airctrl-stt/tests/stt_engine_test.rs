use airctrl_stt::{
    create_whisper_engine, MockWhisperEngine, SpeechToTextEngine, SttConfig, SttError,
    WhisperModelVariant,
};

#[test]
fn test_stt_engine_happy_path() {
    let engine = MockWhisperEngine::new();
    assert!(engine.is_loaded());

    let samples = vec![0.3; 16000]; // 1.0 second of audio
    let result = engine.transcribe(&samples).expect("transcription success");

    assert_eq!(result.sample_count, 16000);
    assert!(result.text.contains("airctrl"));
    assert!(result.confidence >= 0.40 && result.confidence <= 0.99);
    assert_eq!(result.language.as_deref(), Some("en"));
}

#[test]
fn test_stt_engine_empty_buffer_rejected() {
    let engine = MockWhisperEngine::new();
    let err = engine.transcribe(&[]).expect_err("should reject empty buffer");
    assert!(matches!(err, SttError::EmptyAudioBuffer));
}

#[test]
fn test_stt_engine_unloaded_model_rejected() {
    let engine = MockWhisperEngine::new();
    engine.set_loaded(false);
    assert!(!engine.is_loaded());

    let samples = vec![0.5; 1600];
    let err = engine.transcribe(&samples).expect_err("should reject unloaded engine");
    assert!(matches!(err, SttError::ModelNotLoaded(_)));
}

#[test]
fn test_stt_engine_boundary_sample_counts() {
    let engine = MockWhisperEngine::new();

    // 1 sample boundary
    let one_sample = vec![0.2; 1];
    let res_one = engine.transcribe(&one_sample).expect("1 sample accepted");
    assert_eq!(res_one.sample_count, 1);

    // 1600 samples boundary (100ms VAD chunk)
    let vad_chunk = vec![0.5; 1600];
    let res_chunk = engine.transcribe(&vad_chunk).expect("1600 samples accepted");
    assert_eq!(res_chunk.sample_count, 1600);

    // Large audio buffer: 320,000 samples (20 seconds @ 16kHz)
    let large_buffer = vec![0.25; 320_000];
    let res_large = engine.transcribe(&large_buffer).expect("320,000 samples accepted");
    assert_eq!(res_large.sample_count, 320_000);
}

#[test]
fn test_stt_engine_confidence_clamping_boundaries() {
    let engine = MockWhisperEngine::new();

    // Zero energy: all zeroes -> clamped to minimum 0.40
    let silent = vec![0.0; 1600];
    let res_silent = engine.transcribe(&silent).unwrap();
    assert_eq!(res_silent.confidence, 0.40);

    // Maximum energy: all 1.0 -> clamped to maximum 0.99
    let loud = vec![1.0; 1600];
    let res_loud = engine.transcribe(&loud).unwrap();
    assert_eq!(res_loud.confidence, 0.99);
}

#[test]
fn test_stt_engine_factory_and_model_info() {
    let config = SttConfig {
        model_variant: WhisperModelVariant::TinyEn,
        use_mock: true,
        ..Default::default()
    };
    let engine = create_whisper_engine(&config);
    assert!(engine.is_loaded());

    let info = engine.model_info().expect("model info available");
    assert_eq!(info.variant, WhisperModelVariant::TinyEn);
    assert!(info.is_verified);
}
