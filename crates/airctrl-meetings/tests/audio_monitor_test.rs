use airctrl_meetings::audio::{
    compute_dbfs, compute_peak, compute_rms, AudioProcessCorrelator, AudioStreamMonitor,
    CorrelatedState, HangoverStateMachine, SpeechState,
};
use airctrl_meetings::config::AudioMonitorConfig;
use std::f32::consts::PI;

#[test]
fn test_rms_calculation() {
    assert_eq!(compute_rms(&[]), 0.0);

    let dc_one = vec![1.0; 100];
    assert!((compute_rms(&dc_one) - 1.0).abs() < 1e-5);

    // Sine wave of amplitude 1.0: theoretical RMS = 1 / sqrt(2) ≈ 0.7071
    let mut sine = Vec::with_capacity(1000);
    for i in 0..1000 {
        let t = i as f32 / 1000.0;
        sine.push((2.0 * PI * 10.0 * t).sin());
    }
    let rms_sine = compute_rms(&sine);
    assert!((rms_sine - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.02);
}

#[test]
fn test_dbfs_calculation() {
    // Full scale RMS 1.0 -> 0.0 dBFS
    let dbfs_full = compute_dbfs(1.0);
    assert!(dbfs_full.abs() < 1e-4);

    // RMS 0.1 -> -20 dBFS
    let dbfs_tenth = compute_dbfs(0.1);
    assert!((dbfs_tenth - (-20.0)).abs() < 0.05);

    // Digital silence (RMS 0.0) clamped to -120 dBFS
    let dbfs_silence = compute_dbfs(0.0);
    assert_eq!(dbfs_silence, -120.0);
}

#[test]
fn test_peak_amplitude() {
    assert_eq!(compute_peak(&[]), 0.0);
    let samples = vec![0.1, -0.85, 0.4, -0.2];
    assert_eq!(compute_peak(&samples), 0.85);
}

#[test]
fn test_hangover_state_machine_transitions() {
    // 3 onset frames required, 2 hangover frames hold
    let mut sm = HangoverStateMachine::new(3, 2);
    assert_eq!(sm.current_state(), SpeechState::Silence);
    assert!(!sm.is_speech_active());

    // 1st active frame -> SpeechPending
    assert!(!sm.update(true));
    assert_eq!(sm.current_state(), SpeechState::SpeechPending);

    // 2nd active frame -> SpeechPending
    assert!(!sm.update(true));
    assert_eq!(sm.current_state(), SpeechState::SpeechPending);

    // 3rd active frame -> SpeechActive!
    assert!(sm.update(true));
    assert_eq!(sm.current_state(), SpeechState::SpeechActive);

    // 1st silent frame -> HangoverHold (speech still active)
    assert!(sm.update(false));
    assert_eq!(sm.current_state(), SpeechState::HangoverHold);

    // 2nd silent frame -> HangoverHold
    assert!(sm.update(false));
    assert_eq!(sm.current_state(), SpeechState::HangoverHold);

    // 3rd silent frame -> hangover expired -> Silence
    assert!(!sm.update(false));
    assert_eq!(sm.current_state(), SpeechState::Silence);
}

#[test]
fn test_audio_stream_monitor_processing() {
    let config = AudioMonitorConfig {
        energy_threshold: 0.05,
        silence_threshold_dbfs: -35.0,
        min_speech_duration_ms: 100, // 1 frame
        silence_timeout_ms: 200,      // 2 frames
        chunk_size_samples: 1600,
        sample_rate: 16_000,
    };

    let mut monitor = AudioStreamMonitor::new(config);

    // Process silent chunk
    let silent_chunk = vec![0.0; 1600];
    let metrics = monitor.process_chunk(&silent_chunk);
    assert_eq!(metrics.rms, 0.0);
    assert_eq!(metrics.dbfs, -120.0);
    assert!(!metrics.is_speech);

    // Process loud speech chunk (amplitude 0.5)
    let loud_chunk = vec![0.5; 1600];
    let metrics2 = monitor.process_chunk(&loud_chunk);
    assert!(metrics2.rms > 0.4);
    assert!(metrics2.dbfs > -10.0);
    assert!(metrics2.is_speech);
}

#[test]
fn test_audio_process_correlator_matrix() {
    let mut correlator = AudioProcessCorrelator::new();
    assert_eq!(correlator.current_state(), CorrelatedState::Inactive);

    // No meeting, no voice
    assert_eq!(correlator.evaluate(false, false), CorrelatedState::Inactive);

    // Meeting present, but silent
    assert_eq!(
        correlator.evaluate(true, false),
        CorrelatedState::MeetingDetectedSilent
    );

    // Meeting present AND active voice -> active discussion
    assert_eq!(
        correlator.evaluate(true, true),
        CorrelatedState::MeetingActiveVoice
    );

    // Voice detected, but no meeting (e.g. YouTube playback)
    assert_eq!(
        correlator.evaluate(false, true),
        CorrelatedState::NonMeetingAudio
    );
}
