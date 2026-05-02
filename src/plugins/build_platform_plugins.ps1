# Platform-aware Plugin Builder for Mini MSP Agent (Windows)
param(
    [switch]$Clean = $false,
    [string]$Platform = ""
)

Write-Host "🔧 Platform-aware Plugin Builder for Mini MSP Agent" -ForegroundColor Green

# Detect platform if not specified
if ($Platform -eq "") {
    if ($IsWindows) {
        $Platform = "windows"
    } elseif ($IsLinux) {
        $Platform = "linux"
    } elseif ($IsMacOS) {
        $Platform = "macos"
    } else {
        Write-Host "❌ Unsupported platform" -ForegroundColor Red
        exit 1
    }
}

Write-Host "🖥️  Detected platform: $Platform" -ForegroundColor Cyan

# Get directories
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$SourceDir = $ScriptDir
$BuildDir = Join-Path $SourceDir "build"

# Clean if requested
if ($Clean) {
    Write-Host "🧹 Cleaning build directory..." -ForegroundColor Yellow
    if (Test-Path $BuildDir) {
        Remove-Item $BuildDir -Recurse -Force
    }
}

# Create build directory
New-Item -ItemType Directory -Path $BuildDir -Force | Out-Null

# Check CMakeLists.txt exists
$CMakeFile = "CMakeLists.txt"
$CMakeSource = Join-Path $SourceDir $CMakeFile

if (-not (Test-Path $CMakeSource)) {
    Write-Host "❌ CMake file not found: $CMakeFile" -ForegroundColor Red
    exit 1
}

Write-Host "📋 Using CMake file: $CMakeFile (unified for all platforms)" -ForegroundColor Cyan

# Change to build directory
Push-Location $BuildDir

try {
    # Configure with CMake
    Write-Host "🔧 Configuring with CMake..." -ForegroundColor Yellow
    
    if ($Platform -eq "windows") {
        # Try to find Visual Studio
        $VSWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
        if (Test-Path $VSWhere) {
            try {
                $VSPath = & $VSWhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
                $VCVarsPath = Join-Path $VSPath "VC\Auxiliary\Build\vcvars64.bat"
                
                if (Test-Path $VCVarsPath) {
                    Write-Host "🔧 Setting up Visual Studio environment..." -ForegroundColor Yellow
                    
                    # Import VS environment using safer method
                    # Run vcvars64.bat and capture env vars to temp file
                    $TempEnvFile = [System.IO.Path]::GetTempFileName()
                    cmd /c "`"$VCVarsPath`" >nul 2>&1 && set > `"$TempEnvFile`""
                    
                    if (Test-Path $TempEnvFile) {
                        $EnvLines = Get-Content -Path $TempEnvFile -ErrorAction SilentlyContinue
                        foreach ($line in $EnvLines) {
                            if ($line -match "^(.+?)=(.*)$") {
                                $VarName = $matches[1]
                                $VarValue = $matches[2]
                                # Skip variables that are too long or contain special chars
                                if ($VarValue.Length -lt 8192) {
                                    Set-Item -Path "env:$VarName" -Value $VarValue -ErrorAction SilentlyContinue
                                }
                            }
                        }
                        Remove-Item -Path $TempEnvFile -Force -ErrorAction SilentlyContinue
                    }
                }
            } catch {
                Write-Host "⚠️  VS detection failed, using default environment" -ForegroundColor Yellow
            }
        }
        
        # Configure CMake from parent directory (source is in ..)
        & cmake .. -DCMAKE_BUILD_TYPE=Release -G "Ninja"
    } else {
        # Linux/macOS - configure from parent directory
        & cmake .. -DCMAKE_BUILD_TYPE=Release
    }
    
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ CMake configuration failed" -ForegroundColor Red
        exit 1
    }
    
    Write-Host "✅ CMake configuration successful" -ForegroundColor Green
    
    # Build with appropriate tool
    Write-Host "🏗️  Building plugins..." -ForegroundColor Yellow
    
    if ($Platform -eq "windows") {
        & ninja
    } else {
        & make -j4
    }
    
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Build failed" -ForegroundColor Red
        exit 1
    }
    
    Write-Host "✅ Build successful" -ForegroundColor Green
    
    # Copy plugins to platform directory
    $PlatformDir = Join-Path $SourceDir $Platform
    New-Item -ItemType Directory -Path $PlatformDir -Force | Out-Null
    
    # Copy built files
    if ($Platform -eq "windows") {
        Get-ChildItem -Path "." -Filter "*.dll" | ForEach-Object {
            Copy-Item $_.FullName (Join-Path $PlatformDir $_.Name) -Force
            Write-Host "✅ Copied $($_.Name) to $PlatformDir" -ForegroundColor Green
        }
    } else {
        Get-ChildItem -Path "." -Filter "*.so*" | ForEach-Object {
            Copy-Item $_.FullName (Join-Path $PlatformDir $_.Name) -Force
            Write-Host "✅ Copied $($_.Name) to $PlatformDir" -ForegroundColor Green
        }
    }
    
    Write-Host "🎉 Platform-specific build completed for $Platform" -ForegroundColor Green
    
} finally {
    Pop-Location
}
