# Build the Soap desktop app package for Windows with the Clear core that
# scripts/build-clear-windows.ps1 produced. Run from plugin/.
param(
    [string]$Version = "dev",
    [string]$ClearDir = "clear-windows-x64"
)
$ErrorActionPreference = "Stop"

cargo build --release -p soap-app
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

$out = "target/dist/soap-app-$Version-windows"
Remove-Item -Recurse -Force $out, "$out.zip" -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path "$out/Soap" | Out-Null

# Soap.exe sits beside the Clear DLLs: it finds the core there, and the MSVC
# runtime DLLs shipped with the core serve the exe too.
Copy-Item target/release/soap-app.exe "$out/Soap/Soap.exe"
Copy-Item "$ClearDir/*" "$out/Soap/"
Copy-Item packaging/app/install-windows.ps1 "$out/install.ps1"
Copy-Item packaging/app/install-windows.cmd "$out/install.cmd"
Copy-Item packaging/app/README.txt "$out/"

Compress-Archive -Path $out -DestinationPath "$out.zip" -Force
Write-Host "Package: $out.zip"
