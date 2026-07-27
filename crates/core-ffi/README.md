# printers-core-ffi

uniffi bindings over `printers-core`, for a **native (non-Flutter) Kotlin or
Swift app** that wants to embed the renderer directly — no loopback agent
process on mobile, see `docs/ARCHITECTURE.md`.

Note: `packages/sdk_flutter` does **not** use this crate. uniffi's Dart
target is unofficial/experimental; `sdk_flutter` binds `core-rs` directly via
its own `flutter_rust_bridge` crate (`packages/sdk_flutter/rust`) instead —
see docs/ARCHITECTURE.md's "Why sdk_flutter doesn't use core-ffi". Both wrap
the same `printers-core` and stay in sync by construction.

## Building

This crate builds and typechecks on any platform with `cargo check -p
printers-core-ffi`. Producing the actual mobile artifacts needs
platform-specific toolchains not available in every dev environment:

### Android (`.so` via NDK)
```bash
cargo install cargo-ndk
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo ndk -o <your-native-kotlin-app>/app/src/main/jniLibs \
  -t arm64-v8a -t armeabi-v7a -t x86_64 build --release -p printers-core-ffi
uniffi-bindgen generate --library target/release/libprinters_core_ffi.so \
  --language kotlin --out-dir <your-native-kotlin-app>/app/src/main/kotlin
```

### iOS (`.a` / XCFramework)
**Requires a Mac with Xcode — cannot be built in this Linux environment.**
```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
cargo build --release -p printers-core-ffi --target aarch64-apple-ios
# ...repeat per target, then `xcodebuild -create-xcframework` to bundle,
# and `uniffi-bindgen generate --language swift` for the Swift bindings.
```

This crate and its Kotlin/Swift binding generation steps are scaffolded but
**not yet build-verified for mobile targets** in this repo — that needs to
happen on machines with the Android NDK / Xcode installed respectively. What
*is* verified here (see `crates/core-rs`) is the rendering logic itself,
which is what this crate re-exports.
