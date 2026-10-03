[CmdletBinding()]
param(
    [string]$InstallerPath = (Join-Path $PSScriptRoot '..\src-tauri\target\release\bundle\nsis\SiteDatum_1.4.1_x64-setup.exe'),
    [string]$TestRoot = (Join-Path $env:TEMP ('SiteDatum-C8-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))),
    [switch]$Execute,
    [switch]$SmokeOnly,
    [switch]$DisposableProfileAcknowledged
)

$ErrorActionPreference = 'Stop'

function Resolve-AbsolutePath([string]$Path) {
    return [IO.Path]::GetFullPath($ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Path))
}

$installer = Resolve-AbsolutePath $InstallerPath
$root = Resolve-AbsolutePath $TestRoot
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())

if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) {
    throw "C8 installer was not found: $installer"
}
if (-not $root.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw "C8 test root must remain inside the current disposable profile's temporary directory: $tempRoot"
}

$signature = try { (Get-AuthenticodeSignature -LiteralPath $installer).Status.ToString() } catch { 'Unavailable' }
$installedExecutable = Join-Path $env:LOCALAPPDATA 'SiteDatum\SiteDatum.exe'
$profileLooksDisposable = $env:USERNAME -like 'CodexSandbox*' -or $env:USERNAME -like '*SiteDatum*C8*'
$existingInstall = Test-Path -LiteralPath $installedExecutable

$preflight = [ordered]@{
    currentUser = $env:USERNAME
    userProfile = $env:USERPROFILE
    localAppData = $env:LOCALAPPDATA
    installerPath = $installer
    installerSha256 = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash
    signatureStatus = $signature
    testRoot = $root
    disposableNameDetected = $profileLooksDisposable
    existingInstallDetected = $existingInstall
    executeRequested = [bool]$Execute
}

$preflight | ConvertTo-Json

if (-not $Execute) {
    Write-Host 'Preflight only. Re-run inside the disposable Windows profile with -Execute -DisposableProfileAcknowledged.'
    exit 0
}
if (-not $DisposableProfileAcknowledged) {
    throw 'Execution refused. Pass -DisposableProfileAcknowledged only after confirming this is a disposable local account or VM.'
}
if (-not $profileLooksDisposable) {
    throw "Execution refused for user '$env:USERNAME'. Use a disposable account named CodexSandbox* or *SiteDatum*C8*."
}
if ($existingInstall) {
    throw "Execution refused because SiteDatum is already installed for this profile: $installedExecutable"
}

New-Item -ItemType Directory -Path $root -Force | Out-Null
$evidencePath = Join-Path $root 'c8-profile-evidence.json'
$startedUtc = [DateTime]::UtcNow.ToString('o')

$installerProcess = Start-Process -FilePath $installer -ArgumentList '/S' -WindowStyle Hidden -Wait -PassThru
if ($installerProcess.ExitCode -ne 0) {
    throw "SiteDatum installer failed with exit code $($installerProcess.ExitCode)."
}
if (-not (Test-Path -LiteralPath $installedExecutable -PathType Leaf)) {
    throw "Installer exited successfully but SiteDatum.exe was not found: $installedExecutable"
}

$appProcess = Start-Process -FilePath $installedExecutable -WorkingDirectory (Split-Path $installedExecutable) -PassThru
Start-Sleep -Seconds 8
$appProcess.Refresh()
if ($appProcess.HasExited) {
    throw "SiteDatum exited during the eight-second clean-profile startup observation with code $($appProcess.ExitCode)."
}

$evidence = [ordered]@{
    startedUtc = $startedUtc
    completedUtc = [DateTime]::UtcNow.ToString('o')
    currentUser = $env:USERNAME
    installerSha256 = $preflight.installerSha256
    signatureStatus = $signature
    installerExitCode = $installerProcess.ExitCode
    installedExecutable = $installedExecutable
    executableVersion = (Get-Item -LiteralPath $installedExecutable).VersionInfo.FileVersion
    startupObservationSeconds = 8
    startupRemainedRunning = $true
    testRoot = $root
    containsCustomerContent = $false
    manualChecklistStatus = if ($SmokeOnly) { 'not-run-smoke-only' } else { 'operator-action-required' }
}
$evidence | ConvertTo-Json | Set-Content -LiteralPath $evidencePath -Encoding utf8

if ($SmokeOnly) {
    Stop-Process -Id $appProcess.Id
    Write-Host "Clean-profile startup smoke passed. Evidence: $evidencePath"
    exit 0
}

Write-Host ''
Write-Host 'SiteDatum is running in the disposable profile.'
Write-Host "Use only fictional records under: $root"
Write-Host 'Complete C8-05 through C8-12 in docs\engineering\C8-LAUNCH-AUDIT.md.'
Write-Host 'Do not uninstall or remove the profile until evidence and any failure diagnostics are reviewed.'
Write-Host "Content-free startup evidence: $evidencePath"

