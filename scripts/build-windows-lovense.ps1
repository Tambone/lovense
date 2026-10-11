# Run in Windows PowerShell from the source directory or invoke by full path.
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
if ($env:OS -ne 'Windows_NT') { throw 'Flutter Windows builds require a Windows host.' }
foreach ($command in @('flutter', 'cargo', 'rustup')) {
  if (-not (Get-Command $command -ErrorAction SilentlyContinue)) { throw "Install $command before building. See docs/lovense-connect-fork.md." }
}
function Invoke-Checked { param([scriptblock]$Command)
  & $Command
  if ($LASTEXITCODE -ne 0) { throw "Command failed with exit code $LASTEXITCODE. Build stopped." }
}
Invoke-Checked { rustup toolchain install 1.99.0 --profile minimal }
Invoke-Checked { rustup override set 1.99.0 }
Invoke-Checked { flutter config --enable-windows-desktop }
Invoke-Checked { flutter pub get --enforce-lockfile }
Invoke-Checked { flutter analyze --no-fatal-infos lib/util/lovense_connect_settings.dart lib/widget/lovense_connect_address_dialog.dart lib/widget/engine_config_widget.dart lib/bloc/configuration/intiface_configuration_cubit.dart lib/intiface_central_app.dart test/util/lovense_connect_settings_test.dart test/widget/lovense_connect_address_dialog_test.dart }
Invoke-Checked { flutter test }
Invoke-Checked { cargo test --locked --manifest-path rust/Cargo.toml --lib -p rust_lib_intiface_central -p buttplug_server_hwmgr_lovense_connect -j 4 }
Invoke-Checked { flutter build windows --release }
$bundle = 'build/windows/x64/runner/Release'
foreach ($file in @('intiface_central.exe', 'flutter_windows.dll', 'rust_lib_intiface_central.dll', 'data/flutter_assets')) {
  if (-not (Test-Path (Join-Path $bundle $file))) { throw "Incomplete app bundle: missing $file" }
}
Copy-Item 'docs/lovense-connect-fork.md' (Join-Path $bundle 'LOVENSE-CONNECT-README.md')
$zip = 'Intiface-Central-Lovense-Windows-x64.zip'
if (Test-Path $zip) { throw "Move or remove the existing $zip before building another ZIP." }
Compress-Archive -Path "$bundle/*" -DestinationPath $zip
Write-Output "Built $zip. Extract the whole ZIP and run intiface_central.exe."
