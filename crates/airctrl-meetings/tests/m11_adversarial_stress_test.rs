use airctrl_core::types::camera::{FrameBuffer, FrameFormat, PrivacyMuteState};
use airctrl_meetings::camera::{VirtualCameraSplitter, ZeroLockVideoRingBuffer};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Stress test 1: ZeroLockVideoRingBuffer with 1 writer pushing 10,000 frames at high speed
/// and 2 concurrent readers consuming frames concurrently.
/// Asserts:
/// - Zero deadlocks (completes within reasonable time)
/// - Monotonic sequence numbers (every frame received has id > last_seen)
/// - Zero data corruption (frame payload matches stamped frame_id)
#[test]
fn test_ring_buffer_10k_frames_concurrency_stress() {
    let ring = Arc::new(ZeroLockVideoRingBuffer::new());
    let stop_signal = Arc::new(AtomicBool::new(false));
    let total_frames: u64 = 10_000;

    let ring_writer = ring.clone();
    let writer = thread::spawn(move || {
        for frame_id in 1..=total_frames {
            let mut frame = FrameBuffer::allocate(64, 64, FrameFormat::Rgb8);
            // Stamp frame_id into the first 8 bytes of data to detect corruption
            let id_bytes = frame_id.to_le_bytes();
            frame.data[..8].copy_from_slice(&id_bytes);

            ring_writer.publish_frame(Arc::new(frame), frame_id);
        }
    });

    let make_reader = |reader_id: usize| {
        let ring_reader = ring.clone();
        let stop = stop_signal.clone();
        thread::spawn(move || {
            let mut last_seen: u64 = 0;
            let mut read_count: u64 = 0;
            let mut non_monotonic_violations = 0;
            let mut corruption_errors = 0;

            while !stop.load(Ordering::Acquire) || last_seen < total_frames {
                if let Some((frame, id)) = ring_reader.get_latest_frame(last_seen) {
                    if id <= last_seen {
                        non_monotonic_violations += 1;
                        eprintln!(
                            "[Reader {}] NON-MONOTONIC VIOLATION: received id={}, last_seen={}",
                            reader_id, id, last_seen
                        );
                    }
                    // Validate data integrity
                    let mut read_bytes = [0u8; 8];
                    read_bytes.copy_from_slice(&frame.data[..8]);
                    let stamped_id = u64::from_le_bytes(read_bytes);
                    if stamped_id != id {
                        corruption_errors += 1;
                        eprintln!(
                            "[Reader {}] DATA CORRUPTION: frame payload stamped with {} but frame_id={}",
                            reader_id, stamped_id, id
                        );
                    }

                    last_seen = last_seen.max(id);
                    read_count += 1;
                }
                thread::yield_now();
            }
            (read_count, non_monotonic_violations, corruption_errors)
        })
    };

    let reader1 = make_reader(1);
    let reader2 = make_reader(2);

    writer.join().expect("Writer panicked");
    // Give readers a brief moment to catch up to latest frame
    thread::sleep(Duration::from_millis(50));
    stop_signal.store(true, Ordering::Release);

    let (reads1, non_mono1, corrupt1) = reader1.join().expect("Reader 1 panicked");
    let (reads2, non_mono2, corrupt2) = reader2.join().expect("Reader 2 panicked");

    eprintln!(
        "Reader 1 read {} frames (violations: non-monotonic={}, corruption={})",
        reads1, non_mono1, corrupt1
    );
    eprintln!(
        "Reader 2 read {} frames (violations: non-monotonic={}, corruption={})",
        reads2, non_mono2, corrupt2
    );

    assert_eq!(corrupt1, 0, "Reader 1 observed frame data corruption!");
    assert_eq!(corrupt2, 0, "Reader 2 observed frame data corruption!");
    assert_eq!(
        non_mono1, 0,
        "Reader 1 observed non-monotonic sequence numbers (id <= last_seen)!"
    );
    assert_eq!(
        non_mono2, 0,
        "Reader 2 observed non-monotonic sequence numbers (id <= last_seen)!"
    );
}

