use airctrl_stt::AudioRingBuffer;
use std::sync::Arc;
use std::thread;

#[test]
fn test_ring_buffer_basic_lifecycle() {
    let rb = AudioRingBuffer::new(100);
    assert_eq!(rb.capacity(), 100);
    assert_eq!(rb.len(), 0);
    assert!(rb.is_empty());

    rb.push(&[0.1, 0.2, 0.3]);
    assert_eq!(rb.len(), 3);
    assert!(!rb.is_empty());

    let peeked = rb.peek(2);
    assert_eq!(peeked, vec![0.1, 0.2]);
    assert_eq!(rb.len(), 3); // non-destructive

    let popped = rb.pop(2);
    assert_eq!(popped, vec![0.1, 0.2]);
    assert_eq!(rb.len(), 1);

    let all = rb.pop_all();
    assert_eq!(all, vec![0.3]);
    assert!(rb.is_empty());
}

#[test]
fn test_ring_buffer_overflow_drop_oldest() {
    let rb = AudioRingBuffer::new(4);
    rb.push(&[1.0, 2.0, 3.0]);
    assert_eq!(rb.len(), 3);

    // Push 3 more into capacity 4: total 6 pushed, oldest 2 (1.0, 2.0) dropped
    rb.push(&[4.0, 5.0, 6.0]);
    assert_eq!(rb.len(), 4);
    assert_eq!(rb.pop_all(), vec![3.0, 4.0, 5.0, 6.0]);
}

#[test]
fn test_ring_buffer_large_batch_overflow() {
    let rb = AudioRingBuffer::new(3);
    // Push batch larger than capacity
    rb.push(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    assert_eq!(rb.len(), 3);
    assert_eq!(rb.pop_all(), vec![4.0, 5.0, 6.0]);
}

#[test]
fn test_ring_buffer_empty_pop_underflow() {
    let rb = AudioRingBuffer::new(10);
    assert!(rb.pop_all().is_empty());
    assert!(rb.pop(5).is_empty());
    assert!(rb.peek(5).is_empty());
}

#[test]
fn test_ring_buffer_concurrent_pushes() {
    let rb = Arc::new(AudioRingBuffer::new(10_000));
    let mut handles = Vec::new();

    for t in 0..8 {
        let rb_clone = Arc::clone(&rb);
        handles.push(thread::spawn(move || {
            let chunk = vec![t as f32; 100];
            for _ in 0..10 {
                rb_clone.push(&chunk);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    // 8 threads * 10 iterations * 100 samples = 8000 samples (< 10_000 cap)
    assert_eq!(rb.len(), 8000);
    let all = rb.pop_all();
    assert_eq!(all.len(), 8000);
    assert!(rb.is_empty());
}
