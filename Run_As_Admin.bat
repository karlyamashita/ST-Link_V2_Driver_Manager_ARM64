@echo off
cd /d "%~dp0"
powershell -NoProfile -Command "Start-Process -FilePath '%~dp0target\release\ST-Link_V2_Driver_Manager_ARM64.exe' -Verb RunAs"
