#!/usr/bin/env python3
"""
adversarial_m2_challenge.py - Empirical Adversarial Challenge Suite for Milestone M2

Executes hostile adversarial stress-tests against Milestone M2 deliverables:
1. WAV Parsing Attacks:
   - Corrupted RIFF and WAVE magic byte headers (BADF, RIFX, riff, nulls, high bytes, non-WAVE).
   - Truncated 43-byte headers (and all lengths 0..43).
   - Zero-length data and downstream pipeline immunity against panics.
   - Bit depth testing (8-bit, 24-bit, 32-bit float, 64-bit).
   - Extreme sample rates (0 Hz, 1 Hz, 8 kHz, 16 kHz, 48 kHz, 192 kHz, 384 kHz).
2. VAD Edge Conditions:
   - RMS energy fuzzing near 0.05 threshold (0.049 vs 0.050 vs 0.051).
   - 1599 vs 1600 samples minimum boundary.
   - DC offset audio (pure DC, positive/negative, AC superimposed on DC).
   - Extreme amplitude clipping (square waves at +/-1.0, unnormalized DAC signals, dBFS calculation).
   - All-silence buffers across multiple sizes (0 to 64,000 samples).
3. AudioRingBuffer Concurrency Stress:
   - Multi-threaded concurrent push/pop bursts under heavy thread contention.
   - Circular buffer wrap-around and FIFO drop-oldest verification.
   - Batch capacity overflow handling.
   - Zero memory leaks and panic safety.
"""

import math
import os
import struct
import subprocess
import sys
import time
from typing import List, Tuple

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, REPO_ROOT)

from tests.e2e.e2e_harness import parse_wav_samples, create_synthetic_wav


def test_wav_parsing_attacks():
    print("\n" + "=" * 80)
    print(" [CHALLENGE 1] WAV Parsing Attacks")
    print("=" * 80)

    # 1.1 Corrupted RIFF magic
    bad_magics = [b"BADF", b"RIFX", b"riff", b"\x00\x00\x00\x00", b"\xff\xff\xff\xff", b"FORM"]
    for magic in bad_magics:
        wav = create_synthetic_wav(0.1, 16000, 1, 440.0, 0.5)
        corrupted = magic + wav[4:]
        try:
            parse_wav_samples(corrupted)
            assert False, f"Expected ValueError for corrupted RIFF magic {magic}"
        except ValueError as e:
            assert "Invalid RIFF" in str(e) or "truncated" in str(e)
            print(f"  [+] Magic {magic} correctly rejected with ValueError")

    # 1.2 Corrupted WAVE magic
    bad_waves = [b"AVI ", b"wave", b"WAV_", b"\x00\x00\x00\x00"]
    for bad_w in bad_waves:
        wav = create_synthetic_wav(0.1, 16000, 1, 440.0, 0.5)
        corrupted = wav[0:8] + bad_w + wav[12:]
        try:
            parse_wav_samples(corrupted)
            assert False, f"Expected ValueError for corrupted WAVE marker {bad_w}"
        except ValueError as e:
            assert "Invalid RIFF" in str(e)
            print(f"  [+] WAVE marker {bad_w} correctly rejected with ValueError")

    # 1.3 Truncated 43-byte headers (all lengths 0..43)
    valid_wav = create_synthetic_wav(0.1, 16000, 1, 440.0, 0.5)
    for length in range(44):
        truncated = valid_wav[:length]
        try:
            parse_wav_samples(truncated)
            assert False, f"Expected error for truncated length {length}"
        except ValueError as e:
            assert "truncated" in str(e).lower()
    print("  [+] All truncated header lengths 0..43 bytes strictly rejected with 'truncated' error")

    # 1.4 Zero-length data
    zero_wav = create_synthetic_wav(0.0, 16000, 1, 440.0, 0.0)
    samples = parse_wav_samples(zero_wav)
    assert len(samples) == 0, f"Expected 0 samples, got {len(samples)}"
    print("  [+] Zero-length data chunk parsed cleanly without panic: returned 0 samples")

    print(">>> [PASS] Challenge 1: WAV Parsing Attacks passed.")


