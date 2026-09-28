# Build the Clear native core for Windows x64 from Desert Ant Labs'
# desert-ant-core (the npm package ships no win32 build) and collect every DLL
# it needs into one folder.
#
# Needs: Swift 6.2 for Windows, an MSVC developer environment (dumpbin/lib),
# Python with pip, and Git for Windows. CI sets these up; see .github/workflows/build.yml.
param(
    [string]$Version = "3.5.0",
    [string]$OutDir = "clear-windows-x64"
)
$ErrorActionPreference = "Stop"

$work = Join-Path ([System.IO.Path]::GetTempPath()) "desert-ant-core-$Version"
if (-not (Test-Path "$work/Package.swift")) {
    git clone --depth 1 --branch "v$Version" https://github.com/Desert-Ant-Labs/desert-ant-core $work
    if ($LASTEXITCODE -ne 0) { throw "git clone failed" }
}

Push-Location $work
try {
    & ./Tools/windows/vendor-litert.ps1

    # Releases after 3.5.0 also link ONNX Runtime on Windows. Git Bash, located
    # through git itself so WSL's bash.exe is never picked up.
    if (Select-String -Path Tools/dal.sh -Pattern "dal_vendor_onnxruntime\(\)" -Quiet) {
        $gitBash = Join-Path (Split-Path (Split-Path (Get-Command git).Source)) "bin/bash.exe"
        & $gitBash -c "source Tools/dal.sh && dal_vendor_onnxruntime"
        if ($LASTEXITCODE -ne 0) { throw "ONNX Runtime vendoring failed" }
    }

    swift build -c release --product ClearNode `
        -Xlinker /LIBPATH:Vendor/litert/lib/windows-x64 `
        -Xlinker /LIBPATH:Vendor/onnxruntime/lib/windows-x64
    if ($LASTEXITCODE -ne 0) { throw "swift build failed" }
    $built = (Resolve-Path ".build/release/ClearNode.dll").Path
} finally {
    Pop-Location
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$OutDir = (Resolve-Path $OutDir).Path

# Where dependencies may come from: the build, the vendored runtimes, the Swift
# runtime, and the MSVC redistributable (app-local deployment is allowed).
$swiftRuntime = Split-Path (& where.exe swiftCore.dll | Select-Object -First 1)
$searchDirs = @(
    (Split-Path $built),
    "$work/Vendor/litert/lib/windows-x64",
    "$work/Vendor/onnxruntime/lib/windows-x64",
    $swiftRuntime
)
if ($env:VCToolsRedistDir) {
    $searchDirs += Get-ChildItem "$env:VCToolsRedistDir/x64" -Directory -Filter "Microsoft.VC*.CRT" |
        ForEach-Object { $_.FullName }
}

function Find-Dll([string]$name) {
    foreach ($dir in $searchDirs) {
        $candidate = Join-Path $dir $name
        if (Test-Path $candidate) { return $candidate }
    }
    return $null
}

# Walk the import tables from ClearNode.dll; anything not found in the search
# dirs is an OS DLL (kernel32, api-ms-win-*, ...) and stays behind.
$queue = [System.Collections.Generic.Queue[string]]::new()
$seen = @{}
$queue.Enqueue($built)
while ($queue.Count -gt 0) {
    $dll = $queue.Dequeue()
    $name = Split-Path $dll -Leaf
    if ($seen.ContainsKey($name.ToLower())) { continue }
    $seen[$name.ToLower()] = $true
    Copy-Item $dll (Join-Path $OutDir $name) -Force
    $deps = & dumpbin /dependents $dll | ForEach-Object {
        if ($_ -match '^\s+(\S+\.dll)\s*$') { $Matches[1] }
    }
    foreach ($dep in $deps) {
        $path = Find-Dll $dep
        if ($path) { $queue.Enqueue($path) }
    }
}

# Loaded at run time rather than imported, so the walk cannot see them.
foreach ($extra in "DirectML.dll", "onnxruntime_providers_shared.dll") {
    $path = Find-Dll $extra
    if ($path) { Copy-Item $path $OutDir -Force }
}
Copy-Item "$work/LICENSE.md" (Join-Path $OutDir "CLEAR-LICENSE.md") -Force

Write-Host "Clear $Version for Windows in ${OutDir}:"
Get-ChildItem $OutDir | ForEach-Object { Write-Host "  $($_.Name)  $([math]::Round($_.Length / 1MB, 1)) MB" }
