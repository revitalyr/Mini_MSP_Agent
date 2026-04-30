#!/usr/bin/env pwsh

param(
    [Parameter(Mandatory=$false)]
    [string]$Target = "release",
    
    [Parameter(Mandatory=$false)]
    [switch]$Clean,
    
    [Parameter(Mandatory=$false)]
    [switch]$Verbose
)

Write-Host "Building Mini MSP Agent Core..." -ForegroundColor Green

# Check if we're in the right directory
if (-not (Test-Path "Cargo.toml")) {
    Write-Host "Error: Not in the agent core directory" -ForegroundColor Red
    exit 1
}

# Clean previous build if requested
if ($Clean) {
    Write-Host "Cleaning previous build..." -ForegroundColor Yellow
    cargo clean
}

# Build in release mode with optimizations
Write-Host "Building core library..." -ForegroundColor Yellow
$buildArgs = @("build", "--profile", $Target)
if ($Verbose) {
    $buildArgs += "--verbose"
}
cargo @buildArgs

# Get build information
Write-Host "Build information:" -ForegroundColor Cyan
Write-Host "  Target: $($(rustc -vV | Select-String 'host:' | ForEach-Object { $_.Split(' ')[2] }))"
Write-Host "  Profile: $Target"
Write-Host "  Optimizations: size-optimized"

# Check binary size
$depsPath = "target\$Target\deps\libmini_msp_core.rlib"
if (Test-Path $depsPath) {
    $size = (Get-Item $depsPath).Length / 1MB
    Write-Host "  Core library size: $([math]::Round($size, 2)) MB"
}

$agentPath = "..\..\target\$Target\agent.exe"
if (Test-Path $agentPath) {
    $size = (Get-Item $agentPath).Length / 1MB
    Write-Host "  Agent binary size: $([math]::Round($size, 2)) MB"
}

Write-Host "Build completed successfully!" -ForegroundColor Green
