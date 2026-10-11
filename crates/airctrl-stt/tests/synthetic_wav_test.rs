use airctrl_stt::{
    create_synthetic_wav, encode_wav, parse_wav_samples, SttError,
};

#[test]
fn test_synthetic_wav_header_magic_and_structure() {
    let wav = create_synthetic_wav(0.5, 16000, 1, 440.0, 0.5, false);

    assert!(wav.len() >= 44);
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    assert_eq!(&wav[12..16], b"fmt ");
    assert_eq!(&wav[36..40], b"data");

    let samples = parse_wav_samples(&wav).expect("parse valid synthetic wav");
    assert_eq!(samples.len(), 8000); // 0.5s * 16000
    for &s in &samples {
        assert!((-1.0..=1.0).contains(&s));
    }
}

#[test]
fn test_synthetic_wav_truncated_headers_rejected() {
    // 0 to 43 bytes all rejected
    for len in 0..44 {
        let truncated = vec![0u8; len];
        let err = parse_wav_samples(&truncated).expect_err("truncated header must fail");
        assert!(matches!(err, SttError::InvalidWavHeader(_)));
    }
}

#[test]
fn test_synthetic_wav_corrupted_magic_rejected() {
    let corrupt = create_synthetic_wav(0.1, 16000, 1, 440.0, 0.5, true);
    let err = parse_wav_samples(&corrupt).expect_err("corrupted magic must fail");
    assert!(matches!(err, SttError::InvalidWavHeader(_)));
}

#[test]
fn test_synthetic_wav_zero_amplitude() {
    let silent_wav = create_synthetic_wav(0.1, 16000, 1, 440.0, 0.0, false);
    let samples = parse_wav_samples(&silent_wav).unwrap();
    assert_eq!(samples.len(), 1600);
    for &s in &samples {
        assert_eq!(s, 0.0);
    }
}

#[test]
fn test_synthetic_wav_nyquist_frequency() {
    // 8000 Hz tone at 16,000 Hz sample rate (Nyquist boundary)
    let nyquist_wav = create_synthetic_wav(0.05, 16000, 1, 8000.0, 0.8, false);
    let samples = parse_wav_samples(&nyquist_wav).unwrap();
    assert_eq!(samples.len(), 800);
    for &s in &samples {
        assert!((-1.0..=1.0).contains(&s));
    }
}

#[test]
fn test_encode_and_parse_roundtrip() {
    let original = vec![0.0, 0.25, -0.25, 0.5, -0.5, 0.75, -0.75, 1.0, -1.0];
    let encoded = encode_wav(&original, 16000, 1).expect("encoding success");
    let decoded = parse_wav_samples(&encoded).expect("parsing success");

    assert_eq!(original.len(), decoded.len());
    for (o, d) in original.iter().zip(decoded.iter()) {
        // 16-bit quantization error tolerance
        assert!((o - d).abs() < 0.001);
    }
}
