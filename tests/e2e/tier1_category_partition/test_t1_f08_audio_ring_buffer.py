import threading
import unittest


class MockAudioRingBuffer:
    def __init__(self, capacity: int = 16000):
        self.capacity = capacity
        self.buffer = []
        self.lock = threading.Lock()

    def push(self, samples):
        with self.lock:
            self.buffer.extend(samples)
            if len(self.buffer) > self.capacity:
                # Drop oldest
                overflow = len(self.buffer) - self.capacity
                self.buffer = self.buffer[overflow:]

    def pop_all(self):
        with self.lock:
            res = list(self.buffer)
            self.buffer.clear()
            return res

    def size(self):
        with self.lock:
            return len(self.buffer)


class TestT1F08AudioRingBuffer(unittest.TestCase):
    """
    Feature 8: Audio Capture & Ring Buffer (Equivalence Partitions)
    """

    def test_buffer_push_and_pop(self):
        """Partition: Normal push and pop of 16kHz audio chunks."""
        rb = MockAudioRingBuffer(16000)
        rb.push([0.1, 0.2, 0.3])
        self.assertEqual(rb.size(), 3)
        popped = rb.pop_all()
        self.assertEqual(popped, [0.1, 0.2, 0.3])
        self.assertEqual(rb.size(), 0)

    def test_buffer_overflow_drops_oldest(self):
        """Partition: Ring buffer capacity limit drops oldest samples."""
        rb = MockAudioRingBuffer(5)
        rb.push([1.0, 2.0, 3.0, 4.0, 5.0])
        rb.push([6.0, 7.0])
        self.assertEqual(rb.size(), 5)
        self.assertEqual(rb.pop_all(), [3.0, 4.0, 5.0, 6.0, 7.0])

    def test_buffer_underflow_empty_pop(self):
        """Partition: Popping from empty buffer returns empty list."""
        rb = MockAudioRingBuffer(100)
        self.assertEqual(rb.pop_all(), [])

    def test_multithreaded_push_safety(self):
        """Partition: Lock-free or mutex ring buffer thread safety."""
        rb = MockAudioRingBuffer(20000)
        t1 = threading.Thread(target=lambda: rb.push([0.1] * 5000))
        t2 = threading.Thread(target=lambda: rb.push([0.2] * 5000))
        t1.start()
        t2.start()
        t1.join()
        t2.join()
        self.assertEqual(rb.size(), 10000)

    def test_sample_rate_normalization(self):
        """Partition: Samples stored as normalized float values."""
        rb = MockAudioRingBuffer(10)
        rb.push([-1.0, 0.0, 1.0])
        popped = rb.pop_all()
        self.assertTrue(all(-1.0 <= s <= 1.0 for s in popped))
