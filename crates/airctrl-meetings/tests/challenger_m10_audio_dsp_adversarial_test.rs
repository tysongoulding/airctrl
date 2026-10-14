//! challenger_m10_audio_dsp_adversarial_test.rs
//!
//! Milestone M10 (Meetily 1:1 Capability & Settings Parity) Audio DSP & Decoding Stress Tests:
//! 1. `DualAudioMixer` Stress Testing:
//!    - Mismatched buffer lengths (asymmetric, empty, prime, massive length disparities).
//!    - Non-finite floats: `f32::NAN`, `f32::INFINITY`, `f32::NEG_INFINITY`, IEEE 754 subnormals.
//!    - Ducking floor invariant: verifying ducking gain never drops below 0.25 under sustained continuous mic signal.
//!    - Hyperbolic tangent limiter: verifying strict clamping of extreme saturation inputs (+-100.0, +-1000.0, +-1e6) within `[-1.0, 1.0]`.
//! 2. `AudioDecoder` and Enhance Engine Adversarial Coverage:
//!    - Corrupted and truncated audio files (random binary noise, incomplete RIFF headers, truncated PCM payloads).
//!    - Unsupported and unknown file extensions (.exe, .txt, .xyz, missing extension).
//!    - Zero-byte audio files across supported extensions (.wav, .mp3, .ogg, .flac).
//!    - Non-existent file handling.
//!    - Multi-channel stereo downmixing (mono, stereo, 5.1, 7.1, unaligned buffer lengths, anti-phase cancellation).
//!    - Resampling decimation (48kHz 3-tap FIR, 44.1kHz, 32kHz, 8kHz interpolation).
//!    - Chunked transcription stream boundary handling.

use std::fs::File;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tempfile::tempdir;

use airctrl_meetings::audio::mixer::{DualAudioMixer, MixerConfig};
use airctrl_meetings::enhance::decoder::{AudioDecoder, SUPPORTED_AUDIO_EXTENSIONS};
use airctrl_meetings::enhance::preprocessor::{downmix_to_mono, resample_to_16k};
use airctrl_meetings::enhance::transcriber::ChunkedTranscriber;
use airctrl_meetings::error::MeetingError;
use airctrl_stt::MockWhisperEngine;

// ============================================================================
// 1. DualAudioMixer: Mismatched Buffer Lengths & Boundary Sizing
// ============================================================================

#[test]
fn test_adversarial_mixer_mismatched_buffer_lengths() {
    let mut mixer = DualAudioMixer::default();

    // 1.1 Both buffers empty
    let empty_out = mixer.mix(&[], &[]);
    assert!(empty_out.is_empty(), "Empty inputs must yield empty output");
    assert_eq!(mixer.total_samples_processed(), 0);

    // 1.2 Mic empty, Loopback has 5,000 samples
    let loopback_only = vec![0.4f32; 5_000];
    let out_lp = mixer.mix(&[], &loopback_only);
    assert_eq!(out_lp.len(), 5_000, "Output length must match loopback length when mic is empty");
    assert_eq!(mixer.total_samples_processed(), 5_000);
    // Since mic is empty (zero-padded to 0.0), ducking should NOT be active
    assert!(!mixer.is_ducking_active(), "Zero-padded mic should not trigger ducking");
    for &sample in &out_lp {
        assert!(sample.is_finite());
        assert!((-1.0..=1.0).contains(&sample));
    }

    // 1.3 Mic has 5,000 samples, Loopback empty
    let mic_only = vec![0.6f32; 5_000];
    let out_mic = mixer.mix(&mic_only, &[]);
    assert_eq!(out_mic.len(), 5_000, "Output length must match mic length when loopback is empty");
    assert_eq!(mixer.total_samples_processed(), 10_000);
    for &sample in &out_mic {
        assert!(sample.is_finite());
        assert!((-1.0..=1.0).contains(&sample));
    }

    // 1.4 Extreme asymmetry: 1 sample vs 100,000 samples
    let mic_single = vec![0.5f32; 1];
    let lp_huge = vec![0.2f32; 100_000];
    let out_asym = mixer.mix(&mic_single, &lp_huge);
    assert_eq!(out_asym.len(), 100_000, "Length must expand to maximum of the two slices");
    assert_eq!(mixer.total_samples_processed(), 110_000);

    // 1.5 Reverse extreme asymmetry: 100,000 samples vs 1 sample
    let mic_huge = vec![0.7f32; 100_000];
    let lp_single = vec![0.3f32; 1];
    let out_asym_rev = mixer.mix(&mic_huge, &lp_single);
    assert_eq!(out_asym_rev.len(), 100_000);
    assert_eq!(mixer.total_samples_processed(), 210_000);

    // 1.6 Prime length mismatches
    let mic_prime = vec![0.3f32; 1009];
    let lp_prime = vec![0.4f32; 2003];
    let out_prime = mixer.mix(&mic_prime, &lp_prime);
    assert_eq!(out_prime.len(), 2003);
    assert_eq!(mixer.total_samples_processed(), 212_003);
}

