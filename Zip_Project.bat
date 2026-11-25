@echo off
cd /d "%~dp0"
powershell -NoProfile -Command "$root=(Get-Location).Path; $stage=Join-Path ([System.IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString()); $project=Join-Path $stage 'ST-Link_V2_Driver_Manager_ARM64'; try { New-Item -ItemType Directory -Path $project -Force | Out-Null; Copy-Item -Path @('src','Cargo.toml','*.bat','README.md') -Destination $project -Recurse -ErrorAction Stop; Compress-Archive -Path $project -DestinationPath (Join-Path $env:USERPROFILE 'Downloads\ST-Link_V2_Driver_Manager_ARM64.zip') -Force -ErrorAction Stop } finally { Remove-Item $stage -Recurse -Force -ErrorAction SilentlyContinue }"
if errorlevel 1 exit /b 1
