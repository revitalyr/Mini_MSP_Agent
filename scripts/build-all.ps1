# Mini MSP Agent Build Script for Windows
# Собирает все компоненты проекта

param(
    [string[]]$Components = @(),
    [switch]$Clean = $false
)

Write-Host "🚀 Building Mini MSP Agent for Windows..." -ForegroundColor Green

# Function to build component
function Build-Component {
    param(
        [string]$Component
    )
    
    Write-Host "📦 Building $Component..." -ForegroundColor Yellow
    
    try {
        switch ($Component) {
            "agent" {
                cargo build --release --manifest-path apps/agent/Cargo.toml
            }
            "server" {
                cargo build --release --manifest-path apps/server/Cargo.toml
            }
            "qt_client" {
                Write-Host "🔧 Building Qt Client..." -ForegroundColor Yellow
                $QtDir = "apps/qt_client"
                $BuildDir = "$QtDir/build"
                
                if (Test-Path $BuildDir) {
                    Remove-Item -Recurse -Force $BuildDir
                }
                
                New-Item -ItemType Directory -Force $BuildDir | Out-Null
                Set-Location $BuildDir
                
                # Try cmake without generator specification
                if (Get-Command "cmake" -ErrorAction SilentlyContinue) {
                    cmake .. -DCMAKE_BUILD_TYPE=Release
                    if ($LASTEXITCODE -eq 0) {
                        cmake --build . --config Release
                    }
                } else {
                    Write-Host "❌ CMake not found" -ForegroundColor Red
                    return $false
                }
                
                Set-Location $PSScriptRoot
            }
            "shared" {
                cargo build --release --manifest-path shared/Cargo.toml
            }
            "plugins" {
                Write-Host "🔧 Building C++ plugins..." -ForegroundColor Yellow
                $PluginDir = "plugins"
                $BuildDir = "$PluginDir/build"
                
                if (Test-Path $BuildDir) {
                    Remove-Item -Recurse -Force $BuildDir
                }
                
                if (Get-Command "cmake" -ErrorAction SilentlyContinue) {
                    cmake -S $PluginDir -B $BuildDir -A x64
                    if ($LASTEXITCODE -eq 0) {
                        cmake --build $BuildDir --config Release
                    }
                } else {
                    Write-Host "❌ CMake not found" -ForegroundColor Red
                    return $false
                }
            }
            default {
                Write-Host "❌ Unknown component: $Component" -ForegroundColor Red
                return $false
            }
        }
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ $Component built successfully" -ForegroundColor Green
            return $true
        } else {
            Write-Host "❌ $Component build failed" -ForegroundColor Red
            return $false
        }
    }
    catch {
        Write-Host "❌ Error building $Component`: $_" -ForegroundColor Red
        return $false
    }
}

# Clean build if requested
if ($Clean) {
    Write-Host "🧹 Cleaning build directories..." -ForegroundColor Yellow
    Remove-Item -Recurse -Force "target" -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force "apps/*/target" -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force "apps/qt_client/build" -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force "plugins/build" -ErrorAction SilentlyContinue
    Write-Host "✅ Clean completed" -ForegroundColor Green
}

# Check prerequisites
function Test-Command {
    param($Command)
    try {
        Get-Command $Command -ErrorAction Stop | Out-Null
        return $true
    }
    catch {
        return $false
    }
}

if (-not (Test-Command "cargo")) {
    Write-Host "❌ Rust/Cargo not found" -ForegroundColor Red
    exit 1
}

if (-not (Test-Command "cmake")) {
    Write-Host "❌ CMake not found" -ForegroundColor Red
    exit 1
}

# Determine components to build
if ($Components.Count -eq 0) {
    Write-Host "Building all components..." -ForegroundColor Cyan
    $Components = @("shared", "plugins", "agent", "server", "qt_client")
}

# Build components
$Success = $true
foreach ($Component in $Components) {
    if (-not (Build-Component -Component $Component)) {
        $Success = $false
    }
}

if ($Success) {
    Write-Host "🎉 Build completed successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "📋 Available binaries:" -ForegroundColor Cyan
    Write-Host "Agent:    apps/agent/target/release/agent.exe" -ForegroundColor White
    Write-Host "Server:   apps/server/target/release/server.exe" -ForegroundColor White
    Write-Host "Qt Client: apps/qt_client/build/Release/qt_client.exe" -ForegroundColor White
    Write-Host "Plugins:  plugins/build/Release/" -ForegroundColor White
    Write-Host ""
    Write-Host "🚀 Run with: ./scripts/start.ps1" -ForegroundColor Yellow
} else {
    Write-Host "❌ Build failed!" -ForegroundColor Red
    exit 1
}
