use airctrl_core::error::CoreError;
use airctrl_core::platform::PlatformAudioOps;
use airctrl_meetings::audio::{
    compute_dbfs, compute_peak, compute_rms, AudioProcessCorrelator, AudioStreamMonitor,
    CorrelatedState, HangoverStateMachine, SpeechState,
};
use airctrl_meetings::compliance::{ComplianceAudioGenerator, ComplianceAudioInjector};
use airctrl_meetings::config::{
    AudioMonitorConfig, ComplianceConfig, ComplianceMessageKind,
};
use airctrl_meetings::error::MeetingError;
use std::f32::consts::PI;
use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

/// Mock audio device that intentionally simulates playback failures.
struct FailingAudioOps;

impl PlatformAudioOps for FailingAudioOps {
    fn play_audio_stream(&self, _samples: &[f32], _sample_rate: u32) -> Result<(), CoreError> {
        Err(CoreError::Audio(
            "Hardware audio device busy or permission denied".to_string(),
        ))
    }

    fn capture_audio_chunk(&self) -> Result<Vec<f32>, CoreError> {
        Err(CoreError::Audio("Capture not available".to_string()))
    }

    fn is_audio_output_active(&self) -> Result<bool, CoreError> {
        Ok(false)
    }

    fn list_audio_devices(&self) -> Result<Vec<String>, CoreError> {
        Ok(vec![])
    }
}

/// Mock audio device that records played samples and sample rate for inspection.
struct RecordingAudioOps {
    call_count: AtomicUsize,
    last_sample_count: AtomicUsize,
}

impl RecordingAudioOps {
    fn new() -> Self {
        Self {
            call_count: AtomicUsize::new(0),
            last_sample_count: AtomicUsize::new(0),
        }
    }
}

impl PlatformAudioOps for RecordingAudioOps {
    fn play_audio_stream(&self, samples: &[f32], _sample_rate: u32) -> Result<(), CoreError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        self.last_sample_count.store(samples.len(), Ordering::SeqCst);
        Ok(())
    }

    fn capture_audio_chunk(&self) -> Result<Vec<f32>, CoreError> {
        Ok(vec![0.0; 1600])
    }

    fn is_audio_output_active(&self) -> Result<bool, CoreError> {
        Ok(true)
    }

    fn list_audio_devices(&self) -> Result<Vec<String>, CoreError> {
        Ok(vec!["Virtual Cable".to_string()])
    }
}

// ============================================================================
// PART 1: AUDIO ENERGY EDGE CASES
// ============================================================================

#[test]
fn test_adversarial_audio_energy_zero_samples_buffer() {
    let empty: [f32; 0] = [];

    // compute_rms on 0 samples must return 0.0 without division by zero panic
    let rms = compute_rms(&empty);
    assert_eq!(rms, 0.0, "RMS of 0 samples must be 0.0");

    // compute_dbfs on 0.0 RMS must clamp to -120.0 dBFS (digital silence)
    let dbfs = compute_dbfs(rms);
    assert_eq!(dbfs, -120.0, "dBFS of silence must be -120.0");

    // compute_peak on 0 samples must return 0.0
    let peak = compute_peak(&empty);
    assert_eq!(peak, 0.0, "Peak of 0 samples must be 0.0");

    // AudioStreamMonitor must handle 0 samples chunk gracefully
    let mut monitor = AudioStreamMonitor::new(AudioMonitorConfig::default());
    let metrics = monitor.process_chunk(&empty);
    assert_eq!(metrics.rms, 0.0);
    assert_eq!(metrics.dbfs, -120.0);
    assert_eq!(metrics.peak, 0.0);
    assert!(!metrics.is_speech);
    assert_eq!(metrics.frame_count, 1);
}

