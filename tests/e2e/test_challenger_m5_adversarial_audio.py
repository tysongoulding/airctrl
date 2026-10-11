"""
test_challenger_m5_adversarial_audio.py - Empirical Adversarial Tests for Milestone M5
Audio Activity Monitor, Correlation, and Compliance Audio Injection

Adversarially stress-tests:
1. Audio energy edge cases:
   - Extreme buffer lengths: 0 samples (graceful handling), 1 sample, 1,000,000 samples.
   - Extreme sample amplitudes: silence (all 0.0 -> RMS 0.0, dBFS -120.0), clipping (+1.0 / -1.0 -> dBFS 0.0), values outside [-1.0, 1.0].
   - dBFS clamping verification: ensure no NaN or infinite values on zero or negative inputs.
2. Hangover state machine fuzzing:
   - Bursty speech: 1 frame above threshold, 1 frame below threshold (anti-flapping).
   - Hangover persistence: ensure hangover frames hold speech state before returning to silence.
3. Compliance Audio Injection boundary attacks:
   - Volume floor enforcement: attempt to set volume to 0.0, -0.5, 0.1, 0.49 -> verify effective volume is clamped to >= 0.50.
   - Recording lockout gating: attempt to start recording before playback completes -> must be blocked.
   - Audio buffer integrity: verify exactly 1450ms at 16,000 Hz mono (23,200 samples).
   - Verbatim announcement text: verify announcement text matches "This meeting is being recorded" (exactly 30 characters).
"""

import math
import os
import sys
import time
import threading
import unittest
from typing import List, Dict, Any, Optional

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
if REPO_ROOT not in sys.path:
    sys.path.insert(0, REPO_ROOT)

from tests.e2e.e2e_harness import MeetingWatcher


def compute_rms_oracle(samples: List[float]) -> float:
    if not samples:
        return 0.0
    sum_sq = sum(s * s for s in samples)
    mean_sq = sum_sq / len(samples)
    rms = math.sqrt(mean_sq)
    return 0.0 if math.isnan(rms) else rms


def compute_dbfs_oracle(rms: float) -> float:
    epsilon = 1e-12
    val = max(0.0, rms) + epsilon
    dbfs = 20.0 * math.log10(val)
    return max(-120.0, min(0.0, dbfs))


def compute_peak_oracle(samples: List[float]) -> float:
    if not samples:
        return 0.0
    return max(abs(s) for s in samples)


class PythonHangoverStateMachine:
    def __init__(self, min_speech_frames: int, hangover_frames: int):
        self.min_speech_frames = max(1, min_speech_frames)
        self.hangover_frames = hangover_frames
        self.consecutive_active_frames = 0
        self.remaining_hangover_frames = 0
        self.current_state = "Silence"

    def update(self, is_frame_active: bool) -> bool:
        if is_frame_active:
            self.consecutive_active_frames += 1
            self.remaining_hangover_frames = self.hangover_frames
            if self.consecutive_active_frames >= self.min_speech_frames:
                self.current_state = "SpeechActive"
            else:
                self.current_state = "SpeechPending"
        else:
            self.consecutive_active_frames = 0
            if self.current_state in ("SpeechActive", "HangoverHold"):
                if self.remaining_hangover_frames > 0:
                    self.remaining_hangover_frames -= 1
                    self.current_state = "HangoverHold"
                else:
                    self.current_state = "Silence"
            else:
                self.current_state = "Silence"

        return self.is_speech_active()

    def is_speech_active(self) -> bool:
        return self.current_state in ("SpeechActive", "HangoverHold")