// ============================================================================
// 2. DualAudioMixer: Non-Finite Floats & Subnormals (IEEE 754 Sanitization)
// ============================================================================

#[test]
fn test_adversarial_mixer_non_finite_floats_and_subnormals() {
    let mut mixer = DualAudioMixer::default();

    // 2.1 Pure NaNs in both channels
    let nan_mic = vec![f32::NAN; 500];
    let nan_lp = vec![-f32::NAN; 500];
    let out_nan = mixer.mix(&nan_mic, &nan_lp);
    assert_eq!(out_nan.len(), 500);
    for (i, &sample) in out_nan.iter().enumerate() {
        assert!(
            sample.is_finite(),
            "Sample at index {i} must be finite despite NaN inputs: {sample}"
        );
        assert_eq!(sample, 0.0, "Sanitized NaN input must evaluate to 0.0");
    }
    assert!(mixer.last_mic_energy.is_finite(), "last_mic_energy must be finite");
    assert!(mixer.last_loopback_energy.is_finite(), "last_loopback_energy must be finite");
    assert!(mixer.current_envelope.is_finite(), "current_envelope must be finite");
    assert!(mixer.current_duck_gain.is_finite(), "current_duck_gain must be finite");

    // 2.2 Infinities (positive and negative)
    let inf_mic = vec![f32::INFINITY; 500];
    let neg_inf_lp = vec![f32::NEG_INFINITY; 500];
    let out_inf = mixer.mix(&inf_mic, &neg_inf_lp);
    assert_eq!(out_inf.len(), 500);
    for (i, &sample) in out_inf.iter().enumerate() {
        assert!(
            sample.is_finite(),
            "Sample at index {i} must be finite despite Inf inputs: {sample}"
        );
        assert_eq!(sample, 0.0, "Sanitized Infinity input must evaluate to 0.0");
    }

    // 2.3 IEEE 754 Subnormals (denormalized floats)
    let subnormal_pos = f32::from_bits(1); // 1.4012985e-45
    let subnormal_neg = -f32::from_bits(1);
    let subnormal_mid = f32::MIN_POSITIVE / 2.0;

    let subnormal_mic = vec![subnormal_pos, subnormal_neg, subnormal_mid, 1e-40f32];
    let normal_lp = vec![0.1f32; 4];
    let out_subnormal = mixer.mix(&subnormal_mic, &normal_lp);
    assert_eq!(out_subnormal.len(), 4);
    for (i, &sample) in out_subnormal.iter().enumerate() {
        assert!(
            sample.is_finite(),
            "Subnormal at index {i} must remain finite: {sample}"
        );
        assert!((-1.0..=1.0).contains(&sample));
    }

    // 2.4 Stochastic Float Representation Fuzzing
    let mut rng_seed: u64 = 0xDEADBEEFCAFE;
    let mut xorshift = || {
        rng_seed ^= rng_seed << 13;
        rng_seed ^= rng_seed >> 7;
        rng_seed ^= rng_seed << 17;
        rng_seed
    };

    let fuzzed_mic: Vec<f32> = (0..2000)
        .map(|_| {
            let r = xorshift() % 8;
            match r {
                0 => f32::NAN,
                1 => -f32::NAN,
                2 => f32::INFINITY,
                3 => f32::NEG_INFINITY,
                4 => f32::from_bits(1),
                5 => -100.0,
                6 => 100.0,
                _ => (xorshift() as f32 / u64::MAX as f32) * 2.0 - 1.0,
            }
        })
        .collect();

    let fuzzed_lp: Vec<f32> = (0..2000)
        .map(|_| {
            let r = xorshift() % 6;
            match r {
                0 => f32::NAN,
                1 => f32::INFINITY,
                2 => f32::NEG_INFINITY,
                3 => f32::from_bits(2),
                _ => (xorshift() as f32 / u64::MAX as f32) * 2.0 - 1.0,
            }
        })
        .collect();

    let fuzzed_out = mixer.mix(&fuzzed_mic, &fuzzed_lp);
    assert_eq!(fuzzed_out.len(), 2000);
    for (i, &sample) in fuzzed_out.iter().enumerate() {
        assert!(
            sample.is_finite(),
            "Fuzzed sample at index {i} must be finite: {sample}"
        );
        assert!(
            (-1.0..=1.0).contains(&sample),
            "Fuzzed sample at index {i} must be in [-1.0, 1.0]: {sample}"
        );
    }
}

