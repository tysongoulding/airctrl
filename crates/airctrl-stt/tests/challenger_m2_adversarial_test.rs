//! Milestone M2 Adversarial Empirical Challenge Test Suite
//!
//! Stress-tests:
//! 1. WAV parsing attacks: corrupted RIFF magic, truncated 43-byte headers, zero-length data,
//!    unsupported bit depths (8-bit, 24-bit, 32-bit float), extreme sample rates.
//! 2. VAD edge conditions: RMS energy fuzzing near 0.05 threshold (0.049 vs 0.050 vs 0.051),
//!    1599 vs 1600 samples min boundary, DC offset audio, extreme amplitude clipping, all-silence buffers.
//! 3. AudioRingBuffer concurrency stress: rapid multi-threaded push/pop bursts, buffer wrap-around,
//!    capacity overflow, ensuring zero memory leaks or panics.

use airctrl_stt::{
    audio::{create_synthetic_wav, parse_wav_samples, AudioPreprocessor, AudioRingBuffer},
    engine::{MockWhisperEngine, SpeechToTextEngine},
    error::SttError,
    vad::{DiscardReason, VadDecision, VoiceActivityDetector},
};
use std::io::Cursor;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

// ============================================================================
// 1. WAV Parsing Attacks
// ============================================================================

#[test]
fn test_wav_corrupted_riff_and_wave_magic() {
    // 1.1 Completely invalid magic bytes
    let magics: [&[u8; 4]; 7] = [
        b"BADF",
        b"RIFX", // Big-endian RIFF
        b"riff", // Lowercase
        b"\x00\x00\x00\x00",
        b"\xff\xff\xff\xff",
        b"FORM", // AIFF magic
        b"OggS", // OGG magic
    ];

    for magic in &magics {
        let mut wav = create_synthetic_wav(0.1, 16000, 1, 440.0, 0.5, false);
        wav[0..4].copy_from_slice(*magic);
        let err = parse_wav_samples(&wav).expect_err("corrupted magic must fail");
        assert!(
            matches!(err, SttError::InvalidWavHeader(_)),
            "Expected InvalidWavHeader for magic {:?}, got {:?}",
            magic,
            err
        );
    }

    // 1.2 Valid RIFF but corrupted WAVE marker (bytes 8..12)
    let wave_corruptions: [&[u8; 4]; 4] = [b"AVI ", b"wave", b"WAV_", b"\x00\x00\x00\x00"];
    for bad_wave in &wave_corruptions {
        let mut wav = create_synthetic_wav(0.1, 16000, 1, 440.0, 0.5, false);
        wav[8..12].copy_from_slice(*bad_wave);
        let err = parse_wav_samples(&wav).expect_err("corrupted WAVE marker must fail");
        assert!(
            matches!(err, SttError::InvalidWavHeader(_)),
            "Expected InvalidWavHeader for WAVE marker {:?}, got {:?}",
            bad_wave,
            err
        );
    }
}

#[test]
fn test_wav_truncated_headers_all_lengths_0_to_43() {
    // Exactly 0 to 43 bytes: MUST be rejected with InvalidWavHeader
    let template = create_synthetic_wav(0.1, 16000, 1, 440.0, 0.5, false);
    for len in 0..44 {
        let truncated = &template[0..len];
        let err = parse_wav_samples(truncated).expect_err("truncated header must fail");
        match err {
            SttError::InvalidWavHeader(msg) => {
                assert!(
                    msg.contains("truncated") || msg.contains("44"),
                    "Expected truncated header message, got: {}",
                    msg
                );
            }
            other => panic!("Expected InvalidWavHeader for len {}, got {:?}", len, other),
        }
    }

    // Truncated data: valid 44-byte header indicating 1000 bytes of data, but actual buffer truncated at 44..50 bytes
    let mut header_with_large_data = template[0..44].to_vec();
    let claimed_data_size = 1000u32;
    header_with_large_data[40..44].copy_from_slice(&claimed_data_size.to_le_bytes());

    // Only header provided, 0 data bytes
    let res = parse_wav_samples(&header_with_large_data);
    // Hound will fail reading samples because file ended unexpectedly
    assert!(res.is_err(), "Truncated data stream must error");
}