#[test]
fn test_adversarial_audio_energy_single_sample_buffer() {
    // 1 sample silence
    assert_eq!(compute_rms(&[0.0]), 0.0);
    assert_eq!(compute_dbfs(compute_rms(&[0.0])), -120.0);
    assert_eq!(compute_peak(&[0.0]), 0.0);

    // 1 sample clipping positive
    assert_eq!(compute_rms(&[1.0]), 1.0);
    assert!((compute_dbfs(compute_rms(&[1.0])) - 0.0).abs() < 1e-4);
    assert_eq!(compute_peak(&[1.0]), 1.0);

    // 1 sample clipping negative
    assert_eq!(compute_rms(&[-1.0]), 1.0);
    assert!((compute_dbfs(compute_rms(&[-1.0])) - 0.0).abs() < 1e-4);
    assert_eq!(compute_peak(&[-1.0]), 1.0);

    // 1 sample half scale
    assert_eq!(compute_rms(&[0.5]), 0.5);
    let dbfs_half = compute_dbfs(0.5);
    // 20 * log10(0.5) ≈ -6.0206 dBFS
    assert!((dbfs_half - (-6.0206)).abs() < 0.05);
    assert_eq!(compute_peak(&[0.5]), 0.5);

    // Monitor processing 1 sample chunks
    let mut monitor = AudioStreamMonitor::new(AudioMonitorConfig::default());
    let m = monitor.process_chunk(&[0.5]);
    assert_eq!(m.rms, 0.5);
    assert_eq!(m.peak, 0.5);
}

#[test]
fn test_adversarial_audio_energy_massive_buffer_one_million_samples() {
    const N: usize = 1_000_000;

    // 1M samples of digital silence
    let silence = vec![0.0_f32; N];
    let start = Instant::now();
    let rms_silence = compute_rms(&silence);
    let peak_silence = compute_peak(&silence);
    let elapsed = start.elapsed();

    assert_eq!(rms_silence, 0.0);
    assert_eq!(compute_dbfs(rms_silence), -120.0);
    assert_eq!(peak_silence, 0.0);
    assert!(
        elapsed.as_millis() < 100,
        "1M samples RMS calculation must complete in < 100ms in debug mode, took {:?}",
        elapsed
    );

    // 1M samples of full scale DC (+1.0)
    let full_scale = vec![1.0_f32; N];
    let rms_full = compute_rms(&full_scale);
    assert!((rms_full - 1.0).abs() < 1e-6);
    assert!((compute_dbfs(rms_full) - 0.0).abs() < 1e-4);
    assert_eq!(compute_peak(&full_scale), 1.0);

    // 1M samples of alternating square wave (+1.0, -1.0)
    let mut square = Vec::with_capacity(N);
    for i in 0..N {
        square.push(if i % 2 == 0 { 1.0 } else { -1.0 });
    }
    let rms_sq = compute_rms(&square);
    assert!((rms_sq - 1.0).abs() < 1e-6);
    assert!((compute_dbfs(rms_sq) - 0.0).abs() < 1e-4);
    assert_eq!(compute_peak(&square), 1.0);

    // 1M samples sine wave: theoretical RMS = 1 / sqrt(2) ≈ 0.70710678
    let mut sine = Vec::with_capacity(N);
    for i in 0..N {
        let t = i as f32 / 16000.0;
        sine.push((2.0 * PI * 440.0 * t).sin());
    }
    let rms_sine = compute_rms(&sine);
    let expected_rms = std::f32::consts::FRAC_1_SQRT_2;
    assert!(
        (rms_sine - expected_rms).abs() < 1e-3,
        "Sine wave RMS {} should be near {}",
        rms_sine,
        expected_rms
    );
    let dbfs_sine = compute_dbfs(rms_sine);
    // 20 * log10(1/sqrt(2)) ≈ -3.0103 dBFS
    assert!((dbfs_sine - (-3.0103)).abs() < 0.05);
}

#[test]
fn test_adversarial_audio_energy_extreme_amplitudes_and_overdrive() {
    // Values outside [-1.0, 1.0]: overdriven amplifier simulation
    let overdriven_2x = vec![2.0_f32; 1000];
    let rms_2x = compute_rms(&overdriven_2x);
    assert!((rms_2x - 2.0).abs() < 1e-5);
    assert_eq!(compute_peak(&overdriven_2x), 2.0);
    // dBFS must be clamped to maximum 0.0 dBFS (cannot be positive full-scale violation)
    assert_eq!(compute_dbfs(rms_2x), 0.0);

    let overdriven_negative = vec![-5.0_f32; 1000];
    let rms_neg = compute_rms(&overdriven_negative);
    assert!((rms_neg - 5.0).abs() < 1e-5);
    assert_eq!(compute_peak(&overdriven_negative), 5.0);
    assert_eq!(compute_dbfs(rms_neg), 0.0);

    let massive_overdrive = vec![100.0_f32; 100];
    let rms_100 = compute_rms(&massive_overdrive);
    assert!((rms_100 - 100.0).abs() < 1e-4);
    assert_eq!(compute_peak(&massive_overdrive), 100.0);
    assert_eq!(compute_dbfs(rms_100), 0.0);
}