/// Stress test 2: Rapid privacy toggling between Active, PrivacySlate, and DualMute
/// Asserting invariant:
/// - In PrivacySlate: Consumer 2 MUST receive privacy slate frame, Consumer 1 MUST strictly receive raw physical camera frame.
/// - In DualMute: Consumer 1 MUST be None, Consumer 2 MUST receive privacy slate.
/// - In Active: Consumer 1 and Consumer 2 MUST receive raw frame.
#[test]
fn test_splitter_privacy_mode_rapid_transitions_and_isolation() {
    let splitter = Arc::new(VirtualCameraSplitter::with_default_config());
    let iterations = 1000;

    for i in 1..=iterations {
        match i % 3 {
            0 => {
                // Active mode
                splitter.set_dual_mute(false);
                splitter.set_privacy_mute(false);

                let out = splitter.feed_camera_frame(i).expect("feed failed");
                assert_eq!(
                    out.consumer_1_mediapipe,
                    Some(format!("raw_frame_{}", i)),
                    "Consumer 1 should receive raw frame in Active mode"
                );
                assert_eq!(
                    out.consumer_2_meeting,
                    format!("outbound_frame_{}", i),
                    "Consumer 2 should receive outbound frame in Active mode"
                );
                assert!(out.consumer_1_frame.is_some());
                assert!(out.consumer_2_frame.is_some());
                assert!(!out.privacy_muted);
                assert!(!out.dual_muted);
            }
            1 => {
                // PrivacySlate mode
                splitter.set_dual_mute(false);
                splitter.set_privacy_mute(true);

                let out = splitter.feed_camera_frame(i).expect("feed failed");
                assert_eq!(
                    out.consumer_1_mediapipe,
                    Some(format!("raw_frame_{}", i)),
                    "Consumer 1 (MediaPipe) MUST continue receiving raw frame in PrivacySlate mode!"
                );
                assert_eq!(
                    out.consumer_2_meeting, "privacy_slate_frame",
                    "Consumer 2 MUST receive privacy slate frame in PrivacySlate mode!"
                );
                assert!(
                    out.consumer_1_frame.is_some(),
                    "Consumer 1 frame must be Some in PrivacySlate mode"
                );
                assert!(
                    out.consumer_2_frame.is_some(),
                    "Consumer 2 frame must be Some in PrivacySlate mode"
                );
                assert!(out.privacy_muted);
                assert!(!out.dual_muted);
            }
            _ => {
                // DualMute mode
                splitter.set_dual_mute(true);

                let out = splitter.feed_camera_frame(i).expect("feed failed");
                assert_eq!(
                    out.consumer_1_mediapipe, None,
                    "Consumer 1 MUST be None in DualMute mode"
                );
                assert_eq!(
                    out.consumer_2_meeting, "privacy_slate_frame",
                    "Consumer 2 MUST receive privacy slate frame in DualMute mode"
                );
                assert!(
                    out.consumer_1_frame.is_none(),
                    "Consumer 1 frame must be None in DualMute mode"
                );
                assert!(
                    out.consumer_2_frame.is_some(),
                    "Consumer 2 frame must be Some in DualMute mode"
                );
                assert!(out.privacy_muted);
                assert!(out.dual_muted);
            }
        }
    }
}

/// Stress test 3: Direct transition from DualMute to PrivacySlate
/// In DualMute mode, user wishes to restore local MediaPipe gestures while keeping
/// meeting video feed muted to privacy slate.
#[test]
fn test_splitter_transition_from_dual_mute_to_privacy_slate() {
    let splitter = VirtualCameraSplitter::with_default_config();

    // 1. Enter DualMute
    splitter.set_dual_mute(true);
    assert!(splitter.is_dual_muted());
    assert!(splitter.is_privacy_muted());
    assert!(!splitter.is_consumer_1_active());

    // 2. Transition from DualMute to PrivacySlate:
    // User wants outbound to stay privacy muted, but re-enable local gesture tracking.
    splitter.set_dual_mute(false);
    splitter.set_privacy_mute(true);

    let status = splitter.status();
    eprintln!(
        "Status after DualMute -> PrivacySlate: privacy_mode={:?}, privacy_muted={}, dual_muted={}, c1_active={}",
        status.privacy_mode, status.privacy_muted, status.dual_muted, splitter.is_consumer_1_active()
    );

    assert_eq!(
        status.privacy_mode,
        PrivacyMuteState::PrivacySlate,
        "Privacy mode should be PrivacySlate after unsetting dual mute and setting privacy mute"
    );
    assert!(
        splitter.is_privacy_muted(),
        "Should remain privacy muted in PrivacySlate mode"
    );
    assert!(
        !splitter.is_dual_muted(),
        "Should not be dual muted in PrivacySlate mode"
    );
    assert!(
        splitter.is_consumer_1_active(),
        "Consumer 1 (MediaPipe) MUST be active in PrivacySlate mode"
    );

    let out = splitter.feed_camera_frame(999).expect("feed failed");
    assert_eq!(
        out.consumer_1_mediapipe,
        Some("raw_frame_999".to_string()),
        "Consumer 1 must receive raw frames"
    );
    assert_eq!(
        out.consumer_2_meeting, "privacy_slate_frame",
        "Consumer 2 must receive privacy slate frame"
    );
}