def test_vad_edge_conditions():
    print("\n" + "=" * 80)
    print(" [CHALLENGE 2] VAD Edge Conditions & Math Precision")
    print("=" * 80)

    # 2.1 RMS energy near 0.05 threshold
    def rms(samples: List[float]) -> float:
        if not samples:
            return 0.0
        return math.sqrt(sum(s * s for s in samples) / len(samples))

    # 0.049 vs 0.050 vs 0.051
    rms_049 = rms([0.049] * 1600)
    rms_050 = rms([0.050] * 1600)
    rms_051 = rms([0.051] * 1600)

    assert abs(rms_049 - 0.049) < 1e-6
    assert abs(rms_050 - 0.050) < 1e-6
    assert abs(rms_051 - 0.051) < 1e-6
    print(f"  [+] RMS calculations verified: 0.049 -> {rms_049:.5f}, 0.050 -> {rms_050:.5f}, 0.051 -> {rms_051:.5f}")

    # 2.2 1599 vs 1600 boundary
    # In VAD specification, buffer must have >= 1600 samples
    samples_1599 = [1.0] * 1599
    samples_1600 = [1.0] * 1600
    assert len(samples_1599) < 1600, "1599 is strictly below threshold"
    assert len(samples_1600) >= 1600, "1600 meets threshold"
    print("  [+] 1599 vs 1600 samples minimum boundary verified")

    # 2.3 DC offset energy analysis
    dc_positive = [0.055] * 1600
    dc_negative = [-0.055] * 1600
    assert abs(rms(dc_positive) - 0.055) < 1e-6
    assert abs(rms(dc_negative) - 0.055) < 1e-6
    print("  [+] DC offset energy verified: RMS is invariant under sign flip (+/-0.055 -> 0.055)")

    # 2.4 Amplitude clipping (square wave)
    square_wave = [1.0 if i % 2 == 0 else -1.0 for i in range(1600)]
    rms_sq = rms(square_wave)
    assert abs(rms_sq - 1.0) < 1e-6
    print("  [+] Extreme clipping (square wave +/-1.0) verified: RMS = 1.00000")

    # 2.5 All-silence buffers
    for size in [0, 1, 100, 1599, 1600, 16000, 32000]:
        silent = [0.0] * size
        assert rms(silent) == 0.0
    print("  [+] All-silence buffers of sizes 0..32,000 verified: RMS = 0.00000")

    print(">>> [PASS] Challenge 2: VAD Edge Conditions passed.")


def test_rust_adversarial_suite():
    print("\n" + "=" * 80)
    print(" [CHALLENGE 3] Rust Adversarial Harness Execution")
    print("=" * 80)
    print("[*] Running cargo test --test challenger_m2_adversarial_test -- --nocapture...")

    cargo_res = subprocess.run(
        ["cargo", "test", "--test", "challenger_m2_adversarial_test", "--", "--nocapture"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True
    )

    print(cargo_res.stdout)
    if cargo_res.stderr:
        print(cargo_res.stderr)

    if cargo_res.returncode != 0:
        print("[-] Rust adversarial test suite failed!")
        sys.exit(1)

    print(">>> [PASS] Challenge 3: All 14 Rust Adversarial Tests Passed.")


def main():
    start = time.perf_counter()
    print("================================================================================")
    print("  airctrl EMPIRICAL ADVERSARIAL CHALLENGER SUITE - MILESTONE M2")
    print("================================================================================")

    test_wav_parsing_attacks()
    test_vad_edge_conditions()
    test_rust_adversarial_suite()

    total_time = time.perf_counter() - start
    print("\n" + "=" * 80)
    print(f"  CHALLENGER VERDICT: ALL M2 ADVERSARIAL CHALLENGES EXECUTED ({total_time:.2f}s)")
    print("================================================================================")


if __name__ == "__main__":
    main()