#[test]
fn test_adversarial_dbfs_clamping_and_non_finite_inputs() {
    // Exact zero
    assert_eq!(compute_dbfs(0.0), -120.0);
    assert_eq!(compute_dbfs(-0.0), -120.0);

    // Negative RMS inputs (corrupted metric or inverse)
    assert_eq!(compute_dbfs(-0.001), -120.0);
    assert_eq!(compute_dbfs(-1.0), -120.0);
    assert_eq!(compute_dbfs(-1000.0), -120.0);

    // Subnormal / micro float
    assert_eq!(compute_dbfs(f32::MIN_POSITIVE), -120.0);
    assert_eq!(compute_dbfs(1e-15), -120.0);

    // Threshold values
    assert!((compute_dbfs(1e-6) - (-120.0)).abs() < 1e-4); // 20 * log10(1e-6) ≈ -120.0
    let dbfs_1e3 = compute_dbfs(1e-3); // 20 * log10(1e-3) = -60.0
    assert!((dbfs_1e3 - (-60.0)).abs() < 1e-3);

    // Full scale
    assert_eq!(compute_dbfs(1.0), 0.0);

    // Non-finite values: NaN and Infinities
    let nan_result = compute_dbfs(f32::NAN);
    assert!(!nan_result.is_nan(), "compute_dbfs must never return NaN");
    assert_eq!(nan_result, -120.0, "NaN input should clamp to silence");

    let inf_result = compute_dbfs(f32::INFINITY);
    assert!(
        !inf_result.is_infinite(),
        "compute_dbfs must never return Infinity"
    );
    assert_eq!(
        inf_result, 0.0,
        "Infinity input should clamp to 0.0 full-scale"
    );

    let neg_inf_result = compute_dbfs(f32::NEG_INFINITY);
    assert!(
        !neg_inf_result.is_infinite(),
        "compute_dbfs must never return -Infinity"
    );
    assert_eq!(
        neg_inf_result, -120.0,
        "-Infinity input should clamp to silence"
    );

    // NaN inside sample buffer for compute_rms
    let nan_buffer = vec![f32::NAN, 1.0, 0.5];
    let rms_from_nan = compute_rms(&nan_buffer);
    assert!(
        !rms_from_nan.is_nan(),
        "compute_rms must sanitize NaN samples"
    );
    assert_eq!(rms_from_nan, 0.0);
    assert_eq!(compute_dbfs(rms_from_nan), -120.0);

    // Sweep 1,000 pseudo-random and extreme values: verify strictly in [-120.0, 0.0]
    let test_vals = [
        -1e10, -500.0, -1.0, -0.1, -0.0, 0.0, 1e-12, 1e-8, 1e-4, 0.05, 0.1, 0.5, 0.707, 1.0,
        1.5, 10.0, 1000.0, 1e9,
    ];
    for &val in &test_vals {
        let res = compute_dbfs(val);
        assert!(!res.is_nan(), "Failed on val {}", val);
        assert!(!res.is_infinite(), "Failed on val {}", val);
        assert!(
            (-120.0..=0.0).contains(&res),
            "Result {} not in [-120.0, 0.0] for val {}",
            res,
            val
        );
    }
}

// ============================================================================
// PART 2: HANGOVER STATE MACHINE FUZZING
// ============================================================================

#[test]
fn test_adversarial_hangover_state_machine_bursty_speech_rejection() {
    // Configure state machine with 3 onset confirmation frames and 2 hangover frames
    let mut sm = HangoverStateMachine::new(3, 2);

    // Bursty speech pattern: 1 active frame followed by 1 silent frame, repeated 1,000 times
    // This simulates coughs, keyboard clatter, mic pops, or background transients.
    for i in 0..1000 {
        // Frame active: onset count becomes 1 (< 3) -> SpeechPending
        let active_res = sm.update(true);
        assert!(
            !active_res,
            "Iteration {}: 1 active frame must not activate speech (SpeechPending)",
            i
        );
        assert_eq!(sm.current_state(), SpeechState::SpeechPending);
        assert!(!sm.is_speech_active());

        // Frame silent: resets onset count to 0 -> Silence
        let silent_res = sm.update(false);
        assert!(
            !silent_res,
            "Iteration {}: Silence frame must drop back to Silence",
            i
        );
        assert_eq!(sm.current_state(), SpeechState::Silence);
        assert!(!sm.is_speech_active());
    }

    // Two-frame burst pattern: [true, true, false] repeated 500 times with min_speech_frames = 3
    for _i in 0..500 {
        assert!(!sm.update(true)); // frame 1 -> SpeechPending
        assert_eq!(sm.current_state(), SpeechState::SpeechPending);

        assert!(!sm.update(true)); // frame 2 -> SpeechPending
        assert_eq!(sm.current_state(), SpeechState::SpeechPending);

        assert!(!sm.update(false)); // frame 3 -> dropped back to Silence
        assert_eq!(sm.current_state(), SpeechState::Silence);
    }
}

