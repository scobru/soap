# Build the Windows release package (CLAP + VST3) with the Clear core that
# scripts/build-clear-windows.ps1 produced. Run from plugin/.
param(
    [string]$Version = "dev",
    [string]$ClearDir = "clear-windows-x64"
)
$ErrorActionPreference = "Stop"
$name = "Remove That Dirt"

cargo xtask bundle remove_that_dirt --release
if ($LASTEXITCODE -ne 0) { throw "cargo xtask bundle failed" }

cmake -S wrapper -B target/wrapper
if ($LASTEXITCODE -ne 0) { throw "cmake configure failed" }
cmake --build target/wrapper --config Release
if ($LASTEXITCODE -ne 0) { throw "cmake build failed" }
$vst3 = Get-ChildItem target/wrapper -Recurse -Filter "$name.vst3" | Select-Object -First 1
if (-not $vst3) { throw "VST3 not found" }

$out = "target/dist/remove-that-dirt-$Version-windows"
Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path "$out/CLAP/$name", "$out/VST3" | Out-Null

# The core and its DLLs sit beside the .clap: the plugin loads them from there.
Copy-Item "target/bundled/$name.clap" "$out/CLAP/$name/"
Copy-Item "$ClearDir/*" "$out/CLAP/$name/"
Copy-Item -Recurse $vst3.FullName "$out/VST3/"
Copy-Item packaging/install-windows.ps1 "$out/install.ps1"
Copy-Item packaging/install-windows.cmd "$out/install.cmd"
Copy-Item packaging/README.txt "$out/"

Compress-Archive -Path $out -DestinationPath "$out.zip" -Force
Write-Host "Package: $out.zip"
