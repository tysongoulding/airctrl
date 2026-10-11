use std::collections::VecDeque;
use std::sync::Mutex;

/// Thread-safe circular audio ring buffer for real-time streaming audio capture.
///
/// Features:
/// - Thread-safe concurrent access via Mutex.
/// - Fixed capacity.
/// - FIFO drop-oldest overflow policy when newly pushed samples exceed capacity.
pub struct AudioRingBuffer {
    capacity: usize,
    buffer: Mutex<VecDeque<f32>>,
}

impl AudioRingBuffer {
    /// Creates a new circular audio ring buffer with the specified sample capacity.
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.max(1);
        Self {
            capacity: cap,
            buffer: Mutex::new(VecDeque::with_capacity(cap.min(65536))),
        }
    }

    /// Pushes samples into the ring buffer.
    ///
    /// If newly pushed samples exceed available capacity, the oldest samples
    /// are dropped in FIFO order to accommodate the newest samples.
    pub fn push(&self, samples: &[f32]) {
        if samples.is_empty() {
            return;
        }

        let mut buf = self.buffer.lock().unwrap();

        // If the batch itself exceeds capacity, retain only the most recent `capacity` samples
        if samples.len() >= self.capacity {
            buf.clear();
            let start = samples.len() - self.capacity;
            buf.extend(samples[start..].iter().copied());
            return;
        }

        let overflow = (buf.len() + samples.len()).saturating_sub(self.capacity);
        for _ in 0..overflow {
            buf.pop_front();
        }

        buf.extend(samples.iter().copied());
    }

    /// Pops and returns all samples currently stored in the buffer, clearing it.
    pub fn pop_all(&self) -> Vec<f32> {
        let mut buf = self.buffer.lock().unwrap();
        let res: Vec<f32> = buf.iter().copied().collect();
        buf.clear();
        res
    }

    /// Pops up to `count` oldest samples from the buffer.
    pub fn pop(&self, count: usize) -> Vec<f32> {
        let mut buf = self.buffer.lock().unwrap();
        let take_count = count.min(buf.len());
        let mut res = Vec::with_capacity(take_count);
        for _ in 0..take_count {
            if let Some(s) = buf.pop_front() {
                res.push(s);
            }
        }
        res
    }

    /// Peeks up to `count` oldest samples without consuming them.
    pub fn peek(&self, count: usize) -> Vec<f32> {
        let buf = self.buffer.lock().unwrap();
        buf.iter().take(count).copied().collect()
    }

    /// Returns the current number of samples stored in the buffer.
    pub fn len(&self) -> usize {
        let buf = self.buffer.lock().unwrap();
        buf.len()
    }

    /// Returns the maximum sample capacity of the buffer.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns true if the buffer currently contains zero samples.
    pub fn is_empty(&self) -> bool {
        let buf = self.buffer.lock().unwrap();
        buf.is_empty()
    }

    /// Clears all samples currently held in the buffer.
    pub fn clear(&self) {
        let mut buf = self.buffer.lock().unwrap();
        buf.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_push_pop_all() {
        let rb = AudioRingBuffer::new(10);
        assert!(rb.is_empty());
        assert_eq!(rb.capacity(), 10);

        rb.push(&[1.0, 2.0, 3.0]);
        assert_eq!(rb.len(), 3);
        assert!(!rb.is_empty());

        let popped = rb.pop_all();
        assert_eq!(popped, vec![1.0, 2.0, 3.0]);
        assert!(rb.is_empty());
    }

    #[test]
    fn test_ring_buffer_overflow_drops_oldest() {
        let rb = AudioRingBuffer::new(5);
        rb.push(&[1.0, 2.0, 3.0, 4.0]);
        rb.push(&[5.0, 6.0]); // 6 samples into cap 5 -> 1.0 dropped

        assert_eq!(rb.len(), 5);
        let popped = rb.pop_all();
        assert_eq!(popped, vec![2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn test_ring_buffer_large_batch_exceeding_capacity() {
        let rb = AudioRingBuffer::new(3);
        rb.push(&[10.0, 20.0, 30.0, 40.0, 50.0]);
        assert_eq!(rb.len(), 3);
        assert_eq!(rb.pop_all(), vec![30.0, 40.0, 50.0]);
    }

    #[test]
    fn test_ring_buffer_peek_and_pop() {
        let rb = AudioRingBuffer::new(10);
        rb.push(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(rb.peek(2), vec![1.0, 2.0]);
        assert_eq!(rb.len(), 4);

        let popped = rb.pop(2);
        assert_eq!(popped, vec![1.0, 2.0]);
        assert_eq!(rb.len(), 2);
        assert_eq!(rb.pop_all(), vec![3.0, 4.0]);
    }
}