/// Stress test 4: Extreme reader contention with 8 concurrent readers
/// Hammering ZeroLockVideoRingBuffer while 1 writer pushes 20,000 frames.
#[test]
fn test_ring_buffer_extreme_reader_contention_8_readers() {
    let ring = Arc::new(ZeroLockVideoRingBuffer::new());
    let stop_signal = Arc::new(AtomicBool::new(false));
    let total_frames: u64 = 20_000;

    let ring_writer = ring.clone();
    let writer = thread::spawn(move || {
        for frame_id in 1..=total_frames {
            let mut frame = FrameBuffer::allocate(32, 32, FrameFormat::Rgb8);
            let id_bytes = frame_id.to_le_bytes();
            frame.data[..8].copy_from_slice(&id_bytes);
            ring_writer.publish_frame(Arc::new(frame), frame_id);
        }
    });

    let mut reader_handles = Vec::new();
    for reader_id in 0..8 {
        let ring_reader = ring.clone();
        let stop = stop_signal.clone();
        let handle = thread::spawn(move || {
            let mut last_seen: u64 = 0;
            let mut read_count: u64 = 0;
            let mut violations = 0;
            let mut corruptions = 0;

            while !stop.load(Ordering::Acquire) || last_seen < total_frames {
                if let Some((frame, id)) = ring_reader.get_latest_frame(last_seen) {
                    if id <= last_seen {
                        violations += 1;
                    }
                    let mut read_bytes = [0u8; 8];
                    read_bytes.copy_from_slice(&frame.data[..8]);
                    let stamped_id = u64::from_le_bytes(read_bytes);
                    if stamped_id != id {
                        corruptions += 1;
                    }
                    last_seen = last_seen.max(id);
                    read_count += 1;
                }
                thread::yield_now();
            }
            (reader_id, read_count, violations, corruptions)
        });
        reader_handles.push(handle);
    }

    writer.join().expect("Writer panicked");
    thread::sleep(Duration::from_millis(50));
    stop_signal.store(true, Ordering::Release);

    for handle in reader_handles {
        let (id, count, violations, corruptions) = handle.join().expect("Reader panicked");
        eprintln!(
            "[8-Reader Stress] Reader {} completed {} reads: violations={}, corruptions={}",
            id, count, violations, corruptions
        );
        assert_eq!(violations, 0, "Reader {} observed non-monotonic sequence!", id);
        assert_eq!(corruptions, 0, "Reader {} observed data corruption!", id);
    }
}

/// Stress test 5: Asymmetric / Slow Laggard Reader vs Ultra-Fast Writer
/// Simulates MediaPipe processing taking 1-2ms per frame while writer pushes frames at full speed.
/// Rings wrap around thousands of times between slow reader polls.
#[test]
fn test_ring_buffer_slow_laggard_reader_ring_wraparound() {
    let ring = Arc::new(ZeroLockVideoRingBuffer::new());
    let stop_signal = Arc::new(AtomicBool::new(false));
    let total_frames: u64 = 5_000;

    let ring_writer = ring.clone();
    let writer = thread::spawn(move || {
        for frame_id in 1..=total_frames {
            let mut frame = FrameBuffer::allocate(32, 32, FrameFormat::Rgb8);
            let id_bytes = frame_id.to_le_bytes();
            frame.data[..8].copy_from_slice(&id_bytes);
            ring_writer.publish_frame(Arc::new(frame), frame_id);
            if frame_id % 50 == 0 {
                thread::yield_now();
            }
        }
    });

    // Slow reader sleeps 1ms every read
    let ring_reader = ring.clone();
    let stop = stop_signal.clone();
    let slow_reader = thread::spawn(move || {
        let mut last_seen: u64 = 0;
        let mut read_count: u64 = 0;
        let mut violations = 0;
        let mut corruptions = 0;

        while !stop.load(Ordering::Acquire) || last_seen < total_frames {
            if let Some((frame, id)) = ring_reader.get_latest_frame(last_seen) {
                if id <= last_seen {
                    violations += 1;
                }
                let mut read_bytes = [0u8; 8];
                read_bytes.copy_from_slice(&frame.data[..8]);
                let stamped_id = u64::from_le_bytes(read_bytes);
                if stamped_id != id {
                    corruptions += 1;
                }
                last_seen = last_seen.max(id);
                read_count += 1;
            }
            thread::sleep(Duration::from_millis(1));
        }
        (read_count, violations, corruptions)
    });

    writer.join().expect("Writer panicked");
    thread::sleep(Duration::from_millis(50));
    stop_signal.store(true, Ordering::Release);

    let (reads, violations, corruptions) = slow_reader.join().expect("Slow reader panicked");
    eprintln!(
        "[Slow Reader Stress] Laggard reader read {} frames across {} total: violations={}, corruptions={}",
        reads, total_frames, violations, corruptions
    );
    assert!(reads > 0, "Slow reader should have read at least some frames");
    assert_eq!(violations, 0, "Slow reader observed non-monotonic sequence during ring wraps!");
    assert_eq!(corruptions, 0, "Slow reader observed corrupted frame data during ring wraps!");
}