// ============================================================================
// 3. DualAudioMixer: Ducking Gain Floor Invariant Under Sustained Mic Signal
// ============================================================================

#[test]
fn test_adversarial_ducking_gain_floor_sustained_signal() {
    let config = MixerConfig {
        ducking_gain: 0.25,
        ducking_threshold: 0.05,
        attack_ms: 10.0,
        release_ms: 100.0,
        ducking_enabled: true,
        ..Default::default()
    };
    let mut mixer = DualAudioMixer::new(config);

    // Initial condition
    assert_eq!(mixer.current_duck_gain(), 1.0);

    // Sustained continuous loud mic signal: 500,000 samples at 16kHz = 31.25 seconds of speech
    // Feed in chunks of 1000 samples to verify continuous state transitions
    let chunk_size = 1000;
    let total_samples = 500_000;
    let num_chunks = total_samples / chunk_size;

    let mic_chunk = vec![0.9f32; chunk_size];
    let loopback_chunk = vec![0.7f32; chunk_size];

    for chunk_idx in 0..num_chunks {
        let _ = mixer.mix(&mic_chunk, &loopback_chunk);

        let current_gain = mixer.current_duck_gain();

        // INVARIANT 1: Ducking gain must NEVER drop below 0.25
        assert!(
            current_gain >= 0.25,
            "VIOLATION at chunk {chunk_idx}: Ducking gain dropped below 0.25 statutory floor! Actual: {current_gain}"
        );

        // After attack period (>30ms = >480 samples), gain should have dropped to the floor
        if chunk_idx >= 5 {
            assert!(
                current_gain <= 0.2501,
                "Ducking gain should asymptote and stay at the floor (0.25) during continuous speech. Actual: {current_gain}"
            );
            assert!(
                (current_gain - 0.25).abs() < 1e-4,
                "Ducking gain should be locked at exactly 0.25. Actual: {current_gain}"
            );
        }
    }

    // Now test extreme overdrive mic input (e.g. 10.0, 100.0, 1000.0)
    for overdrive_amp in [2.0f32, 10.0, 100.0, 1000.0] {
        let overdrive_mic = vec![overdrive_amp; 2000];
        let normal_lp = vec![0.5f32; 2000];
        let _ = mixer.mix(&overdrive_mic, &normal_lp);

        let gain = mixer.current_duck_gain();
        assert!(
            gain >= 0.25,
            "VIOLATION under overdrive amp {overdrive_amp}: Ducking gain dropped below floor! Actual: {gain}"
        );
        assert!(
            (gain - 0.25).abs() < 1e-4,
            "Gain must stay clamped at floor under overdrive. Actual: {gain}"
        );
    }

    // Test custom ducking floor configurations: 0.10 and 0.50
    for custom_floor in [0.10f32, 0.40, 0.50, 0.05] {
        let custom_cfg = MixerConfig {
            ducking_gain: custom_floor,
            ducking_threshold: 0.05,
            attack_ms: 5.0,
            release_ms: 100.0,
            ducking_enabled: true,
            ..Default::default()
        };
        let mut custom_mixer = DualAudioMixer::new(custom_cfg);
        let loud_mic = vec![0.8f32; 20_000];
        let normal_lp = vec![0.5f32; 20_000];
        let _ = custom_mixer.mix(&loud_mic, &normal_lp);

        assert!(
            custom_mixer.current_duck_gain() >= custom_floor,
            "Ducking gain breached custom floor {custom_floor}! Actual: {}",
            custom_mixer.current_duck_gain()
        );
        assert!(
            (custom_mixer.current_duck_gain() - custom_floor).abs() < 1e-4,
            "Ducking gain should settle at custom floor {custom_floor}. Actual: {}",
            custom_mixer.current_duck_gain()
        );
    }

    // Verify recovery after silence
    let silent_mic = vec![0.0f32; 32_000]; // 2 seconds of silence
    let continuous_lp = vec![0.5f32; 32_000];
    let _ = mixer.mix(&silent_mic, &continuous_lp);
    assert!(
        mixer.current_duck_gain() > 0.99,
        "Ducking gain must recover towards 1.0 during silence: {}",
        mixer.current_duck_gain()
    );
}

