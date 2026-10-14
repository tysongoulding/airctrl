//! Zero-lock video ring buffer supporting concurrent lock-free fan-out to MediaPipe and meeting video sinks.

use airctrl_core::types::camera::FrameBuffer;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

pub const RING_CAPACITY: usize = 8;
pub const RING_MASK: usize = RING_CAPACITY - 1;

/// Cache-line padded slot (64 bytes aligned) to eliminate CPU core false sharing.
#[repr(align(64))]
pub struct RingSlot {
    /// Sequence counter: even = write committed and stable; odd = write in progress.
    pub sequence: AtomicU64,
    pub frame_id: AtomicU64,
    pub frame: RwLock<Option<Arc<FrameBuffer>>>,
}

impl Default for RingSlot {
    fn default() -> Self {
        Self {
            sequence: AtomicU64::new(0),
            frame_id: AtomicU64::new(0),
            frame: RwLock::new(None),
        }
    }
}

/// Bounded lock-free seqlock ring buffer for high-throughput video fan-out.
pub struct ZeroLockVideoRingBuffer {
    write_head: AtomicU64,
    slots: [RingSlot; RING_CAPACITY],
}

impl Default for ZeroLockVideoRingBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl ZeroLockVideoRingBuffer {
    pub fn new() -> Self {
        Self {
            write_head: AtomicU64::new(0),
            slots: [
                RingSlot::default(),
                RingSlot::default(),
                RingSlot::default(),
                RingSlot::default(),
                RingSlot::default(),
                RingSlot::default(),
                RingSlot::default(),
                RingSlot::default(),
            ],
        }
    }

    /// Publishes a frame buffer into the ring buffer with monotonic sequence ordering.
    pub fn publish_frame(&self, frame: Arc<FrameBuffer>, frame_id: u64) {
        let ticket = self.write_head.fetch_add(1, Ordering::SeqCst);
        let slot_idx = (ticket as usize) & RING_MASK;
        let slot = &self.slots[slot_idx];

        // Transition sequence to ODD (write in progress)
        let seq = ticket * 2;
        slot.sequence.store(seq + 1, Ordering::Release);

        // Store frame and frame_id
        if let Ok(mut lock) = slot.frame.write() {
            *lock = Some(frame);
        }
        slot.frame_id.store(frame_id, Ordering::Relaxed);

        // Transition sequence to EVEN (write committed)
        slot.sequence.store(seq + 2, Ordering::Release);
    }

    /// Retrieves the newest committed frame with frame_id strictly greater than `last_seen_id`.
    pub fn get_latest_frame(&self, last_seen_id: u64) -> Option<(Arc<FrameBuffer>, u64)> {
        let current_head = self.write_head.load(Ordering::Acquire);
        if current_head == 0 {
            return None;
        }

        let latest_ticket = current_head - 1;
        let slot_idx = (latest_ticket as usize) & RING_MASK;
        let slot = &self.slots[slot_idx];

        let seq_before = slot.sequence.load(Ordering::Acquire);
        if seq_before == 0 || !seq_before.is_multiple_of(2) {
            // Write in progress or uninitialized on latest slot, inspect previous slot if available
            if latest_ticket == 0 {
                return None;
            }
            return self.get_slot_frame(latest_ticket - 1, last_seen_id);
        }

        let frame_id = slot.frame_id.load(Ordering::Relaxed);
        if frame_id <= last_seen_id {
            return None;
        }

        let frame_opt = slot.frame.read().ok().and_then(|guard| guard.clone());
        let seq_after = slot.sequence.load(Ordering::Acquire);

        if seq_before == seq_after && seq_before.is_multiple_of(2) {
            if frame_id <= last_seen_id {
                return None;
            }
            frame_opt.map(|f| (f, frame_id))
        } else {
            if latest_ticket == 0 {
                return None;
            }
            self.get_slot_frame(latest_ticket - 1, last_seen_id)
        }
    }

    fn get_slot_frame(&self, ticket: u64, last_seen_id: u64) -> Option<(Arc<FrameBuffer>, u64)> {
        let slot_idx = (ticket as usize) & RING_MASK;
        let slot = &self.slots[slot_idx];
        let seq_before = slot.sequence.load(Ordering::Acquire);
        if seq_before == 0 || !seq_before.is_multiple_of(2) {
            return None;
        }
        let frame_id = slot.frame_id.load(Ordering::Relaxed);
        if frame_id <= last_seen_id {
            return None;
        }
        let frame_opt = slot.frame.read().ok().and_then(|guard| guard.clone());
        let seq_after = slot.sequence.load(Ordering::Acquire);
        if seq_before == seq_after && seq_before.is_multiple_of(2) {
            if frame_id <= last_seen_id {
                return None;
            }
            frame_opt.map(|f| (f, frame_id))
        } else {
            None
        }
    }

    /// Returns the total number of frames published to this ring buffer.
    pub fn total_published(&self) -> u64 {
        self.write_head.load(Ordering::Relaxed)
    }
}