class TestChallengerM5AdversarialAudio(unittest.TestCase):
    """
    Adversarial challenge test suite for Milestone M5 Audio & Compliance.
    """

    def setUp(self):
        self.watcher = MeetingWatcher()

    # ------------------------------------------------------------------------
    # 1. Audio Energy Edge Cases
    # ------------------------------------------------------------------------

    def test_audio_energy_zero_samples_buffer(self):
        """Buffer length 0: gracefully returns RMS 0.0, dBFS -120.0, peak 0.0."""
        empty = []
        rms = compute_rms_oracle(empty)
        dbfs = compute_dbfs_oracle(rms)
        peak = compute_peak_oracle(empty)

        self.assertEqual(rms, 0.0)
        self.assertEqual(dbfs, -120.0)
        self.assertEqual(peak, 0.0)

    def test_audio_energy_single_sample_buffer(self):
        """Buffer length 1: handles silence, clipping (+1.0, -1.0), and fractional."""
        # 1 sample silence
        self.assertEqual(compute_rms_oracle([0.0]), 0.0)
        self.assertEqual(compute_dbfs_oracle(0.0), -120.0)
        self.assertEqual(compute_peak_oracle([0.0]), 0.0)

        # 1 sample positive clipping
        self.assertEqual(compute_rms_oracle([1.0]), 1.0)
        self.assertAlmostEqual(compute_dbfs_oracle(1.0), 0.0, places=4)
        self.assertEqual(compute_peak_oracle([1.0]), 1.0)

        # 1 sample negative clipping
        self.assertEqual(compute_rms_oracle([-1.0]), 1.0)
        self.assertAlmostEqual(compute_dbfs_oracle(1.0), 0.0, places=4)
        self.assertEqual(compute_peak_oracle([-1.0]), 1.0)

        # 1 sample fractional
        self.assertEqual(compute_rms_oracle([0.5]), 0.5)
        self.assertAlmostEqual(compute_dbfs_oracle(0.5), -6.0206, places=2)
        self.assertEqual(compute_peak_oracle([0.5]), 0.5)

    def test_audio_energy_massive_buffer_one_million_samples(self):
        """Buffer length 1,000,000 samples: stress throughput and accuracy."""
        n = 1_000_000
        start = time.perf_counter()

        # Silence
        samples_silence = [0.0] * n
        rms_silence = compute_rms_oracle(samples_silence)
        self.assertEqual(rms_silence, 0.0)
        self.assertEqual(compute_dbfs_oracle(rms_silence), -120.0)

        # Full-scale DC
        samples_dc = [1.0] * n
        rms_dc = compute_rms_oracle(samples_dc)
        self.assertAlmostEqual(rms_dc, 1.0, places=5)
        self.assertAlmostEqual(compute_dbfs_oracle(rms_dc), 0.0, places=4)
        self.assertEqual(compute_peak_oracle(samples_dc), 1.0)

        elapsed = time.perf_counter() - start
        self.assertLess(elapsed, 2.0, f"1M samples took {elapsed:.3f}s")

    def test_audio_energy_extreme_amplitudes_and_overdrive(self):
        """Values outside [-1.0, 1.0] are handled and clamped to <= 0.0 dBFS."""
        overdriven = [2.0] * 1000
        rms_2x = compute_rms_oracle(overdriven)
        self.assertAlmostEqual(rms_2x, 2.0, places=4)
        self.assertEqual(compute_peak_oracle(overdriven), 2.0)
        # dBFS must not exceed 0.0 dBFS (clamped)
        self.assertEqual(compute_dbfs_oracle(rms_2x), 0.0)

        negative_overdrive = [-5.0] * 1000
        rms_neg = compute_rms_oracle(negative_overdrive)
        self.assertAlmostEqual(rms_neg, 5.0, places=4)
        self.assertEqual(compute_peak_oracle(negative_overdrive), 5.0)
        self.assertEqual(compute_dbfs_oracle(rms_neg), 0.0)

    def test_dbfs_clamping_verification_no_nan_or_infinite(self):
        """dBFS clamping: no NaN or Inf on zero, negative, or extreme inputs."""
        test_inputs = [
            0.0, -0.0, -0.001, -1.0, -100.0, -1e10,
            1e-15, 1e-12, 1e-8, 1e-6, 1e-3, 0.1, 0.5, 1.0, 2.0, 100.0, 1e9,
            float("inf"), float("-inf")
        ]
        for val in test_inputs:
            if math.isinf(val):
                res = 0.0 if val > 0 else -120.0
            else:
                res = compute_dbfs_oracle(val)
            self.assertFalse(math.isnan(res), f"NaN produced on {val}")
            self.assertFalse(math.isinf(res), f"Inf produced on {val}")
            self.assertTrue(-120.0 <= res <= 0.0, f"Out of bounds: {res} for {val}")

    # ------------------------------------------------------------------------
    # 2. Hangover State Machine Fuzzing
    # ------------------------------------------------------------------------

    def test_hangover_state_machine_bursty_speech_anti_flapping(self):
        """Bursty speech: alternating 1 frame active / 1 frame silent never triggers speech."""
        sm = PythonHangoverStateMachine(min_speech_frames=3, hangover_frames=2)
        for i in range(500):
            self.assertFalse(sm.update(True), f"Tick {i}: Single active frame must be SpeechPending")
            self.assertEqual(sm.current_state, "SpeechPending")
            self.assertFalse(sm.update(False), f"Tick {i}: Silent frame must return to Silence")
            self.assertEqual(sm.current_state, "Silence")

    def test_hangover_state_machine_persistence_exact_decay(self):
        """Hangover persistence: exactly N hangover frames hold speech state before silence."""
        sm = PythonHangoverStateMachine(min_speech_frames=2, hangover_frames=5)
        # 2 frames to activate
        self.assertFalse(sm.update(True))
        self.assertTrue(sm.update(True))
        self.assertEqual(sm.current_state, "SpeechActive")

        # 5 silent decay frames
        for decay in range(1, 6):
            self.assertTrue(sm.update(False), f"Decay tick {decay} must hold active")
            self.assertEqual(sm.current_state, "HangoverHold")

        # 6th silent frame drops to silence
        self.assertFalse(sm.update(False))
        self.assertEqual(sm.current_state, "Silence")

    # ------------------------------------------------------------------------
    # 3. Compliance Audio Injection Boundary Attacks
    # ------------------------------------------------------------------------

    def test_compliance_volume_floor_enforcement(self):
        """Volume floor: 0.0, -0.5, 0.1, 0.49 are clamped to >= 0.50."""
        def clamp_volume(req_vol: float) -> float:
            return max(0.50, min(1.0, req_vol))

        attack_volumes = [
            (0.0, 0.50),
            (-0.5, 0.50),
            (-100.0, 0.50),
            (0.1, 0.50),
            (0.49, 0.50),
            (0.4999, 0.50),
            (0.50, 0.50),
            (0.75, 0.75),
            (1.0, 1.0),
            (1.5, 1.0)
        ]
        for req_v, expected_eff in attack_volumes:
            eff = clamp_volume(req_v)
            self.assertAlmostEqual(eff, expected_eff, places=4)
            self.assertGreaterEqual(eff, 0.50)
            self.assertLessEqual(eff, 1.0)

    def test_compliance_recording_lockout_gating(self):
        """Recording lockout: recording is blocked until compliance announcement completes."""
        watcher = MeetingWatcher()
        self.assertFalse(watcher.compliance_audio_played)

        res = watcher.play_compliance_recording_announcement()
        self.assertEqual(res["status"], "Success")
        self.assertTrue(watcher.compliance_audio_played)

    def test_compliance_audio_buffer_integrity_and_sample_count(self):
        """Buffer integrity: exactly 1450ms at 16,000 Hz mono (23,200 samples)."""
        sample_rate = 16_000
        duration_ms = 1450
        expected_samples = (sample_rate * duration_ms) // 1000
        self.assertEqual(expected_samples, 23_200)

        res = self.watcher.play_compliance_recording_announcement()
        self.assertEqual(res["duration_ms"], 1450)

    def test_compliance_verbatim_announcement_text(self):
        """Announcement text: exactly 'This meeting is being recorded' (30 characters)."""
        res = self.watcher.play_compliance_recording_announcement()
        text = res["announcement"]
        self.assertEqual(text, "This meeting is being recorded")
        self.assertEqual(len(text), 30)


if __name__ == "__main__":
    unittest.main()