#[test]
fn test_adversarial_hangover_persistence_exact_decay() {
    // min_speech_frames = 2, hangover_frames = 5
    let mut sm = HangoverStateMachine::new(2, 5);

    // Frame 1 active
    assert!(!sm.update(true));
    assert_eq!(sm.current_state(), SpeechState::SpeechPending);

    // Frame 2 active -> Reaches threshold 2 -> SpeechActive!
    assert!(sm.update(true));
    assert_eq!(sm.current_state(), SpeechState::SpeechActive);
    assert!(sm.is_speech_active());

    // Now feed silent frames: must hold exactly 5 hangover decay frames
    for decay_tick in 1..=5 {
        let is_active = sm.update(false);
        assert!(
            is_active,
            "Hangover tick {}/5 must hold speech active",
            decay_tick
        );
        assert_eq!(sm.current_state(), SpeechState::HangoverHold);
    }

    // 6th silent frame: hangover expired -> drops to Silence!
    let is_active_after_expiry = sm.update(false);
    assert!(
        !is_active_after_expiry,
        "6th silent frame must return to Silence"
    );
    assert_eq!(sm.current_state(), SpeechState::Silence);

    // Additional silent frames remain in Silence
    for _ in 0..10 {
        assert!(!sm.update(false));
        assert_eq!(sm.current_state(), SpeechState::Silence);
    }
}

#[test]
fn test_adversarial_hangover_state_machine_reset_and_boundaries() {
    // Boundary: min_speech_frames = 0 clamped to 1
    let mut sm_zero = HangoverStateMachine::new(0, 3);
    // With min_speech_frames = 1, single active frame activates speech immediately
    assert!(sm_zero.update(true));
    assert_eq!(sm_zero.current_state(), SpeechState::SpeechActive);

    // Boundary: hangover_frames = 0
    let mut sm_no_hangover = HangoverStateMachine::new(2, 0);
    assert!(!sm_no_hangover.update(true));
    assert!(sm_no_hangover.update(true)); // SpeechActive
    // Next silent frame: 0 hangover frames -> immediate drop to Silence!
    assert!(!sm_no_hangover.update(false));
    assert_eq!(sm_no_hangover.current_state(), SpeechState::Silence);

    // Test explicit reset during SpeechActive
    let mut sm_reset = HangoverStateMachine::new(2, 4);
    sm_reset.update(true);
    sm_reset.update(true);
    assert_eq!(sm_reset.current_state(), SpeechState::SpeechActive);
    sm_reset.reset();
    assert_eq!(sm_reset.current_state(), SpeechState::Silence);
    assert!(!sm_reset.is_speech_active());

    // Test explicit reset during HangoverHold
    sm_reset.update(true);
    sm_reset.update(true);
    sm_reset.update(false);
    assert_eq!(sm_reset.current_state(), SpeechState::HangoverHold);
    sm_reset.reset();
    assert_eq!(sm_reset.current_state(), SpeechState::Silence);
    assert!(!sm_reset.is_speech_active());
}

// ============================================================================
// PART 3: AUDIO STREAM MONITOR & CORRELATOR INTEGRATION
// ============================================================================

