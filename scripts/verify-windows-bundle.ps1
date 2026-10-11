# Intended for a fresh Windows CI runner; it launches the packaged GUI briefly.
param([Parameter(Mandatory = $true)][string]$Archive)
$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') { throw 'Package startup verification requires Windows.' }
$bundle = Join-Path ([System.IO.Path]::GetTempPath()) ("intiface-package-" + [guid]::NewGuid())
Expand-Archive -LiteralPath $Archive -DestinationPath $bundle
foreach ($file in @('intiface_central.exe', 'flutter_windows.dll', 'rust_lib_intiface_central.dll', 'data/icudtl.dat', 'data/flutter_assets/AssetManifest.bin')) {
  if (-not (Test-Path (Join-Path $bundle $file) -PathType Leaf)) { throw "Incomplete app bundle: missing $file" }
}
$application = $null
try {
  $application = Start-Process -FilePath (Join-Path $bundle 'intiface_central.exe') -WorkingDirectory $bundle -PassThru
  $deadline = [DateTime]::UtcNow.AddSeconds(30)
  $ready = $false
  do {
    Start-Sleep -Seconds 1
    $application.Refresh()
    if ($application.HasExited) { throw "Packaged app exited during startup: $($application.ExitCode)" }
    if ($application.MainWindowHandle -ne 0 -and $application.MainWindowTitle -match '^Intiface.*Central 3\.2\.2') {
      $ready = $true
      break
    }
  } while ([DateTime]::UtcNow -lt $deadline)
  if (-not $ready) { throw 'The packaged app did not show its Flutter application window within 30 seconds.' }
  Start-Sleep -Seconds 5
  $application.Refresh()
  if ($application.HasExited) { throw "Packaged app exited after showing its window: $($application.ExitCode)" }
  $hash = (Get-FileHash -LiteralPath $Archive -Algorithm SHA256).Hash.ToLowerInvariant()
  Write-Output "Application window: $($application.MainWindowTitle)"
  Write-Output "ZIP SHA256: $hash"
  Write-Output "::notice title=Windows package verified::Required executable, DLLs, ICU data, and Flutter assets are present. The app showed its Intiface Central window and stayed running. ZIP SHA256: $hash"
} finally {
  if ($application -and -not $application.HasExited) { Stop-Process -Id $application.Id -Force }
}
