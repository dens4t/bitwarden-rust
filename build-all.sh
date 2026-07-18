#!/bin/bash
# Build script for bitwarden-rs
# Requires: Rust, optional cross-compilers for targets other than native

set -euo pipefail

PROJECT="bitwarden-rs"
TARGET_DIR="target/release"

echo "🔧 Bitwarden-rs Builder"
echo "======================"
echo ""

# Native build (default)
echo "📦 Building for native target..."
cargo build --release
echo "   ✅ Built: $TARGET_DIR/$PROJECT"
echo "   Size: $(du -sh $TARGET_DIR/$PROJECT | cut -f1)"
echo ""

# Check for cross-compilation arguments
if [ $# -eq 0 ]; then
    echo "💡 Tip: To build for other targets, specify them as arguments:"
    echo "   ./build-all.sh aarch64-unknown-linux-gnu x86_64-pc-windows-gnu"
    echo ""
    echo "✅ Native build complete!"
    exit 0
fi

# Cross-compile for specified targets
for target in "$@"; do
    echo "📦 Building for $target..."
    
    # Set up linker based on target
    case "$target" in
        aarch64-unknown-linux-gnu)
            export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="aarch64-linux-gnu-gcc"
            ;;
        aarch64-unknown-linux-musl)
            export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="aarch64-linux-gnu-gcc"
            ;;
        x86_64-pc-windows-gnu)
            export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="x86_64-w64-mingw32-gcc"
            ;;
        x86_64-unknown-linux-musl)
            export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER="musl-gcc"
            ;;
    esac
    
    if cargo build --release --target "$target" 2>/dev/null; then
        echo "   ✅ Built for $target"
        echo "   Size: $(du -sh target/$target/release/$PROJECT* 2>/dev/null | cut -f1)"
    else
        echo "   ❌ Failed to build for $target"
        echo "   Make sure you have the cross-compilation tools installed:"
        echo "   - Linux ARM64: apt install gcc-aarch64-linux-gnu"
        echo "   - Windows: apt install mingw-w64"
        echo "   - Musl: apt install musl-tools"
    fi
    echo ""
done

echo "✅ All builds complete!"
