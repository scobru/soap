# Install the Soap app for the current user, in %LOCALAPPDATA%\Programs\Soap,
# with a Start menu shortcut. No admin rights needed.
$ErrorActionPreference = "Stop"
$dest = Join-Path $env:LOCALAPPDATA "Programs\Soap"
Get-Process Soap -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "$dest\*" } | Stop-Process -Force
Remove-Item -Recurse -Force $dest -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $dest | Out-Null
Copy-Item -Recurse (Join-Path $PSScriptRoot "Soap\*") $dest

$shortcut = (New-Object -ComObject WScript.Shell).CreateShortcut((Join-Path ([Environment]::GetFolderPath("Programs")) "Soap.lnk"))
$shortcut.TargetPath = Join-Path $dest "Soap.exe"
$shortcut.WorkingDirectory = $dest
$shortcut.Description = "Soap voice cleaner"
$shortcut.Save()
Write-Host "Soap installed in $dest. Find it in the Start menu."
Read-Host "Press Enter to close" | Out-Null