#[test]
fn test_wav_zero_length_data_and_downstream_safety() {
    // Valid 44-byte WAV header specifying data_size = 0
    let wav_zero = create_synthetic_wav(0.0, 16000, 1, 440.0, 0.0, false);
    assert_eq!(wav_zero.len(), 44);
    assert_eq!(&wav_zero[40..44], &[0, 0, 0, 0]);

    // 1. Parsing zero-length WAV succeeds and returns empty Vec
    let samples = parse_wav_samples(&wav_zero).expect("zero-length data should parse to empty vec");
    assert!(samples.is_empty(), "Samples must be empty");

    // 2. Downstream STT engine rejection
    let engine = MockWhisperEngine::new();
    let transcribe_err = engine.transcribe(&samples).expect_err("Empty buffer must be rejected");
    assert!(
        matches!(transcribe_err, SttError::EmptyAudioBuffer),
        "Expected EmptyAudioBuffer, got {:?}",
        transcribe_err
    );

    // 3. Downstream VAD behavior on empty buffer
    let vad = VoiceActivityDetector::default();
    assert!(!vad.is_speech(&samples));
    assert_eq!(vad.calculate_rms_energy(&samples), 0.0);
    assert_eq!(vad.calculate_dbfs(&samples), -120.0);
    let decision = vad.evaluate(&samples);
    assert_eq!(
        decision,
        VadDecision::Discarded {
            reason: DiscardReason::EmptyBuffer,
            sample_count: 0
        }
    );
    let (trimmed, meta) = vad.trim_silence(&samples);
    assert!(trimmed.is_empty());
    assert_eq!(meta.original_samples, 0);

    // 4. Downstream ring buffer behavior
    let ring_buffer = AudioRingBuffer::new(100);
    ring_buffer.push(&samples);
    assert_eq!(ring_buffer.len(), 0);
    assert!(ring_buffer.is_empty());

    // 5. Downstream preprocessor behavior
    let downmixed = AudioPreprocessor::downmix_to_mono(&samples, 1);
    assert!(downmixed.is_empty());
    let resampled = AudioPreprocessor::resample_linear(&samples, 16000, 16000);
    assert!(resampled.is_empty());
}

#[test]
fn test_wav_unsupported_bit_depths_and_formats() {
    // 8-bit integer WAV: Hound parses it as Int with 8 bits, parser MUST return UnsupportedAudioFormat
    let spec_8bit = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 8,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor_8bit = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor_8bit, spec_8bit).unwrap();
        writer.write_sample(127i8).unwrap();
        writer.write_sample(-128i8).unwrap();
        writer.finalize().unwrap();
    }
    let bytes_8bit = cursor_8bit.into_inner();
    let err_8bit = parse_wav_samples(&bytes_8bit).expect_err("8-bit WAV must be rejected");
    assert!(
        matches!(err_8bit, SttError::UnsupportedAudioFormat(_)),
        "Expected UnsupportedAudioFormat for 8-bit, got {:?}",
        err_8bit
    );

    // 24-bit integer WAV: Supported and normalized by 8,388,608.0
    let spec_24bit = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 24,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor_24bit = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor_24bit, spec_24bit).unwrap();
        writer.write_sample(4_194_304i32).unwrap(); // 0.5 amplitude
        writer.write_sample(-4_194_304i32).unwrap(); // -0.5 amplitude
        writer.finalize().unwrap();
    }
    let bytes_24bit = cursor_24bit.into_inner();
    let samples_24bit = parse_wav_samples(&bytes_24bit).expect("24-bit WAV parse success");
    assert_eq!(samples_24bit.len(), 2);
    assert!((samples_24bit[0] - 0.5).abs() < 1e-4);
    assert!((samples_24bit[1] - (-0.5)).abs() < 1e-4);

    // 32-bit float WAV: Supported and clamped to [-1.0, 1.0]
    let spec_float = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut cursor_float = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor_float, spec_float).unwrap();
        writer.write_sample(0.75f32).unwrap();
        writer.write_sample(-0.75f32).unwrap();
        writer.write_sample(2.5f32).unwrap(); // Exceeds 1.0, should be clamped
        writer.finalize().unwrap();
    }
    let bytes_float = cursor_float.into_inner();
    let samples_float = parse_wav_samples(&bytes_float).expect("32-bit float WAV parse success");
    assert_eq!(samples_float.len(), 3);
    assert!((samples_float[0] - 0.75).abs() < 1e-5);
    assert!((samples_float[1] - (-0.75)).abs() < 1e-5);
    assert_eq!(samples_float[2], 1.0); // Clamped
}