// ============================================================================
// 4. DualAudioMixer: Hyperbolic Tangent Limiter Extreme Saturation Clamping
// ============================================================================

#[test]
fn test_adversarial_hyperbolic_tangent_limiter_extreme_saturation() {
    let config = MixerConfig {
        anti_clipping_enabled: true,
        ducking_enabled: false,
        ..Default::default()
    };
    let mut mixer = DualAudioMixer::new(config);

    let extreme_amplitudes = [
        10.0f32,
        -10.0,
        100.0,
        -100.0,
        1000.0,
        -1000.0,
        1e6,
        -1e6,
        1e20,
        -1e20,
    ];

    for &amp in &extreme_amplitudes {
        let mic = vec![amp; 100];
        let lp = vec![amp; 100];
        let out = mixer.mix(&mic, &lp);

        for (i, &sample) in out.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "Output at {i} must be finite for input {amp}: {sample}"
            );
            assert!(
                (-1.0..=1.0).contains(&sample),
                "VIOLATION: Sample at {i} breached [-1.0, 1.0] limiter boundary! Input: {amp}, Output: {sample}"
            );
            if amp > 0.0 {
                assert!(
                    sample > 0.9999,
                    "Positive extreme saturation should saturate to near 1.0: {sample}"
                );
            } else {
                assert!(
                    sample < -0.9999,
                    "Negative extreme saturation should saturate to near -1.0: {sample}"
                );
            }
        }
    }

    // Interference combinations
    let mic_pos = vec![500.0f32; 100];
    let lp_neg = vec![-500.0f32; 100];
    let out_cancel = mixer.mix(&mic_pos, &lp_neg);
    for &sample in &out_cancel {
        assert!(sample.abs() < 1e-4, "Cancelling signals should sum to ~0.0: {sample}");
    }

    // Limiter disabled fallback: verify standard hard clamp still protects [-1.0, 1.0]
    let no_limiter_cfg = MixerConfig {
        anti_clipping_enabled: false,
        ducking_enabled: false,
        ..Default::default()
    };
    let mut raw_mixer = DualAudioMixer::new(no_limiter_cfg);
    let hot_mic = vec![50.0f32; 100];
    let hot_lp = vec![50.0f32; 100];
    let out_raw = raw_mixer.mix(&hot_mic, &hot_lp);
    for &sample in &out_raw {
        assert!(
            (-1.0..=1.0).contains(&sample),
            "Hard clamp must strictly enforce [-1.0, 1.0]: {sample}"
        );
        assert_eq!(sample, 1.0, "Hard clamp of 100.0 must equal exactly 1.0");
    }
}

// ============================================================================
// 5. AudioDecoder: Corrupt, Truncated, Unknown, and Zero-Byte Files
// ============================================================================

