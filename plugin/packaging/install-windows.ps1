# Install Remove That Dirt (CLAP and VST3) into Common Files, where every DAW
# looks. Use -User to install for the current user only, without admin rights.
param([switch]$User)
$ErrorActionPreference = "Stop"
$name = "Remove That Dirt"
Set-Location $PSScriptRoot

if ($User) {
    $root = Join-Path $env:LOCALAPPDATA "Programs\Common"
} else {
    $admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
        [Security.Principal.WindowsBuiltInRole]::Administrator)
    if (-not $admin) {
        Start-Process powershell -Verb RunAs -Wait -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`""
        exit
    }
    $root = $env:CommonProgramFiles
}

$clapDir = Join-Path $root "CLAP"
$vst3Dir = Join-Path $root "VST3"
New-Item -ItemType Directory -Force -Path $clapDir, $vst3Dir | Out-Null
Remove-Item -Recurse -Force (Join-Path $clapDir $name), (Join-Path $vst3Dir "$name.vst3") -ErrorAction SilentlyContinue
Copy-Item -Recurse "CLAP\$name" $clapDir
Copy-Item -Recurse "VST3\$name.vst3" $vst3Dir
Write-Host "Installed in $clapDir and $vst3Dir. Rescan plugins in your DAW."
if (-not $User) { Read-Host "Press Enter to close" | Out-Null }