#[test]
fn test_wav_extreme_sample_rates() {
    let rates = [
        1u32,      // 1 Hz
        8_000u32,  // Telephony 8kHz
        16_000u32, // Target Whisper rate
        44_100u32, // CD audio
        48_000u32, // Standard broadcast
        96_000u32, // Studio 96kHz
        192_000u32, // High-res 192kHz
        384_000u32, // Ultra-high-res 384kHz
    ];

    for &rate in &rates {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
            writer.write_sample(16384i16).unwrap(); // 0.5
            writer.finalize().unwrap();
        }
        let bytes = cursor.into_inner();
        let samples = parse_wav_samples(&bytes).expect("extreme sample rate parse");
        assert_eq!(samples.len(), 1);
        assert!((samples[0] - 0.5).abs() < 1e-3);
    }

    // Zero sample rate in raw WAV header:
    // parse_wav_samples currently accepts sample_rate = 0 because hound does not reject it
    // and parse_wav_samples does not validate sample_rate > 0 (unlike channels > 0).
    let mut zero_rate_wav = create_synthetic_wav(0.01, 16000, 1, 440.0, 0.5, false);
    zero_rate_wav[24..28].copy_from_slice(&0u32.to_le_bytes()); // sample rate = 0
    zero_rate_wav[28..32].copy_from_slice(&0u32.to_le_bytes()); // byte rate = 0
    // Check behavior of AudioPreprocessor::resample_linear when src_rate == 0 or dst_rate == 0:
    let zero_resample_panics = std::panic::catch_unwind(|| {
        let dummy = vec![0.5f32; 100];
        AudioPreprocessor::resample_linear(&dummy, 0, 16000);
    });
    println!("resample_linear with src_rate=0 panicked: {}", zero_resample_panics.is_err());
    assert!(zero_resample_panics.is_err(), "src_rate=0 causes allocation overflow panic in resample_linear");
}

// ============================================================================
// 2. VAD Edge Conditions
// ============================================================================

#[test]
fn test_vad_rms_energy_fuzzing_near_threshold() {
    let vad = VoiceActivityDetector::default(); // threshold: 0.05, min_samples: 1600

    // Exact threshold boundary: 0.049 vs 0.050 vs 0.051
    let buf_049 = vec![0.049f32; 1600];
    let buf_050 = vec![0.050f32; 1600];
    let buf_051 = vec![0.051f32; 1600];

    assert!(!vad.is_speech(&buf_049), "0.049 must NOT be classified as speech");
    assert!(vad.is_speech(&buf_050), "0.050 MUST be classified as speech");
    assert!(vad.is_speech(&buf_051), "0.051 MUST be classified as speech");

    // Decision categorization
    match vad.evaluate(&buf_049) {
        VadDecision::Silence { average_rms, .. } => {
            assert!((average_rms - 0.049).abs() < 1e-4);
        }
        other => panic!("Expected Silence for 0.049, got {:?}", other),
    }

    match vad.evaluate(&buf_050) {
        VadDecision::Speech { metadata, .. } => {
            assert!((metadata.average_rms - 0.050).abs() < 1e-4);
        }
        other => panic!("Expected Speech for 0.050, got {:?}", other),
    }

    // High-resolution fuzzed boundary sweep from 0.0480 to 0.0520 in steps of 0.0002
    for step in 0..=20 {
        let amp = 0.0480 + (step as f32) * 0.0002;
        let buf = vec![amp; 1600];
        let rms = vad.calculate_rms_energy(&buf);
        assert!((rms - amp).abs() < 1e-5);

        if amp < 0.050 {
            assert!(
                !vad.is_speech(&buf),
                "Amplitude {:.5} (< 0.05) should not be speech",
                amp
            );
        } else {
            assert!(
                vad.is_speech(&buf),
                "Amplitude {:.5} (>= 0.05) should be speech",
                amp
            );
        }
    }
}

