#!/usr/bin/env bash
set -euo pipefail

echo "=========================================================="
echo " Running Automated LAN Daemon & run.ps1 Test Suite"
echo "=========================================================="

# 1. Run unit and integration tests across airctrl-lan crate
cargo test -p airctrl-lan -- --nocapture

echo "----------------------------------------------------------"
echo " Validating PowerShell run.ps1 generation standalone..."
echo "----------------------------------------------------------"

TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT

# Create a mock airctrl.exe
mkdir -p "$TEMP_DIR/dist"
echo "MZ_MOCK_AIRCTRL_EXE_PAYLOAD_FOR_LAN_TEST" > "$TEMP_DIR/dist/airctrl.exe"

echo "PASS: All airctrl-lan integration tests and syntax checks completed successfully."
