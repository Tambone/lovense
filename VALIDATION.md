# Validation of the Lovense Connect maintenance changes

This record applies to the source fork based on Intiface Central commit `5413bf0e23d22423c626b6bc63bd7673745a48c6`.

## Passed in the Linux cloud workspace

- Full native library compile: `cargo check --locked --manifest-path rust/Cargo.toml -j 4`.
- Native tests: `cargo test --locked --manifest-path rust/Cargo.toml --lib -p rust_lib_intiface_central -p buttplug_server_hwmgr_lovense_connect -j 4`.
- **26 tests passed, 0 failed, 0 ignored**: 18 app/native engine tests and 8 Lovense Connect tests.
- The Lovense tests use real local HTTP requests against a mock phone. Coverage includes discovery and retry, malformed/iPhone JSON responses, Lush 4 name normalization, initial battery reading, encoded toy IDs, vibration and zero-speed stop through the hardware adapter, failed HTTP/API acknowledgements, and disappearance/disconnection notification.
- The full Buttplug test uses the existing server, its actual Lush device configuration and protocol handler, and v3 game messages. It performs handshake, scanning, device discovery, 50% vibration, Stop All, and Stop Scanning. The phone receives `Vibrate1?v=10&t=lush-id` followed by `Vibrate1?v=0&t=lush-id`.
- Verified that the existing generated Dart/Rust FFI files, pubspec.yaml, and pubspec.lock are unchanged. No custom game protocol/server was substituted.

The native checks used Rust 1.99.0. Local Debian development libraries and CMake were extracted under `/workspace/toolchains/sysroot` from APT downloads verified against signed Debian package indexes. Tool activation is retained at `/workspace/toolchains/activate.sh`.

## Unrun / outstanding

- Flutter analysis, new Dart/widget tests, and the existing Flutter suites are **unrun**. The proxy rejected access to `storage.googleapis.com` (HTTP 403 while opening a CONNECT tunnel), which prevents downloading the Flutter/Dart SDK. Required network additions are saved in the environment draft; saving a draft alone does not apply them to the running machine.
- The Windows release build and Windows startup are **unrun**. Flutter cannot build its Windows app on this Linux host. A Windows build workflow and local PowerShell build script are included, but no Windows binary is claimed or supplied.
- Connection to the user's actual iPhone, Lovense Connect version, Lush 4, and game is **unverified**. The mock tests do not establish that the iPhone still exposes its local HTTP API.

The cloud network draft adds `storage.googleapis.com` for Flutter SDK artifacts and `api.lovense.com` for optional cloud discovery. Existing package-manager presets remain preserved.