#[test]
fn test_adversarial_audio_decoder_error_handling() {
    let dir = tempdir().expect("Failed to create tempdir");

    // 5.1 Non-existent file
    let missing_path = dir.path().join("does_not_exist.wav");
    let res_missing = AudioDecoder::decode_file(&missing_path);
    match res_missing {
        Err(MeetingError::AudioFileNotFound(path)) => {
            assert!(path.contains("does_not_exist.wav"));
        }
        other => panic!("Expected AudioFileNotFound, got: {other:?}"),
    }

    // 5.2 Zero-byte files across all supported extensions
    for ext in SUPPORTED_AUDIO_EXTENSIONS {
        let zero_path = dir.path().join(format!("empty_file.{ext}"));
        File::create(&zero_path).expect("Failed to create empty file");

        let res_zero = AudioDecoder::decode_file(&zero_path);
        match res_zero {
            Err(MeetingError::EmptyAudioFile(path)) => {
                assert!(path.contains(&format!("empty_file.{ext}")));
            }
            other => panic!("Expected EmptyAudioFile for zero-byte .{ext}, got: {other:?}"),
        }
    }

    // 5.3 Unknown and unsupported extensions
    let unsupported_exts = ["exe", "txt", "py", "json", "dat", "bin", "xyz", "dll"];
    for ext in unsupported_exts {
        let unsup_path = dir.path().join(format!("dummy.{ext}"));
        let mut f = File::create(&unsup_path).expect("Failed to create file");
        f.write_all(b"Some non-audio content").expect("write");

        let res_unsup = AudioDecoder::decode_file(&unsup_path);
        match res_unsup {
            Err(MeetingError::UnsupportedAudioFormat(msg)) => {
                assert!(msg.contains(ext), "Error message should mention .{ext}");
            }
            other => panic!("Expected UnsupportedAudioFormat for .{ext}, got: {other:?}"),
        }
    }

    // 5.4 File with no extension
    let no_ext_path = dir.path().join("audio_without_extension");
    let mut f = File::create(&no_ext_path).expect("Failed to create file");
    f.write_all(b"audio payload").expect("write");
    let res_no_ext = AudioDecoder::decode_file(&no_ext_path);
    match res_no_ext {
        Err(MeetingError::UnsupportedAudioFormat(_)) => {}
        other => panic!("Expected UnsupportedAudioFormat for file without extension, got: {other:?}"),
    }

    // 5.5 Corrupt WAV file (random garbage bytes)
    let corrupt_wav_path = dir.path().join("garbage.wav");
    let mut f = File::create(&corrupt_wav_path).expect("create");
    f.write_all(&[0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04, 0xFF, 0xFE, 0xFD])
        .expect("write");
    let res_corrupt = AudioDecoder::decode_file(&corrupt_wav_path);
    match res_corrupt {
        Err(MeetingError::AudioDecodeFailed(msg)) => {
            assert!(msg.contains("WAV decode error") || msg.contains("decode"));
        }
        other => panic!("Expected AudioDecodeFailed for garbage.wav, got: {other:?}"),
    }

    // 5.6 Truncated WAV Header (RIFF magic only, 4 bytes)
    let trunc_riff_path = dir.path().join("trunc_riff.wav");
    let mut f = File::create(&trunc_riff_path).expect("create");
    f.write_all(b"RIFF").expect("write");
    let res_trunc_riff = AudioDecoder::decode_file(&trunc_riff_path);
    match res_trunc_riff {
        Err(MeetingError::AudioDecodeFailed(_)) => {}
        other => panic!("Expected AudioDecodeFailed for truncated RIFF, got: {other:?}"),
    }

    // 5.7 Incomplete WAV Header (20 bytes: RIFF + size + WAVE + fmt)
    let trunc_hdr_path = dir.path().join("trunc_hdr.wav");
    let mut f = File::create(&trunc_hdr_path).expect("create");
    f.write_all(b"RIFF\x24\x00\x00\x00WAVEfmt \x10\x00\x00\x00").expect("write");
    let res_trunc_hdr = AudioDecoder::decode_file(&trunc_hdr_path);
    match res_trunc_hdr {
        Err(MeetingError::AudioDecodeFailed(_)) => {}
        other => panic!("Expected AudioDecodeFailed for incomplete WAV header, got: {other:?}"),
    }

    // 5.8 Corrupt MP3/OGG file with garbage bytes
    let corrupt_mp3_path = dir.path().join("garbage.mp3");
    let mut f = File::create(&corrupt_mp3_path).expect("create");
    f.write_all(b"Not an MP3 audio file at all").expect("write");
    let res_mp3 = AudioDecoder::decode_file(&corrupt_mp3_path);
    match res_mp3 {
        Err(MeetingError::AudioDecodeFailed(msg)) => {
            assert!(msg.contains("mp3"));
        }
        other => panic!("Expected AudioDecodeFailed for garbage.mp3, got: {other:?}"),
    }

    // 5.9 Genuine Synthetic WAV Round-Trip Verification
    let valid_wav_path = dir.path().join("valid_test.wav");
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&valid_wav_path, spec).expect("create WavWriter");
    for i in 0..16_000 {
        let sample = (32767.0 * (i as f32 * 0.05).sin()) as i16;
        writer.write_sample(sample).expect("write_sample");
    }
    writer.finalize().expect("finalize");

    let decoded = AudioDecoder::decode_file(&valid_wav_path).expect("Must decode valid WAV");
    assert_eq!(decoded.format, "wav");
    assert_eq!(decoded.channels, 1);
    assert_eq!(decoded.sample_rate, 16_000);
    assert_eq!(decoded.samples.len(), 16_000);
    assert_eq!(decoded.duration_secs, 1);
    for &sample in &decoded.samples {
        assert!((-1.0..=1.0).contains(&sample));
    }
}