#[test]
fn test_adversarial_audio_stream_monitor_full_lifecycle() {
    let config = AudioMonitorConfig {
        energy_threshold: 0.05,
        silence_threshold_dbfs: -35.0,
        min_speech_duration_ms: 300, // 3 chunks at 100ms
        silence_timeout_ms: 400,      // 4 chunks at 100ms
        chunk_size_samples: 1600,     // 100ms at 16kHz
        sample_rate: 16_000,
    };

    let mut monitor = AudioStreamMonitor::new(config);
    assert_eq!(monitor.latest_metrics().frame_count, 0);
    assert!(!monitor.is_speech_active());

    let silence_chunk = vec![0.0_f32; 1600];
    let loud_chunk = vec![0.5_f32; 1600];

    // 1. Initial 5 silent chunks
    for i in 1..=5 {
        let m = monitor.process_chunk(&silence_chunk);
        assert_eq!(m.frame_count, i);
        assert_eq!(m.rms, 0.0);
        assert_eq!(m.dbfs, -120.0);
        assert_eq!(m.peak, 0.0);
        assert!(!m.is_speech);
    }

    // 2. Onset smoothing: 2 loud chunks (not yet 3)
    let m1 = monitor.process_chunk(&loud_chunk);
    assert!(!m1.is_speech, "Chunk 1 of speech must not trigger speech yet");

    let m2 = monitor.process_chunk(&loud_chunk);
    assert!(!m2.is_speech, "Chunk 2 of speech must not trigger speech yet");

    // 3. 3rd loud chunk -> Speech active!
    let m3 = monitor.process_chunk(&loud_chunk);
    assert!(m3.is_speech, "Chunk 3 of speech must activate speech!");
    assert!(monitor.is_speech_active());

    // 4. Inactive chunks: 4 hangover chunks must hold speech
    for tick in 1..=4 {
        let m_silent = monitor.process_chunk(&silence_chunk);
        assert!(
            m_silent.is_speech,
            "Hangover chunk {}/4 must retain is_speech = true",
            tick
        );
    }

    // 5. 5th silent chunk: hangover expired -> speech is false
    let m_expired = monitor.process_chunk(&silence_chunk);
    assert!(
        !m_expired.is_speech,
        "5th silent chunk must return is_speech = false"
    );
    assert!(!monitor.is_speech_active());

    // 6. Reset monitor
    monitor.reset();
    assert_eq!(monitor.latest_metrics().frame_count, 0);
    assert!(!monitor.is_speech_active());
}

#[test]
fn test_adversarial_audio_process_correlator_matrix() {
    let mut correlator = AudioProcessCorrelator::new();
    assert_eq!(correlator.current_state(), CorrelatedState::Inactive);

    // Matrix truth table tests:
    // (meeting, speech) -> Expected State
    assert_eq!(
        correlator.evaluate(false, false),
        CorrelatedState::Inactive
    );
    assert_eq!(
        correlator.evaluate(true, false),
        CorrelatedState::MeetingDetectedSilent
    );
    assert_eq!(
        correlator.evaluate(true, true),
        CorrelatedState::MeetingActiveVoice
    );
    assert_eq!(
        correlator.evaluate(false, true),
        CorrelatedState::NonMeetingAudio
    );

    // Reset back to Inactive
    correlator.reset();
    assert_eq!(correlator.current_state(), CorrelatedState::Inactive);
}

// ============================================================================
// PART 4: COMPLIANCE AUDIO INJECTION BOUNDARY ATTACKS
// ============================================================================

