use airctrl_stt::{
    create_vad_detector, DiscardReason, StreamVadState, VadConfig, VadDecision,
    VoiceActivityDetector,
};

#[test]
fn test_vad_rms_and_dbfs_calculation() {
    let vad = VoiceActivityDetector::default();

    // Empty samples
    assert_eq!(vad.calculate_rms_energy(&[]), 0.0);
    assert_eq!(vad.calculate_dbfs(&[]), -120.0);

    // Constant 0.5 amplitude
    let samples = vec![0.5; 1000];
    let rms = vad.calculate_rms_energy(&samples);
    assert!((rms - 0.5).abs() < 1e-5);

    let dbfs = vad.calculate_dbfs(&samples);
    // 20 * log10(0.5) approx -6.02 dBFS
    assert!((dbfs - (-6.0206)).abs() < 0.01);
}

#[test]
fn test_vad_boundary_sample_counts() {
    let vad = VoiceActivityDetector::default(); // min_samples: 1600

    // Exactly 1599 samples rejected
    let under_min = vec![1.0; 1599];
    assert!(!vad.is_speech(&under_min));

    // Exactly 1600 samples with high energy accepted
    let exact_min = vec![1.0; 1600];
    assert!(vad.is_speech(&exact_min));

    // Exactly 1601 samples accepted
    let over_min = vec![1.0; 1601];
    assert!(vad.is_speech(&over_min));
}

#[test]
fn test_vad_energy_threshold_boundaries() {
    let vad = VoiceActivityDetector::default(); // energy_threshold: 0.05

    // 0.049 RMS -> silence
    let below = vec![0.049; 1600];
    assert!(!vad.is_speech(&below));

    // 0.050 RMS -> speech
    let exact = vec![0.050; 1600];
    assert!(vad.is_speech(&exact));

    // 0.051 RMS -> speech
    let above = vec![0.051; 1600];
    assert!(vad.is_speech(&above));
}

#[test]
fn test_vad_silence_trimming_retains_margins() {
    let config = VadConfig {
        energy_threshold: 0.05,
        min_samples: 1600,
        frame_size: 320,
        hop_size: 160,
        pre_roll_samples: 800,  // 50ms @ 16kHz
        post_roll_samples: 1600, // 100ms @ 16kHz
        ..Default::default()
    };
    let vad = VoiceActivityDetector::new(config);

    // Buffer structure:
    // 1600 samples silence (100ms)
    // 3200 samples speech (200ms @ amp 0.5)
    // 3200 samples silence (200ms)
    let mut buffer = Vec::new();
    buffer.extend(vec![0.0; 1600]);
    buffer.extend(vec![0.5; 3200]);
    buffer.extend(vec![0.0; 3200]);

    let (trimmed, meta) = vad.trim_silence(&buffer);
    assert!(!trimmed.is_empty());
    assert_eq!(meta.original_samples, 8000);
    // Leading trim should preserve ~800 samples of pre-roll before the speech at idx 1600
    // so trim_start is around 1600 - 800 = 800
    assert!(meta.leading_trimmed_samples <= 1600);
    assert!(meta.trailing_trimmed_samples > 0);
}

#[test]
fn test_vad_evaluate_decision_categories() {
    let vad = VoiceActivityDetector::default();

    // Empty buffer
    let dec_empty = vad.evaluate(&[]);
    assert_eq!(
        dec_empty,
        VadDecision::Discarded {
            reason: DiscardReason::EmptyBuffer,
            sample_count: 0
        }
    );

    // Too short (<1600)
    let dec_short = vad.evaluate(&vec![0.5; 500]);
    assert_eq!(
        dec_short,
        VadDecision::Discarded {
            reason: DiscardReason::TooShort,
            sample_count: 500
        }
    );

    // Silence (below 0.05 threshold)
    let dec_silence = vad.evaluate(&vec![0.01; 2000]);
    assert!(matches!(dec_silence, VadDecision::Silence { .. }));

    // Valid speech
    let mut speech_buf = vec![0.0; 800];
    speech_buf.extend(vec![0.4; 2000]);
    speech_buf.extend(vec![0.0; 800]);
    let dec_speech = vad.evaluate(&speech_buf);
    assert!(matches!(dec_speech, VadDecision::Speech { .. }));
}

#[test]
fn test_vad_streaming_hangover_state_machine() {
    let mut vad = create_vad_detector(Some(VadConfig {
        energy_threshold: 0.05,
        hangover_duration_ms: 100, // short hangover
        ..Default::default()
    }));

    let speech_frame = vec![0.5; 320];
    let silence_frame = vec![0.0; 320];

    assert_eq!(vad.process_stream_frame(&speech_frame), StreamVadState::Speech);
    assert_eq!(vad.process_stream_frame(&silence_frame), StreamVadState::Hangover);

    // Exhaust hangover frames
    for _ in 0..10 {
        vad.process_stream_frame(&silence_frame);
    }
    assert_eq!(vad.process_stream_frame(&silence_frame), StreamVadState::Silence);
}