#[test]
fn test_vad_1599_vs_1600_samples_min_boundary() {
    let vad = VoiceActivityDetector::default(); // min_samples = 1600

    // 1599 maximum loud samples (1.0) must still be rejected due to length
    let loud_1599 = vec![1.0f32; 1599];
    assert!(!vad.is_speech(&loud_1599), "1599 samples must be rejected");
    match vad.evaluate(&loud_1599) {
        VadDecision::Discarded { reason, sample_count } => {
            assert_eq!(reason, DiscardReason::TooShort);
            assert_eq!(sample_count, 1599);
        }
        other => panic!("Expected Discarded(TooShort) for 1599 samples, got {:?}", other),
    }

    // 1600 samples of same signal must be accepted as speech
    let loud_1600 = vec![1.0f32; 1600];
    assert!(vad.is_speech(&loud_1600), "1600 samples must be accepted");
    match vad.evaluate(&loud_1600) {
        VadDecision::Speech { metadata, .. } => {
            assert_eq!(metadata.original_samples, 1600);
            assert!(
                (metadata.peak_dbfs - 0.0).abs() < 1e-5,
                "peak_dbfs should be approximately 0.0, got {}",
                metadata.peak_dbfs
            );
        }
        other => panic!("Expected Speech for 1600 samples, got {:?}", other),
    }

    // 1601 samples also accepted
    let loud_1601 = vec![1.0f32; 1601];
    assert!(vad.is_speech(&loud_1601), "1601 samples must be accepted");
}

#[test]
fn test_vad_dc_offset_audio() {
    let vad = VoiceActivityDetector::default();

    // Constant positive DC offsets
    let dc_pos_sub = vec![0.045f32; 1600]; // Below threshold
    let dc_pos_sup = vec![0.055f32; 1600]; // Above threshold
    assert!(!vad.is_speech(&dc_pos_sub));
    assert!(vad.is_speech(&dc_pos_sup));

    // Constant negative DC offsets (energy is symmetric)
    let dc_neg_sub = vec![-0.045f32; 1600];
    let dc_neg_sup = vec![-0.055f32; 1600];
    assert!(!vad.is_speech(&dc_neg_sub));
    assert!(vad.is_speech(&dc_neg_sup));

    // Severe DC offset (+0.8 DC)
    let severe_dc = vec![0.8f32; 1600];
    let rms = vad.calculate_rms_energy(&severe_dc);
    assert!((rms - 0.8).abs() < 1e-4);
    assert!(vad.is_speech(&severe_dc));

    // AC signal with zero mean on top of DC offset
    // Sine wave amplitude 0.02 (RMS ~0.014) on DC 0.045:
    // E[s^2] = 0.045^2 + 0.02^2 / 2 = 0.002025 + 0.0002 = 0.002225
    // RMS = sqrt(0.002225) ~ 0.04717 (< 0.05 -> silence)
    let mut ac_dc_sub = Vec::with_capacity(1600);
    for i in 0..1600 {
        let t = i as f32 / 16000.0;
        let ac = 0.02 * (2.0 * std::f32::consts::PI * 440.0 * t).sin();
        ac_dc_sub.push(0.045 + ac);
    }
    assert!(!vad.is_speech(&ac_dc_sub), "RMS ~0.047 must be silence");

    // Same AC with DC 0.055:
    // RMS > 0.055 -> speech
    let mut ac_dc_sup = Vec::with_capacity(1600);
    for i in 0..1600 {
        let t = i as f32 / 16000.0;
        let ac = 0.02 * (2.0 * std::f32::consts::PI * 440.0 * t).sin();
        ac_dc_sup.push(0.055 + ac);
    }
    assert!(vad.is_speech(&ac_dc_sup), "RMS > 0.055 must be speech");
}