#[test]
fn test_adversarial_compliance_volume_floor_enforcement() {
    // Requirement: Volume floor enforcement: attempt to set volume to 0.0, -0.5, 0.1, 0.49
    // -> verify effective volume is clamped to >= 0.50.
    let test_cases = [
        (0.0_f32, 0.50_f32),
        (-0.5_f32, 0.50_f32),
        (-100.0_f32, 0.50_f32),
        (0.1_f32, 0.50_f32),
        (0.49_f32, 0.50_f32),
        (0.49999_f32, 0.50_f32),
        (0.50_f32, 0.50_f32),
        (0.50001_f32, 0.50001_f32),
        (0.75_f32, 0.75_f32),
        (1.0_f32, 1.0_f32),
        (1.5_f32, 1.0_f32), // capped at 1.0
    ];

    for (input_vol, expected_effective) in test_cases {
        let config = ComplianceConfig {
            volume: input_vol,
            ..Default::default()
        };
        let effective = config.effective_volume();
        assert!(
            (effective - expected_effective).abs() < 1e-4,
            "Input volume {} produced effective volume {}, expected {}",
            input_vol,
            effective,
            expected_effective
        );
        assert!(
            effective >= 0.50,
            "Effective volume {} must be >= 0.50",
            effective
        );
        assert!(
            effective <= 1.0,
            "Effective volume {} must be <= 1.0",
            effective
        );
    }

    // Direct injector verification: play announcement with 0.0 volume request
    let recording_ops = Arc::new(RecordingAudioOps::new());
    let injector = ComplianceAudioInjector::with_audio_ops(recording_ops.clone());

    let zero_vol_config = ComplianceConfig {
        volume: 0.0,
        ..Default::default()
    };
    let res = injector.play_announcement(&zero_vol_config).unwrap();
    assert_eq!(res.status, "Success");
    assert_eq!(recording_ops.call_count.load(Ordering::SeqCst), 1);
    assert_eq!(
        recording_ops.last_sample_count.load(Ordering::SeqCst),
        23_200
    );

    // Verify PCM buffer generated at floor volume has non-zero acoustic energy
    let floor_samples = ComplianceAudioGenerator::generate_pcm_buffer(
        zero_vol_config.sample_rate,
        zero_vol_config.duration_ms,
        zero_vol_config.effective_volume(),
    );
    let floor_rms = compute_rms(&floor_samples);
    assert!(
        floor_rms > 0.10,
        "Generated announcement at volume floor 0.50 must have audible RMS energy (> 0.10), got {}",
        floor_rms
    );
    assert!(
        compute_peak(&floor_samples) > 0.30,
        "Generated announcement must have audible peak energy (> 0.30)"
    );
}

#[test]
fn test_adversarial_compliance_recording_lockout_gating() {
    // 1. Fresh injector must disallow recording prior to compliance announcement
    let injector = ComplianceAudioInjector::new();
    assert!(
        !injector.is_recording_allowed(),
        "Recording MUST BE BLOCKED prior to compliance announcement playback"
    );

    // 2. Play announcement successfully -> recording allowed
    let config = ComplianceConfig::default();
    let res = injector.play_announcement(&config).unwrap();
    assert_eq!(res.status, "Success");
    assert!(
        injector.is_recording_allowed(),
        "Recording MUST BE PERMITTED after successful compliance announcement"
    );

    // 3. Reset recording gate -> locked again
    injector.reset_recording_gate();
    assert!(
        !injector.is_recording_allowed(),
        "Recording MUST BE BLOCKED again after reset_recording_gate()"
    );

    // 4. Boundary Attack: Failing Audio Device
    // If playback fails, recording MUST REMAIN BLOCKED!
    let failing_ops = Arc::new(FailingAudioOps);
    let failing_injector = ComplianceAudioInjector::with_audio_ops(failing_ops);

    assert!(
        !failing_injector.is_recording_allowed(),
        "Initial state must disallow recording"
    );

    let fail_res = failing_injector.play_announcement(&config);
    assert!(
        fail_res.is_err(),
        "Playback through failing device must return Err"
    );
    match fail_res {
        Err(MeetingError::AudioPlaybackFailed(ref msg)) => {
            assert!(
                msg.contains("Hardware audio device busy"),
                "Expected audio failure message, got {}",
                msg
            );
        }
        other => panic!("Expected MeetingError::AudioPlaybackFailed, got {:?}", other),
    }

    // Critical assertion: Recording lockout is NOT breached when audio playback fails!
    assert!(
        !failing_injector.is_recording_allowed(),
        "Recording lockout MUST REMAIN ACTIVE when audio playback fails"
    );
}