// ============================================================================
// 6. Preprocessor: Multi-Channel Stereo & Surround Downmix and Resampling
// ============================================================================

#[test]
fn test_adversarial_multi_channel_downmix_and_resampling() {
    // 6.1 Standard Stereo Downmixing (channels = 2)
    let stereo_interleaved = vec![
        1.0f32, 0.0, // Frame 0: (1.0 + 0.0)/2 = 0.5
        0.5, 0.5,    // Frame 1: (0.5 + 0.5)/2 = 0.5
        0.8, -0.8,   // Frame 2: anti-phase cancellation = 0.0
        -1.0, -1.0,  // Frame 3: (-1.0 + -1.0)/2 = -1.0
    ];
    let mono = downmix_to_mono(&stereo_interleaved, 2);
    assert_eq!(mono.len(), 4);
    assert!((mono[0] - 0.5).abs() < 1e-5);
    assert!((mono[1] - 0.5).abs() < 1e-5);
    assert!(mono[2].abs() < 1e-5);
    assert!((mono[3] - (-1.0)).abs() < 1e-5);

    // 6.2 High-Channel Surround Downmixing (5.1 = 6 channels, 7.1 = 8 channels)
    let surround_6ch: Vec<f32> = vec![0.6f32; 60]; // 10 frames of 6 channels
    let mono_6ch = downmix_to_mono(&surround_6ch, 6);
    assert_eq!(mono_6ch.len(), 10);
    for &sample in &mono_6ch {
        assert!((sample - 0.6).abs() < 1e-5);
    }

    let surround_8ch: Vec<f32> = vec![0.8f32; 80]; // 10 frames of 8 channels
    let mono_8ch = downmix_to_mono(&surround_8ch, 8);
    assert_eq!(mono_8ch.len(), 10);
    for &sample in &mono_8ch {
        assert!((sample - 0.8).abs() < 1e-5);
    }

    // 6.3 Boundary Channel Numbers
    // Mono (channels = 1): should return exact clamped copy
    let mono_input = vec![0.3f32, -0.5, 1.2, -1.5];
    let mono_out = downmix_to_mono(&mono_input, 1);
    assert_eq!(mono_out.len(), 4);
    assert_eq!(mono_out[0], 0.3);
    assert_eq!(mono_out[1], -0.5);
    assert_eq!(mono_out[2], 1.0, "1.2 must be clamped to 1.0");
    assert_eq!(mono_out[3], -1.0, "-1.5 must be clamped to -1.0");

    // Zero channels (channels = 0): should not panic with divide-by-zero
    let zero_ch_out = downmix_to_mono(&mono_input, 0);
    assert_eq!(zero_ch_out.len(), 4);

    // 6.4 Unaligned buffer length (trailing incomplete frame)
    let unaligned = vec![0.2f32; 9]; // 9 samples with 4 channels = 2 full frames, 1 trailing
    let unaligned_out = downmix_to_mono(&unaligned, 4);
    assert_eq!(unaligned_out.len(), 2, "Must truncate partial frame without out-of-bounds");

    // 6.5 Resampling: 48,000 Hz -> 16,000 Hz (3:1 integer decimation with 3-tap FIR filter)
    let pcm_48k = vec![0.6f32; 48_000];
    let pcm_16k = resample_to_16k(&pcm_48k, 48_000);
    assert_eq!(pcm_16k.len(), 16_000);
    for &sample in &pcm_16k {
        assert!((sample - 0.6).abs() < 1e-5);
        assert!((-1.0..=1.0).contains(&sample));
    }

    // 6.6 Resampling: 44,100 Hz, 32,000 Hz, 8,000 Hz
    let pcm_44k = vec![0.4f32; 44_100];
    let resampled_44k = resample_to_16k(&pcm_44k, 44_100);
    assert_eq!(resampled_44k.len(), 16_000);
    for &sample in &resampled_44k {
        assert!((sample - 0.4).abs() < 1e-3);
    }

    let pcm_8k = vec![0.5f32; 8_000];
    let upsampled_8k = resample_to_16k(&pcm_8k, 8_000);
    assert_eq!(upsampled_8k.len(), 16_000);
    for &sample in &upsampled_8k {
        assert!((sample - 0.5).abs() < 1e-3);
    }

    // Resampling boundary: 16,000 Hz passthrough
    let pcm_already_16k = vec![0.3f32; 16_000];
    let pass_16k = resample_to_16k(&pcm_already_16k, 16_000);
    assert_eq!(pass_16k.len(), 16_000);
    assert_eq!(pass_16k, pcm_already_16k);

    // Empty input resampling
    let empty_resample = resample_to_16k(&[], 48_000);
    assert!(empty_resample.is_empty());
}