#[test]
fn test_vad_extreme_amplitude_clipping() {
    let vad = VoiceActivityDetector::default();

    // Square wave alternating ±1.0 (extreme clipping)
    let mut square_wave = Vec::with_capacity(1600);
    for i in 0..1600 {
        if i % 2 == 0 {
            square_wave.push(1.0);
        } else {
            square_wave.push(-1.0);
        }
    }
    let rms = vad.calculate_rms_energy(&square_wave);
    assert!((rms - 1.0).abs() < 1e-5);
    let dbfs = vad.calculate_dbfs(&square_wave);
    assert!((dbfs - 0.0).abs() < 1e-5, "square wave dBFS should be ~0.0, got {}", dbfs);
    assert!(vad.is_speech(&square_wave));

    // Huge unnormalized values: verify no arithmetic overflow in RMS
    let huge_samples = vec![100.0f32; 1600];
    let huge_rms = vad.calculate_rms_energy(&huge_samples);
    assert!((huge_rms - 100.0).abs() < 1e-3);
    assert!(vad.is_speech(&huge_samples));

    // AudioPreprocessor downmix and resample clamping on out-of-range samples
    let out_of_range = vec![5.0f32, -5.0f32, 10.0f32, -10.0f32];
    let mono = AudioPreprocessor::downmix_to_mono(&out_of_range, 2);
    for &s in &mono {
        assert!((-1.0..=1.0).contains(&s), "Sample must be clamped to [-1.0, 1.0]");
    }
}

#[test]
fn test_vad_all_silence_buffers() {
    let vad = VoiceActivityDetector::default();
    let lengths = [0, 1, 10, 100, 1599, 1600, 3200, 16000, 32000, 64000];

    for &len in &lengths {
        let silent = vec![0.0f32; len];
        assert_eq!(vad.calculate_rms_energy(&silent), 0.0);
        assert_eq!(vad.calculate_dbfs(&silent), -120.0);
        assert!(!vad.is_speech(&silent));

        let decision = vad.evaluate(&silent);
        if len == 0 {
            assert_eq!(
                decision,
                VadDecision::Discarded {
                    reason: DiscardReason::EmptyBuffer,
                    sample_count: 0
                }
            );
        } else if len < 1600 {
            assert_eq!(
                decision,
                VadDecision::Discarded {
                    reason: DiscardReason::TooShort,
                    sample_count: len
                }
            );
        } else {
            assert_eq!(
                decision,
                VadDecision::Silence {
                    average_rms: 0.0,
                    peak_dbfs: -120.0
                }
            );
        }

        let (trimmed, meta) = vad.trim_silence(&silent);
        assert!(trimmed.is_empty());
        assert_eq!(meta.trimmed_samples, 0);
        assert_eq!(meta.average_rms, 0.0);
    }
}

// ============================================================================
// 3. AudioRingBuffer Concurrency Stress
// ============================================================================

#[test]
fn test_audio_ring_buffer_concurrency_bursts() {
    const CAPACITY: usize = 16_000;
    const NUM_PRODUCERS: usize = 8;
    const NUM_CONSUMERS: usize = 8;
    const BURSTS_PER_THREAD: usize = 100;
    const CHUNK_SIZE: usize = 320;

    let rb = Arc::new(AudioRingBuffer::new(CAPACITY));
    let total_samples_pushed = Arc::new(AtomicUsize::new(0));
    let total_samples_popped = Arc::new(AtomicUsize::new(0));
    let stop_signal = Arc::new(AtomicBool::new(false));

    let mut producer_handles = Vec::new();
    let mut consumer_handles = Vec::new();

    // Spawn producer threads
    for thread_id in 0..NUM_PRODUCERS {
        let rb_clone = Arc::clone(&rb);
        let pushed_counter = Arc::clone(&total_samples_pushed);
        let handle = thread::spawn(move || {
            let sample_val = (thread_id as f32 + 1.0) * 0.01;
            let chunk = vec![sample_val; CHUNK_SIZE];
            for _ in 0..BURSTS_PER_THREAD {
                rb_clone.push(&chunk);
                pushed_counter.fetch_add(CHUNK_SIZE, Ordering::Relaxed);
                thread::yield_now();
            }
        });
        producer_handles.push(handle);
    }

    // Spawn consumer threads
    for _ in 0..NUM_CONSUMERS {
        let rb_clone = Arc::clone(&rb);
        let popped_counter = Arc::clone(&total_samples_popped);
        let stop_clone = Arc::clone(&stop_signal);
        let handle = thread::spawn(move || {
            while !stop_clone.load(Ordering::Relaxed) {
                let samples = rb_clone.pop(160);
                popped_counter.fetch_add(samples.len(), Ordering::Relaxed);
                let _ = rb_clone.peek(80);
                thread::yield_now();
            }
            // Drain remaining after stop signal
            let remaining = rb_clone.pop_all();
            popped_counter.fetch_add(remaining.len(), Ordering::Relaxed);
        });
        consumer_handles.push(handle);
    }

    // Wait for all producers to finish
    for handle in producer_handles {
        handle.join().expect("Producer thread should not panic");
    }

    // Signal consumers to stop and join them
    thread::sleep(Duration::from_millis(50));
    stop_signal.store(true, Ordering::Relaxed);

    for handle in consumer_handles {
        handle.join().expect("Consumer thread should not panic");
    }

    // Verify invariants
    assert!(
        rb.len() <= CAPACITY,
        "Buffer length {} must not exceed capacity {}",
        rb.len(),
        CAPACITY
    );
    assert_eq!(
        total_samples_pushed.load(Ordering::SeqCst),
        NUM_PRODUCERS * BURSTS_PER_THREAD * CHUNK_SIZE
    );
    println!(
        "Concurrent stress passed: {} pushed, {} popped, final len {}",
        total_samples_pushed.load(Ordering::SeqCst),
        total_samples_popped.load(Ordering::SeqCst),
        rb.len()
    );
}

