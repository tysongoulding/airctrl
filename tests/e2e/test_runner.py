#!/usr/bin/env python3
"""
test_runner.py - airctrl Comprehensive E2E Test Suite Runner

Runs and aggregates test results across all 4 tiers:
- Tier 1: Category-Partition (>= 130 tests across 26 features)
- Tier 2: Boundary Value Analysis (>= 130 tests across 26 features)
- Tier 3: Pairwise Combinatorial (>= 26 tests)
- Tier 4: Real-World Workload Scenarios (>= 13 tests)

Enforces threshold verification: Total >= 299 tests.
"""

import os
import sys
import time
import unittest


def run_suite():
    print("=" * 80)
    print("  airctrl E2E Automated Verification Test Runner")
    print("=" * 80)

    start_time = time.perf_counter()
    loader = unittest.TestLoader()

    base_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.dirname(os.path.dirname(base_dir))

    # Discover per tier with repo_root as top_level_dir
    t1_dir = os.path.join(base_dir, "tier1_category_partition")
    t2_dir = os.path.join(base_dir, "tier2_boundary_value_analysis")
    t3_dir = os.path.join(base_dir, "tier3_pairwise_combinatorial")
    t4_dir = os.path.join(base_dir, "tier4_real_world_workloads")

    suite_t1 = loader.discover(t1_dir, pattern="test_*.py", top_level_dir=repo_root)
    suite_t2 = loader.discover(t2_dir, pattern="test_*.py", top_level_dir=repo_root)
    suite_t3 = loader.discover(t3_dir, pattern="test_*.py", top_level_dir=repo_root)
    suite_t4 = loader.discover(t4_dir, pattern="test_*.py", top_level_dir=repo_root)

    count_t1 = suite_t1.countTestCases()
    count_t2 = suite_t2.countTestCases()
    count_t3 = suite_t3.countTestCases()
    count_t4 = suite_t4.countTestCases()
    total_count = count_t1 + count_t2 + count_t3 + count_t4

    print(f"[*] Discovered Tests:")
    print(f"    - Tier 1 (Category-Partition):        {count_t1:3d} tests (Threshold: >= 130)")
    print(f"    - Tier 2 (Boundary Value Analysis):   {count_t2:3d} tests (Threshold: >= 130)")
    print(f"    - Tier 3 (Pairwise Combinatorial):    {count_t3:3d} tests (Threshold: >=  26)")
    print(f"    - Tier 4 (Real-World Scenarios):      {count_t4:3d} tests (Threshold: >=  13)")
    print(f"    ------------------------------------------------")
    print(f"    - TOTAL DISCOVERED:                   {total_count:3d} tests (Threshold: >= 299)")
    print("=" * 80)

    if count_t1 < 130 or count_t2 < 130 or count_t3 < 26 or count_t4 < 13 or total_count < 299:
        print("[!] ERROR: Coverage threshold check failed!")
        sys.exit(1)

    combined_suite = unittest.TestSuite([suite_t1, suite_t2, suite_t3, suite_t4])
    runner = unittest.TextTestRunner(verbosity=1)
    result = runner.run(combined_suite)

    elapsed = time.perf_counter() - start_time

    print("=" * 80)
    print("  VERIFICATION SUMMARY")
    print("=" * 80)
    print(f"  Tests Executed:   {result.testsRun}")
    print(f"  Failures:         {len(result.failures)}")
    print(f"  Errors:           {len(result.errors)}")
    print(f"  Execution Time:   {elapsed:.3f} seconds")

    if result.wasSuccessful():
        print("\n  STATUS: 100% PASS - ALL QUALITY GATES & THRESHOLDS SATISFIED")
        print("=" * 80)
        sys.exit(0)
    else:
        print("\n  STATUS: FAILED - TEST DEFECTS FOUND")
        print("=" * 80)
        sys.exit(1)


if __name__ == "__main__":
    run_suite()
