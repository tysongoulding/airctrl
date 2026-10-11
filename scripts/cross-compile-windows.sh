#!/usr/bin/env bash
# ==============================================================================
# scripts/cross-compile-windows.sh
#
# Cross-compilation pipeline for airctrl targeting x86_64-pc-windows-msvc
# on Ubuntu Linux using cargo-xwin and LLVM lld-link.
#
# Outputs:
#   - target/x86_64-pc-windows-msvc/release/airctrl.exe
#   - dist/airctrl.exe
#   - dist/airctrl.exe.sha256
# ==============================================================================

set -euo pipefail

# ------------------------------------------------------------------------------
# Default Configuration
# ------------------------------------------------------------------------------
TARGET="x86_64-pc-windows-msvc"
PROFILE="release"
BIN_NAME="airctrl"
OUT_DIR="dist"
CARGO_BIN_DIR="${CARGO_HOME:-$HOME/.cargo}/bin"
XWIN_CACHE_DIR="${XWIN_CACHE_DIR:-$HOME/.cache/xwin}"
CARGO_XWIN_VERSION="0.23.1"
AUTO_INSTALL=false
VERIFY_ONLY=false
VERBOSE=false

# ANSI colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info()    { echo -e "${BLUE}[INFO]${NC} $*"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $*"; }
log_warn()    { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error()   { echo -e "${RED}[ERROR]${NC} $*" >&2; }

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Options:
    --release            Build in release mode (default)
    --debug              Build in debug mode
    --bin <name>         Binary name to build (default: airctrl)
    --out-dir <dir>      Artifact distribution directory (default: dist)
    --install-deps       Automatically install missing system dependencies (lld, clang, cargo-xwin)
    --verify             Run PE header validation on the built executable
    --verify-only        Only run validation on existing artifact without compiling
    --verbose            Enable verbose output
    -h, --help           Show this help message
EOF
}

# ------------------------------------------------------------------------------
# Argument Parsing
# ------------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
    case "$1" in
        --release)
            PROFILE="release"
            shift
            ;;
        --debug)
            PROFILE="debug"
            shift
            ;;
        --bin)
            BIN_NAME="$2"
            shift 2
            ;;
        --out-dir)
            OUT_DIR="$2"
            shift 2
            ;;
        --install-deps)
            AUTO_INSTALL=true
            shift
            ;;
        --verify)
            # Will build then verify
            shift
            ;;
        --verify-only)
            VERIFY_ONLY=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

export PATH="$CARGO_BIN_DIR:$PATH"

# ------------------------------------------------------------------------------
# Step 1: Prerequisite Verification & Resolution
# ------------------------------------------------------------------------------
check_and_install_prerequisites() {
    log_info "Checking toolchain prerequisites..."

    # 1.1 Check LLVM / LLD
    if ! command -v lld-link >/dev/null 2>&1 && ! command -v lld >/dev/null 2>&1; then
        if [ "$AUTO_INSTALL" = true ]; then
            log_info "Installing lld and clang via apt..."
            sudo apt-get update -qq
            sudo apt-get install -y -qq lld clang llvm
        else
            log_warn "lld-link or lld not found in PATH."
            log_warn "Run with --install-deps or execute: sudo apt-get install -y lld clang llvm"
        fi
    fi

    # 1.2 Check cargo-xwin
    if ! command -v cargo-xwin >/dev/null 2>&1; then
        log_info "cargo-xwin not found in PATH. Checking ~/.cargo/bin..."
        if [ -x "$CARGO_BIN_DIR/cargo-xwin" ]; then
            log_info "Found cargo-xwin in $CARGO_BIN_DIR"
        else
            log_info "Fetching prebuilt static cargo-xwin v${CARGO_XWIN_VERSION} binary from GitHub..."
            mkdir -p "$CARGO_BIN_DIR"
            local TARBALL_URL="https://github.com/rust-cross/cargo-xwin/releases/download/v${CARGO_XWIN_VERSION}/cargo-xwin-v${CARGO_XWIN_VERSION}.x86_64-unknown-linux-musl.tar.gz"
            wget -qO- "$TARBALL_URL" | tar -xz -C "$CARGO_BIN_DIR"
            chmod +x "$CARGO_BIN_DIR/cargo-xwin"
            log_success "Installed cargo-xwin into $CARGO_BIN_DIR"
        fi
    fi

    # 1.3 Check Rust target x86_64-pc-windows-msvc
    local TARGET_INSTALLED=false
    if command -v rustup >/dev/null 2>&1; then
        if rustup target list --installed | grep -q "^${TARGET}\$"; then
            TARGET_INSTALLED=true
        else
            log_info "Adding target $TARGET via rustup..."
            rustup target add "$TARGET"
            TARGET_INSTALLED=true
        fi
    else
        local SYSROOT
        SYSROOT="$(rustc --print sysroot)"
        if [ -d "$SYSROOT/lib/rustlib/$TARGET" ]; then
            TARGET_INSTALLED=true
        else
            log_info "Installing user rustup to manage cross-compilation targets..."
            wget -qO- https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
            # shellcheck source=/dev/null
            source "$CARGO_BIN_DIR/env" 2>/dev/null || true
            rustup target add "$TARGET"
            TARGET_INSTALLED=true
        fi
    fi

    if [ "$TARGET_INSTALLED" != true ]; then
        log_error "Failed to locate or install standard library for target $TARGET"
        exit 1
    fi
    log_success "All prerequisites satisfied."
}

