#!/usr/bin/env bash
set -e

# target platforms
TARGETS=(
    x86_64-unknown-linux-gnu
    aarch64-unknown-linux-gnu
    x86_64-unknown-freebsd
    aarch64-linux-android
    x86_64-linux-android
)

# Build scripts and proc macros run inside the container. Keep their artifacts
# separate because cross images can have incompatible host glibc versions.
BUILD_ROOT="${CARGO_TARGET_DIR:-target}/build"

# cross build
for target in "${TARGETS[@]}"; do
    echo "==> Building for $target..."
    if CARGO_TARGET_DIR="$BUILD_ROOT/$target" cross build --target "$target"; then
        echo "✅ Success: $target"
    else
        echo "❌ Failed: $target"
        exit 1
    fi
done

echo ""
echo "✅ All builds succeeded!"
