# Validation of the Lovense Connect maintenance changes

This record applies to the source fork based on Intiface Central commit `5413bf0e23d22423c626b6bc63bd7673745a48c6`.

## Passed in the Linux cloud workspace

- Full native library compile: `cargo check --locked --manifest-path rust/Cargo.toml -j 4`.
- Native tests: `cargo test --locked --manifest-path rust/Cargo.toml --lib -p rust_lib_intiface_central -p buttplug_server_hwmgr_lovense_connect -j 4`.
- **26 tests passed, 0 failed, 0 ignored**: 18 app/native engine tests and 8 Lovense Connect tests.
- Flutter analysis of the changed app code and new tests: **no issues found**.
- Full Flutter unit, widget, bloc, and documentation screenshot suite: **111 tests passed** with `flutter test --no-pub --reporter expanded`.
- Locked Flutter dependencies resolve successfully with Flutter **3.47.7**. The source lockfile requires Dart 3.13; Flutter 3.44.9 contains Dart 3.12 and cannot resolve it.
- The Lovense tests use real local HTTP requests against a mock phone. Coverage includes discovery and retry, malformed/iPhone JSON responses, Lush 4 name normalization, initial battery reading, encoded toy IDs, vibration and zero-speed stop through the hardware adapter, failed HTTP/API acknowledgements, and disappearance/disconnection notification.
- The full Buttplug test uses the existing server, its actual Lush device configuration and protocol handler, and v3 game messages. It performs handshake, scanning, device discovery, 50% vibration, Stop All, and Stop Scanning. The phone receives `Vibrate1?v=10&t=lush-id` followed by `Vibrate1?v=0&t=lush-id`.
- Verified that the existing generated Dart/Rust FFI files, pubspec.yaml, and pubspec.lock are unchanged. No custom game protocol/server was substituted.

The native checks used Rust 1.99.0. Local Debian development libraries and CMake were extracted under `/workspace/toolchains/sysroot` from APT downloads verified against signed Debian package indexes. Tool activation is retained at `/workspace/toolchains/activate.sh`.

## Passed on Windows

- [Release build 38165983297](https://github.com/Tambone/lovense/actions/runs/38165983297) succeeded for app source commit `ba323e0ae2e728a7fb9a3651013f47667153af4e`. Later commits add package verification, documentation, and restored upstream reference files; the app code is unchanged.
- Locked dependency resolution, Flutter analysis, and the full Flutter test step passed on the Windows runner.
- Native tests passed on Windows: **8 Lovense Connect tests and 18 app/native tests**, with 0 failed or ignored.
- `flutter build windows --release` succeeded and packaged the complete application folder.
- [Package verification 38167648964](https://github.com/Tambone/lovense/actions/runs/38167648964) downloaded that artifact on a fresh Windows runner. It checked the executable, Flutter and Rust DLLs, ICU data, and Flutter asset manifest. The app showed its Intiface Central 3.2.2 window and remained running for the startup check.
- [Download the Windows artifact](https://github.com/Tambone/lovense/actions/runs/38165983297/artifacts/11712742628). Extract the artifact ZIP, then the application ZIP inside it. The inner application ZIP has SHA256 `a24ab3bcda905e9f68489fe1088e0ba5e1be83ab3fac8a59686d6c972c74e8be`.

## Outstanding physical-device check

- Connection to the user's actual iPhone, Lovense Connect version, Lush 4, and game is **unverified**. The mock tests do not establish that the iPhone still exposes its local HTTP API.

The cloud workspace's network policy blocks GitHub's artifact storage host, so the Windows package was verified directly on the Windows runner and is delivered through GitHub. No local Windows ZIP is claimed in `/workspace/artifacts`.

Flutter and Rust toolchains are installed under `/workspace/toolchains`; source `/workspace/toolchains/activate.sh` to use them. Flutter tool configuration and analyzer caches are directed to writable workspace paths, and CLI analytics are suppressed for this cloud environment.
