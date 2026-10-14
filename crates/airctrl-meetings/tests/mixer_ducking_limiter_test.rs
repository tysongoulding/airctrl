use airctrl_meetings::{DualAudioMixer, LiveMeetingTranscriber, MixerConfig};
use airctrl_stt::MockWhisperEngine;

#[test]
fn test_dual_audio_mixer_ducking_gain_floor_invariant() {
    let config = MixerConfig {
        ducking_gain: 0.25,
        ducking_threshold: 0.05,
        attack_ms: 10.0,
        release_ms: 50.0,
        ducking_enabled: true,
        ..Default::default()
    };

    let mut mixer = DualAudioMixer::new(config);

    // Initial state: ducking gain is 1.0
    assert_eq!(mixer.current_duck_gain(), 1.0);
    assert!(!mixer.is_ducking_active());

    // Feed loud mic samples (amplitude 0.8) along with loopback (amplitude 0.5)
    let mic_samples = vec![0.8f32; 1600]; // 100ms at 16kHz
    let loopback_samples = vec![0.5f32; 1600];

    let mixed = mixer.mix(&mic_samples, &loopback_samples);
    assert_eq!(mixed.len(), 1600);

    // After 100ms of speech (>> attack time of 10ms), ducking should be active
    assert!(mixer.is_ducking_active(), "Ducking should be active during mic speech");
    // Gain floor must strictly respect the configured floor (>= 0.25)
    assert!(
        mixer.current_duck_gain() >= 0.2499,
        "Ducking gain must not drop below configured floor"
    );
    assert!(
        mixer.current_duck_gain() <= 0.35,
        "Ducking gain should have attenuated towards 0.25"
    );

    // Now feed silence to mic to observe release
    let silent_mic = vec![0.0f32; 16000]; // 1s at 16kHz
    let continuous_loopback = vec![0.5f32; 16000];
    let _ = mixer.mix(&silent_mic, &continuous_loopback);

    // After 1s of silence (>> release time of 50ms), ducking should recover towards 1.0
    assert!(
        mixer.current_duck_gain() > 0.95,
        "Ducking gain should recover towards 1.0 during silence: {}",
        mixer.current_duck_gain()
    );
}

#[test]
fn test_hyperbolic_tangent_limiter_anti_clipping() {
    let config = MixerConfig {
        anti_clipping_enabled: true,
        ducking_enabled: false,
        ..Default::default()
    };

    let mut mixer = DualAudioMixer::new(config);

    // Extreme high-amplitude input that would severely clip without limiter (sum = 2.0 + 3.0 = 5.0)
    let hot_mic = vec![2.0f32; 1000];
    let hot_loopback = vec![3.0f32; 1000];

    let output = mixer.mix(&hot_mic, &hot_loopback);

    for (idx, &sample) in output.iter().enumerate() {
        assert!(
            (-1.0..=1.0).contains(&sample),
            "Sample at {} breached limiter [-1.0, 1.0]: {}",
            idx,
            sample
        );
        // Tanh(5.0) is approx 0.9999
        assert!(sample > 0.99 && sample < 1.0, "Expected tanh compression for hot signal: {}", sample);
    }
}

#[test]
fn test_dual_audio_mixer_sanitizes_nan_inf_and_mismatched_lengths() {
    let mut mixer = DualAudioMixer::default();

    // Loopback is longer than mic, and mic has NaN/Inf
    let mic_samples = vec![f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.4];
    let loopback_samples = vec![0.2f32; 10]; // 10 samples

    let output = mixer.mix(&mic_samples, &loopback_samples);
    assert_eq!(output.len(), 10, "Mismatched buffer should pad to longest slice");

    for (idx, &sample) in output.iter().enumerate() {
        assert!(
            sample.is_finite(),
            "Output at {} must be sanitized finite float: {}",
            idx,
            sample
        );
    }
}

#[test]
fn test_live_meeting_transcriber_speaker_attribution_and_pcm_pipeline() {
    let mut transcriber = LiveMeetingTranscriber::new();

    // Case 1: Host dominant (mic energy high, loopback low -> ratio > 2.0)
    let host_chunk = transcriber.feed_audio(0.8, 0.1, 1000, "Good morning everyone.");
    assert_eq!(host_chunk.speaker, "Host");
    assert_eq!(host_chunk.start_time_ms, 0);
    assert_eq!(host_chunk.end_time_ms, 1000);

    // Case 2: Participant dominant (loopback energy high, mic low -> loop/mic > 2.0)
    let part_chunk = transcriber.feed_audio(0.1, 0.7, 2000, "Hi, I have a quick update.");
    assert_eq!(part_chunk.speaker, "Participant");
    assert_eq!(part_chunk.start_time_ms, 1000);
    assert_eq!(part_chunk.end_time_ms, 3000);

    // Case 3: Mixed / Overlapping conversation (energies close -> ratio <= 2.0)
    let mixed_chunk = transcriber.feed_audio(0.5, 0.4, 1500, "Yes, absolutely.");
    assert_eq!(mixed_chunk.speaker, "Mixed");
    assert_eq!(mixed_chunk.start_time_ms, 3000);
    assert_eq!(mixed_chunk.end_time_ms, 4500);

    assert_eq!(transcriber.chunks().len(), 3);
    assert_eq!(transcriber.current_time_ms(), 4500);

    // End-to-end PCM feed with mock whisper engine
    let mut mixer = DualAudioMixer::default();
    let mock_engine = MockWhisperEngine::default();
    let pcm_mic = vec![0.3f32; 16000];
    let pcm_loop = vec![0.05f32; 16000];

    let pcm_chunk = transcriber
        .feed_pcm_frames(&pcm_mic, &pcm_loop, &mut mixer, &mock_engine)
        .expect("feed_pcm_frames should succeed")
        .expect("should produce chunk");

    assert_eq!(pcm_chunk.speaker, "Host");
    assert_eq!(pcm_chunk.start_time_ms, 4500);
    assert_eq!(pcm_chunk.end_time_ms, 5500);
}