// ============================================================================
// 7. Enhance Engine: Chunked Transcriber Boundary & Progress Reporting
// ============================================================================

#[test]
fn test_adversarial_chunked_transcriber_boundaries() {
    let engine = MockWhisperEngine::default();

    // 7.1 Empty audio buffer
    let (segments, full_text) =
        ChunkedTranscriber::transcribe_stream(&[], "meeting_empty", &engine, None)
            .expect("Empty stream must succeed");
    assert!(segments.is_empty());
    assert!(full_text.is_empty());

    // 7.2 Multi-chunk streaming with progress reporting
    // 30 seconds = 480,000 samples at 16kHz
    // Feed 75 seconds of audio = 1,200,000 samples (should yield 3 chunks: 30s, 30s, 15s)
    let samples_75s = vec![0.05f32; 1_200_000];
    let progress_calls = Arc::new(AtomicUsize::new(0));
    let progress_cb = {
        let calls = progress_calls.clone();
        Some(Arc::new(move |progress: f32| {
            assert!((0.0..=1.0).contains(&progress));
            calls.fetch_add(1, Ordering::SeqCst);
        }) as Arc<dyn Fn(f32) + Send + Sync>)
    };

    let (segments, full_text) =
        ChunkedTranscriber::transcribe_stream(&samples_75s, "meeting_75s", &engine, progress_cb)
            .expect("Transcribe stream must succeed");

    assert_eq!(segments.len(), 3, "75s audio must produce exactly 3 chunks");
    assert_eq!(progress_calls.load(Ordering::SeqCst), 3);

    // Chunk 0: 0ms -> 30,000ms
    assert_eq!(segments[0].start_ms, 0);
    assert_eq!(segments[0].end_ms, 30_000);
    // Chunk 1: 30,000ms -> 60,000ms
    assert_eq!(segments[1].start_ms, 30_000);
    assert_eq!(segments[1].end_ms, 60_000);
    // Chunk 2: 60,000ms -> 75,000ms
    assert_eq!(segments[2].start_ms, 60_000);
    assert_eq!(segments[2].end_ms, 75_000);

    assert!(!full_text.is_empty());
}
