@echo off
REM Build script for C++ plugins on Windows

setlocal enabledelayedexpansion

set PLUGIN_DIR=%~dp0
set BUILD_DIR=%PLUGIN_DIR%build
set AGENT_PLUGIN_DIR=%PLUGIN_DIR%..\agent\plugins

echo 🔨 Building Mini MSP Agent C++ plugins...

REM Create build directory
if not exist "%BUILD_DIR%" mkdir "%BUILD_DIR%"
if not exist "%AGENT_PLUGIN_DIR%" mkdir "%AGENT_PLUGIN_DIR%"

REM Configure and build
cd /d "%BUILD_DIR%"

echo 📦 Configuring CMake...
cmake .. -DCMAKE_BUILD_TYPE=Release -G "Visual Studio 16 2019" -A x64

echo 🏗️  Building plugins...
cmake --build . --config Release

REM Copy plugins to agent directory
echo 📋 Copying plugins to agent directory...

if exist "Release\system_plugin.dll" (
    copy "Release\system_plugin.dll" "%AGENT_PLUGIN_DIR%\"
    echo ✅ Copied system_plugin.dll
) else (
    echo ❌ system_plugin.dll not found
)

echo 🎉 Plugin build completed!
echo 📁 Plugin location: %AGENT_PLUGIN_DIR%

pause
