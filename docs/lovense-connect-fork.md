# Intiface Central: retained Lovense Connect support

This is an unofficial maintenance fork of Intiface Central 3.2.2+46. It updates the original Flutter app and Rust engine integration. Games keep using Intiface's existing Buttplug server; no game-specific plugin is introduced.

## iPhone + Lush 4 + Windows

1. Connect the Lush 4 in **Lovense Connect** on the iPhone. Keep the app open and the toy connected.
2. Put the iPhone and Windows computer on the same local Wi-Fi. Allow **Local Network** access for Lovense Connect in iOS Settings. Guest networks that isolate devices will prevent a connection.
3. In Intiface Central, stop the server before changing Settings. Enable **Lovense Connect Service**.
4. Open **Lovense Connect phone address**. Enter the phone's local HTTP address and the HTTP port reported by Connect, for example `http://192.168.1.50:20010`. The example port is not a guaranteed default. Use the app's actual HTTP port, not its HTTPS or WebSocket port.
5. Start Intiface's server and scan for devices. The Lush should appear in the existing Devices page. Connect your game to Intiface as usual (the default server address remains `ws://127.0.0.1:12345`).

Leaving the address blank keeps the original automatic discovery through `https://api.lovense.com/api/lan/getToys`. A manual address bypasses that cloud discovery and retries the phone on later scans after interruptions. Update the saved address if your phone's Wi-Fi address changes.

If the phone cannot be reached, check its current address, local-network permission, firewall, VPN, and Wi-Fi isolation. A read-only check from the Windows computer is `curl.exe http://PHONE-IP:HTTP-PORT/GetToys`. It should return JSON with code 200 and a connected toy. This fork still needs Lovense Connect to provide that local API; if an iOS update removes the API entirely, this desktop change cannot restore the missing phone service.

## Changes

- Retains and patches the existing Lovense Connect communication manager rather than replacing Intiface's UI or Buttplug protocol implementation.
- Adds a persisted manual local HTTP phone address and clears it when automatic discovery is selected.
- Handles string status/code/battery fields, null batteries, nested or JSON-encoded toy lists, and the names `Lush`, `Lush 4`, and `Lush4`.
- Handles malformed discovery replies without panicking, retries manual hosts, bounds request timeouts, and reports failed HTTP/API command acknowledgements.
- Polls connected-device status every five seconds and exits the polling task after disconnection or device disposal.
- Removes Lovense Connect deprecation dialogs in this fork; the unrelated USB dongle deprecation notice is preserved.
- Keeps the existing generated Rust/Dart FFI ABI and the published engine's device-control profiles. Manual settings are written to `lovense-connect.json` beside the existing user-device configuration and read before reserving engine startup.

## Build the Windows app

Windows builds require Windows, Visual Studio 2022 with **Desktop development with C++**, Flutter 3.47.7, and Rust 1.99.0. The locked dependencies require Dart 3.13, which this Flutter release includes. The Flutter build compiles and bundles the Rust library through the existing Cargokit integration.

```powershell
rustup toolchain install 1.99.0 --profile minimal
rustup override set 1.99.0
flutter config --enable-windows-desktop
flutter pub get --enforce-lockfile
flutter test
cargo test --locked --manifest-path rust/Cargo.toml --lib -p rust_lib_intiface_central -p buttplug_server_hwmgr_lovense_connect -j 4
flutter build windows --release
```

Alternatively, run `./scripts/build-windows-lovense.ps1` from Windows PowerShell after installing the prerequisites. It stops on failed checks and verifies required runtime files before creating the ZIP.

Use the **whole** `build/windows/x64/runner/Release` directory. `intiface_central.exe` requires the bundled Flutter data, Flutter DLL, plugin DLLs, and Rust library. Do not replace only the uploaded launcher. Close the original app first to avoid the same settings files or server port being used by two instances.

The [GitHub Actions workflow](https://github.com/Tambone/lovense/actions/workflows/windows-lovense.yml), **Build Intiface Central with Lovense Connect**, performs the checks on Windows and uploads a ZIP of the entire application directory. Open the latest successful run and download **Intiface-Central-Lovense-Windows-x64** under Artifacts. Extract that ZIP, then extract the application ZIP inside it and run `intiface_central.exe`. The output is unsigned; no publisher signing credentials are requested or embedded.

## Validation status

Validation is recorded in [VALIDATION.md](../VALIDATION.md). The Windows build, Flutter and native tests, runtime-file checks, and packaged-app startup check passed. Mock-server tests validate discovery, responses, command acknowledgement, retries, and stop requests. The remaining check is connectivity to your physical iPhone and Lush 4; mock tests cannot establish whether your phone still exposes the local API.