#[test]
fn test_adversarial_compliance_audio_buffer_integrity_and_sample_count() {
    // Requirement: Audio buffer integrity: verify exactly 1450ms at 16,000 Hz mono (23,200 samples).
    let sample_rate = 16_000_u32;
    let duration_ms = 1450_u32;
    let volume = 1.0_f32;

    let samples = ComplianceAudioGenerator::generate_pcm_buffer(sample_rate, duration_ms, volume);

    // Assert exact sample count: (16,000 * 1450) / 1000 = 23,200 samples
    let expected_sample_count = ((sample_rate as u64 * duration_ms as u64) / 1000) as usize;
    assert_eq!(expected_sample_count, 23_200);
    assert_eq!(
        samples.len(),
        23_200,
        "PCM buffer must contain exactly 23,200 samples"
    );

    // Assert all samples strictly normalized in [-1.0, 1.0] without clipping overflow
    for (i, &s) in samples.iter().enumerate() {
        assert!(
            (-1.0..=1.0).contains(&s),
            "Sample at index {} exceeded [-1.0, 1.0]: {}",
            i,
            s
        );
    }

    // Verify acoustic envelope: smooth attack ramp and release ramp
    // First sample should start close to 0.0
    assert!(
        samples[0].abs() < 0.05,
        "First sample should start near zero (smooth attack), got {}",
        samples[0]
    );
    // Last sample should finish close to 0.0
    let last_sample = samples[samples.len() - 1];
    assert!(
        last_sample.abs() < 0.05,
        "Last sample should finish near zero (smooth release), got {}",
        last_sample
    );

    // Verify WAV byte encoding through hound
    let wav_bytes = ComplianceAudioGenerator::encode_wav_bytes(&samples, sample_rate).unwrap();

    // Verify WAV RIFF header and specification
    let reader = hound::WavReader::new(Cursor::new(wav_bytes.clone())).unwrap();
    let spec = reader.spec();
    assert_eq!(spec.channels, 1, "WAV audio stream must be mono (1 channel)");
    assert_eq!(
        spec.sample_rate, 16_000,
        "WAV audio stream must be 16,000 Hz"
    );
    assert_eq!(
        spec.bits_per_sample, 16,
        "WAV audio stream must be 16-bit PCM"
    );
    assert_eq!(
        spec.sample_format,
        hound::SampleFormat::Int,
        "WAV audio stream must use integer PCM samples"
    );

    // Total sample count in WAV file
    assert_eq!(
        reader.len() as usize,
        23_200,
        "WAV header sample count must match exactly 23,200"
    );

    // Exactly 1450 milliseconds duration
    let duration_calc_ms = (reader.len() as f64 / spec.sample_rate as f64) * 1000.0;
    assert!(
        (duration_calc_ms - 1450.0).abs() < 1e-4,
        "WAV duration must be exactly 1450.0 ms, got {}",
        duration_calc_ms
    );

    // Exact byte length check: 44-byte standard RIFF header + (23,200 samples * 2 bytes/sample) = 46,444 bytes
    assert_eq!(
        wav_bytes.len(),
        46_444,
        "16-bit mono 23,200-sample WAV stream must be exactly 46,444 bytes"
    );
}

#[test]
fn test_adversarial_compliance_verbatim_announcement_text() {
    // Requirement: Verbatim announcement text: verify announcement text matches
    // "This meeting is being recorded" (exactly 30 characters).
    let legal_variant = ComplianceMessageKind::LegalRecordingDisclosure;
    let announcement_str = legal_variant.as_str();

    assert_eq!(
        announcement_str, "This meeting is being recorded",
        "Announcement text must match 'This meeting is being recorded' verbatim"
    );
    assert_eq!(
        announcement_str.len(),
        30,
        "Announcement text length must be exactly 30 characters"
    );
    assert_eq!(
        announcement_str.chars().count(),
        30,
        "Announcement text Unicode char count must be exactly 30"
    );

    // Also verify playback result returned from injector
    let injector = ComplianceAudioInjector::new();
    let config = ComplianceConfig::default();
    let res = injector.play_announcement(&config).unwrap();

    assert_eq!(res.announcement, "This meeting is being recorded");
    assert_eq!(res.announcement.len(), 30);
    assert_eq!(res.status, "Success");
    assert!(res.injected_into_stream);
    assert_eq!(res.duration_ms, 1450);

    // Verify secondary variant: AssistantTranscriptionNotice
    let notice_variant = ComplianceMessageKind::AssistantTranscriptionNotice;
    assert_eq!(
        notice_variant.as_str(),
        "This meeting is being transcribed and automated by airctrl assistant."
    );
    assert_eq!(notice_variant.as_str().len(), 69);
}

#[test]
fn test_adversarial_compliance_injector_multithreaded_concurrency() {
    let injector = Arc::new(ComplianceAudioInjector::new());
    let mut handles = Vec::new();

    // 16 concurrent threads playing compliance disclosure
    for _ in 0..16 {
        let inj = injector.clone();
        handles.push(thread::spawn(move || {
            let config = ComplianceConfig::default();
            inj.play_announcement(&config)
        }));
    }

    for h in handles {
        let res = h.join().unwrap().unwrap();
        assert_eq!(res.status, "Success");
        assert_eq!(res.announcement.len(), 30);
    }

    assert_eq!(injector.fallback_sink().played_count(), 16);
    assert!(injector.is_recording_allowed());
}