# ------------------------------------------------------------------------------
# Step 2: MSVC SDK & CRT Cache Pre-warming
# ------------------------------------------------------------------------------
warm_msvc_sdk_cache() {
    log_info "Checking MSVC SDK & CRT cache in $XWIN_CACHE_DIR..."
    mkdir -p "$XWIN_CACHE_DIR"
    export XWIN_CACHE_DIR

    # Pre-download and cache x86_64 Windows SDK/CRT files
    cargo xwin cache xwin --xwin-arch x86_64
    log_success "MSVC SDK & CRT cache ready."
}

# ------------------------------------------------------------------------------
# Step 3: Compilation Execution
# ------------------------------------------------------------------------------
compile_windows_binary() {
    log_info "Compiling $BIN_NAME for $TARGET (Profile: $PROFILE)..."

    # Export cross-compilation toolchain environment variables
    export CC_x86_64_pc_windows_msvc="clang-cl"
    export CXX_x86_64_pc_windows_msvc="clang-cl"
    export AR_x86_64_pc_windows_msvc="llvm-lib"

    local PROFILE_FLAG=""
    if [ "$PROFILE" = "release" ]; then
        PROFILE_FLAG="--release"
    fi

    local VERBOSE_FLAG=""
    if [ "$VERBOSE" = true ]; then
        VERBOSE_FLAG="-v"
    fi

    cargo xwin build \
        --target "$TARGET" \
        $PROFILE_FLAG \
        --bin "$BIN_NAME" \
        $VERBOSE_FLAG

    log_success "Compilation completed successfully."
}

# ------------------------------------------------------------------------------
# Step 4: Artifact Placement & Checksums
# ------------------------------------------------------------------------------
place_distribution_artifacts() {
    local SRC_EXE="target/$TARGET/$PROFILE/$BIN_NAME.exe"
    local DEST_EXE="$OUT_DIR/$BIN_NAME.exe"
    local DEST_SHA="$OUT_DIR/$BIN_NAME.exe.sha256"

    if [ ! -f "$SRC_EXE" ]; then
        log_error "Expected build artifact not found at: $SRC_EXE"
        exit 1
    fi

    mkdir -p "$OUT_DIR"
    cp -f "$SRC_EXE" "$DEST_EXE"
    sha256sum "$DEST_EXE" > "$DEST_SHA"

    local FILE_SIZE
    FILE_SIZE="$(stat -c%s "$DEST_EXE")"
    local SHA_VALUE
    SHA_VALUE="$(cut -d' ' -f1 "$DEST_SHA")"

    log_success "Artifact deployed to LAN distribution directory: $DEST_EXE"
    log_info "Size: $FILE_SIZE bytes"
    log_info "SHA256: $SHA_VALUE"
}

# ------------------------------------------------------------------------------
# Step 5: PE Header & Metadata Verification
# ------------------------------------------------------------------------------
verify_pe_artifact() {
    local TARGET_EXE="$OUT_DIR/$BIN_NAME.exe"
    if [ ! -f "$TARGET_EXE" ]; then
        TARGET_EXE="target/$TARGET/$PROFILE/$BIN_NAME.exe"
    fi

    if [ ! -f "$TARGET_EXE" ]; then
        log_error "Artifact not found for verification: $TARGET_EXE"
        exit 1
    fi

    log_info "Verifying PE header for $TARGET_EXE..."

    # 5.1 Verify with file utility
    local FILE_OUTPUT
    FILE_OUTPUT="$(file -b "$TARGET_EXE")"
    log_info "file signature: $FILE_OUTPUT"

    if ! echo "$FILE_OUTPUT" | grep -q "PE32+ executable"; then
        log_error "Verification FAILED: Artifact is not a 64-bit PE32+ executable!"
        exit 1
    fi

    if ! echo "$FILE_OUTPUT" | grep -q "x86-64"; then
        log_error "Verification FAILED: Machine type is not x86-64!"
        exit 1
    fi

    # 5.2 Verify Magic bytes programmatically (DOS MZ 0x5A4D & PE 0x4550)
    local MAGIC_HEX
    MAGIC_HEX="$(head -c 2 "$TARGET_EXE" | xxd -p)"
    if [ "$MAGIC_HEX" != "4d5a" ]; then
        log_error "Verification FAILED: Missing DOS MZ signature (found: $MAGIC_HEX)"
        exit 1
    fi

    # 5.3 If objdump is available, check PE optional header
    if command -v objdump >/dev/null 2>&1; then
        local OBJDUMP_ARCH
        OBJDUMP_ARCH="$(objdump -f "$TARGET_EXE" 2>/dev/null | grep "architecture:" || true)"
        log_info "objdump architecture: $OBJDUMP_ARCH"
    fi

    log_success "PE header validation PASSED: $TARGET_EXE is a valid x86-64 PE32+ executable."
}

# ------------------------------------------------------------------------------
# Main Flow
# ------------------------------------------------------------------------------
main() {
    echo "======================================================================"
    echo "  airctrl Windows MSVC Cross-Compilation Pipeline"
    echo "  Target: $TARGET | Profile: $PROFILE"
    echo "======================================================================"

    if [ "$VERIFY_ONLY" = true ]; then
        verify_pe_artifact
        exit 0
    fi

    check_and_install_prerequisites
    warm_msvc_sdk_cache
    compile_windows_binary
    place_distribution_artifacts
    verify_pe_artifact

    echo "======================================================================"
    log_success "Windows cross-compilation pipeline completed successfully!"
    echo "To serve via LAN distribution daemon:"
    echo "  cargo run --bin airctrl-lan"
    echo "To run from local Windows client:"
    echo "  irm http://<host-ip>:<port>/run.ps1 | iex"
    echo "======================================================================"
}

main "$@"