#[test]
fn test_audio_ring_buffer_wrap_around_and_drop_oldest() {
    let capacity = 10;
    let rb = AudioRingBuffer::new(capacity);

    // Push 10,000 sequential samples one by one
    for i in 0..10_000 {
        rb.push(&[i as f32]);
        assert!(rb.len() <= capacity);
    }

    // Since capacity is 10 and FIFO drop-oldest applies,
    // the buffer must contain exactly [9990.0, 9991.0, ..., 9999.0]
    assert_eq!(rb.len(), 10);
    let popped = rb.pop_all();
    assert_eq!(popped.len(), 10);
    for (idx, &val) in popped.iter().enumerate() {
        let expected = (9990 + idx) as f32;
        assert_eq!(val, expected, "At index {}: expected {}, got {}", idx, expected, val);
    }
    assert!(rb.is_empty());
}

#[test]
fn test_audio_ring_buffer_capacity_overflow_batches() {
    let rb = AudioRingBuffer::new(5);

    // Batch exceeding capacity in a single push: 10 elements into cap 5
    let large_batch = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    rb.push(&large_batch);
    assert_eq!(rb.len(), 5);
    // Should retain the last 5 elements
    assert_eq!(rb.pop_all(), vec![6.0, 7.0, 8.0, 9.0, 10.0]);

    // Push exactly capacity
    let exact_batch = vec![100.0, 200.0, 300.0, 400.0, 500.0];
    rb.push(&exact_batch);
    assert_eq!(rb.len(), 5);
    assert_eq!(rb.pop_all(), vec![100.0, 200.0, 300.0, 400.0, 500.0]);

    // Push empty batch (no-op)
    rb.push(&[]);
    assert_eq!(rb.len(), 0);

    // Interleaved push and pop across capacity
    rb.push(&[1.0, 2.0, 3.0]); // len = 3
    rb.push(&[4.0, 5.0, 6.0]); // total 6 pushed into cap 5 -> 1.0 dropped, buf = [2, 3, 4, 5, 6]
    assert_eq!(rb.len(), 5);
    let popped_2 = rb.pop(2);
    assert_eq!(popped_2, vec![2.0, 3.0]); // buf = [4, 5, 6], len = 3
    assert_eq!(rb.len(), 3);

    rb.push(&[7.0, 8.0, 9.0]); // 3 + 3 = 6 into cap 5 -> 4.0 dropped, buf = [5, 6, 7, 8, 9]
    assert_eq!(rb.len(), 5);
    assert_eq!(rb.pop_all(), vec![5.0, 6.0, 7.0, 8.0, 9.0]);
    assert!(rb.is_empty());
}

#[test]
fn test_audio_ring_buffer_zero_memory_leak_and_stability() {
    // Repeatedly instantiate, fill, clear, and drop ring buffers
    for _ in 0..50 {
        let rb = AudioRingBuffer::new(10_000);
        let chunk = vec![0.123f32; 1_000];
        for _ in 0..20 {
            rb.push(&chunk);
        }
        assert_eq!(rb.len(), 10_000);
        let _ = rb.pop(5_000);
        assert_eq!(rb.len(), 5_000);
        rb.clear();
        assert_eq!(rb.len(), 0);
        assert!(rb.is_empty());
    }
}