/// Stress test 6: Boundary queries on ZeroLockVideoRingBuffer
#[test]
fn test_ring_buffer_boundary_queries() {
    let ring = ZeroLockVideoRingBuffer::new();

    // 1. Empty buffer queries
    assert!(ring.get_latest_frame(0).is_none());
    assert!(ring.get_latest_frame(100).is_none());
    assert!(ring.get_latest_frame(u64::MAX).is_none());
    assert_eq!(ring.total_published(), 0);

    // 2. Publish 1 frame
    let frame1 = Arc::new(FrameBuffer::allocate(16, 16, FrameFormat::Rgb8));
    ring.publish_frame(frame1, 1);
    assert_eq!(ring.total_published(), 1);

    // Queries on 1 frame
    assert!(ring.get_latest_frame(1).is_none(), "last_seen == frame_id should return None");
    assert!(ring.get_latest_frame(2).is_none(), "last_seen > frame_id should return None");
    assert!(ring.get_latest_frame(u64::MAX).is_none());

    let (f, id) = ring.get_latest_frame(0).expect("Should read frame 1");
    assert_eq!(id, 1);
    assert_eq!(f.width, 16);

    // 3. Overflowing last_seen_id
    assert!(ring.get_latest_frame(u64::MAX - 1).is_none());
}

/// Stress test 7: Multi-threaded Splitter streaming under concurrent privacy mutations
#[test]
fn test_splitter_concurrent_frame_streaming_and_privacy_mutation() {
    let splitter = Arc::new(VirtualCameraSplitter::with_default_config());
    let stop = Arc::new(AtomicBool::new(false));
    let frame_count = 3_000;

    // Mutator thread toggling privacy modes rapidly
    let splitter_mutator = splitter.clone();
    let stop_mutator = stop.clone();
    let mutator = thread::spawn(move || {
        let mut step = 0;
        while !stop_mutator.load(Ordering::Acquire) {
            match step % 3 {
                0 => splitter_mutator.set_privacy_mode(PrivacyMuteState::Active),
                1 => splitter_mutator.set_privacy_mode(PrivacyMuteState::PrivacySlate),
                _ => splitter_mutator.set_privacy_mode(PrivacyMuteState::DualMute),
            }
            step += 1;
            if step % 20 == 0 {
                thread::yield_now();
            }
        }
    });

    // Producer / Consumer thread feeding camera frames
    let splitter_worker = splitter.clone();
    let worker = thread::spawn(move || {
        for i in 1..=frame_count {
            let out = splitter_worker.feed_camera_frame(i).expect("feed must never fail");
            assert_eq!(out.frame_id, i);

            // Invariant verification under concurrency
            if out.dual_muted {
                assert!(out.privacy_muted, "DualMute implies privacy_muted");
                assert_eq!(out.consumer_1_mediapipe, None, "Consumer 1 must be None when dual muted");
                assert_eq!(out.consumer_2_meeting, "privacy_slate_frame", "Consumer 2 must be privacy slate");
            } else if out.privacy_muted {
                assert_eq!(out.consumer_1_mediapipe, Some(format!("raw_frame_{}", i)), "Consumer 1 must receive raw frames in PrivacySlate");
                assert_eq!(out.consumer_2_meeting, "privacy_slate_frame", "Consumer 2 must be privacy slate");
            } else {
                assert_eq!(out.consumer_1_mediapipe, Some(format!("raw_frame_{}", i)));
                assert_eq!(out.consumer_2_meeting, format!("outbound_frame_{}", i));
            }
        }
    });

    worker.join().expect("Worker panicked");
    stop.store(true, Ordering::Release);
    mutator.join().expect("Mutator panicked");

    assert_eq!(splitter.frames_fanned_out(), frame_count);
}

