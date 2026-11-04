#!/usr/bin/env bash
set -e

# Target arsitektur Android
TARGETS=("aarch64-linux-android" "armv7-linux-androideabi" "x86_64-linux-android")
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_DIR="$PROJECT_ROOT/telegram-core"
JNI_DIR="$PROJECT_ROOT/android-app/app/src/main/jniLibs"

echo "=== 1. Memeriksa Toolchain Rust & NDK ==="
if ! command -v cargo-ndk &> /dev/null; then
    echo "cargo-ndk belum terpasang. Jalankan: cargo install cargo-ndk"
fi

cd "$RUST_DIR"

echo "=== 2. Generate UniFFI Bindings untuk Kotlin ==="
cargo run --bin uniffi-bindgen generate src/telegram.udl \
    --language kotlin \
    --out-dir "$PROJECT_ROOT/android-app/app/src/main/java/com/telegram/core" || true

echo "=== 3. Build Rust Library untuk Android (.so) ==="
for TARGET in "${TARGETS[@]}"; do
    ABI=""
    case $TARGET in
        "aarch64-linux-android") ABI="arm64-v8a" ;;
        "armv7-linux-androideabi") ABI="armeabi-v7a" ;;
        "x86_64-linux-android") ABI="x86_64" ;;
    esac

    echo "Building untuk $TARGET ($ABI)..."
    cargo ndk -t $ABI -o "$JNI_DIR" build --release
done

echo "=== Selesai! File .so dan binding Kotlin siap di android-app ==="
