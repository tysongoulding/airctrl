import unittest
from tests.e2e.tier1_category_partition.test_t1_f08_audio_ring_buffer import MockAudioRingBuffer


class TestT2F08AudioRingBuffer(unittest.TestCase):
    """
    Feature 8: Audio Capture & Ring Buffer (Boundary Value Analysis)
    """

    def test_buffer_capacity_minimum_1(self):
        """BVA: Capacity 1 ring buffer boundary."""
        rb = MockAudioRingBuffer(capacity=1)
        rb.push([0.5])
        self.assertEqual(rb.size(), 1)
        rb.push([0.9])
        self.assertEqual(rb.size(), 1)
        self.assertEqual(rb.pop_all(), [0.9])

    def test_buffer_exact_capacity_fill(self):
        """BVA: Pushing exactly capacity items."""
        rb = MockAudioRingBuffer(capacity=100)
        rb.push([0.1] * 100)
        self.assertEqual(rb.size(), 100)
        self.assertEqual(len(rb.pop_all()), 100)

    def test_buffer_off_by_one_overflow(self):
        """BVA: Capacity + 1 samples drops oldest 1 sample."""
        rb = MockAudioRingBuffer(capacity=5)
        rb.push([1.0, 2.0, 3.0, 4.0, 5.0, 6.0])  # 6 items
        self.assertEqual(rb.size(), 5)
        self.assertEqual(rb.pop_all(), [2.0, 3.0, 4.0, 5.0, 6.0])

    def test_zero_samples_push(self):
        """BVA: Pushing empty list does not alter size."""
        rb = MockAudioRingBuffer(capacity=10)
        rb.push([])
        self.assertEqual(rb.size(), 0)

    def test_large_capacity_160000_samples(self):
        """BVA: 10-second 16kHz ring buffer capacity (160,000 samples)."""
        rb = MockAudioRingBuffer(capacity=160000)
        self.assertEqual(rb.capacity, 160000)
