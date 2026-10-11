use airctrl_meetings::compliance::{ComplianceAudioGenerator, ComplianceAudioInjector};
use airctrl_meetings::config::{ComplianceConfig, ComplianceMessageKind};
use std::io::Cursor;

#[test]
fn test_compliance_announcement_text_verbatim() {
    let injector = ComplianceAudioInjector::new();
    let config = ComplianceConfig::default();
    let res = injector.play_announcement(&config).unwrap();

    assert_eq!(res.announcement, "This meeting is being recorded");
    assert_eq!(res.announcement.len(), 30);
    assert_eq!(res.status, "Success");
    assert!(res.injected_into_stream);
}

#[test]
fn test_assistant_transcription_announcement() {
    let injector = ComplianceAudioInjector::new();
    let config = ComplianceConfig {
        message: ComplianceMessageKind::AssistantTranscriptionNotice,
        ..Default::default()
    };
    let res = injector.play_announcement(&config).unwrap();

    assert_eq!(
        res.announcement,
        "This meeting is being transcribed and automated by airctrl assistant."
    );
    assert_eq!(res.status, "Success");
}

#[test]
fn test_compliance_audio_duration_ms_boundary() {
    let injector = ComplianceAudioInjector::new();
    let config = ComplianceConfig::default();
    let res = injector.play_announcement(&config).unwrap();

    assert_eq!(res.duration_ms, 1450);
}

#[test]
fn test_compliance_audio_format_pcm_16khz() {
    let config = ComplianceConfig::default();
    assert_eq!(config.sample_rate, 16_000);
    assert_eq!(config.channels, 1);

    // 16,000 samples/sec * 1.450 sec = 23,200 samples
    let samples = ComplianceAudioGenerator::generate_pcm_buffer(
        config.sample_rate,
        config.duration_ms,
        config.effective_volume(),
    );
    assert_eq!(samples.len(), 23_200);

    // Verify WAV bytes encoded via hound
    let wav_bytes = ComplianceAudioGenerator::encode_wav_bytes(&samples, config.sample_rate).unwrap();
    let reader = hound::WavReader::new(Cursor::new(wav_bytes)).unwrap();
    let spec = reader.spec();

    assert_eq!(spec.channels, 1);
    assert_eq!(spec.sample_rate, 16_000);
    assert_eq!(spec.bits_per_sample, 16);
    assert_eq!(reader.len() as usize, 23_200);
}

#[test]
fn test_compliance_audio_zero_volume_floor_enforcement() {
    // Attempting 0.0 volume must be clamped to 0.50
    let config_zero = ComplianceConfig {
        volume: 0.0,
        ..Default::default()
    };
    assert_eq!(config_zero.effective_volume(), 0.50);

    // Attempting 0.25 volume must be clamped to 0.50
    let config_quarter = ComplianceConfig {
        volume: 0.25,
        ..Default::default()
    };
    assert_eq!(config_quarter.effective_volume(), 0.50);

    // Volume above 0.50 is preserved
    let config_high = ComplianceConfig {
        volume: 0.85,
        ..Default::default()
    };
    assert_eq!(config_high.effective_volume(), 0.85);

    // Volume capped at 1.0
    let config_capped = ComplianceConfig {
        volume: 1.5,
        ..Default::default()
    };
    assert_eq!(config_capped.effective_volume(), 1.0);
}

#[test]
fn test_recording_session_blocked_until_compliance_completes() {
    let injector = ComplianceAudioInjector::new();
    assert!(!injector.is_recording_allowed());

    let res = injector.play_announcement(&ComplianceConfig::default()).unwrap();
    assert_eq!(res.status, "Success");
    assert!(injector.is_recording_allowed());

    // Resetting gate locks recording again
    injector.reset_recording_gate();
    assert!(!injector.is_recording_allowed());
}

#[test]
fn test_repeated_compliance_playback() {
    let injector = ComplianceAudioInjector::new();
    let res1 = injector.play_announcement(&ComplianceConfig::default()).unwrap();
    let res2 = injector.play_announcement(&ComplianceConfig::default()).unwrap();

    assert_eq!(res1.status, "Success");
    assert_eq!(res2.status, "Success");
    assert_eq!(injector.fallback_sink().played_count(), 2);
}
