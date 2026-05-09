# Mini MSP Agent Startup Script for Windows - All Components
# Запускает все компоненты системы

param(
    [string]$Config = "configs/config.toml",
    [switch]$Build = $false,
    [switch]$Clean = $false
)

Write-Host "🚀 Starting Mini MSP Agent (All Components) on Windows" -ForegroundColor Green

# Function to check if command exists
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

# Function to start component
function Start-Component {
    param(
        [string]$Component,
        [string]$Path,
        [string]$Arguments = ""
    )
    
    Write-Host "🔄 Starting $Component..." -ForegroundColor Yellow
    
    try {
        if ($Arguments) {
            $Process = Start-Process -FilePath $Path -ArgumentList $Arguments -PassThru -WindowStyle Normal
        } else {
            $Process = Start-Process -FilePath $Path -PassThru -WindowStyle Normal
        }
        
        Write-Host "✅ $Component started (PID: $($Process.Id))" -ForegroundColor Green
        return $Process
    }
    catch {
        Write-Host "❌ Failed to start $Component`: $_" -ForegroundColor Red
        return $null
    }
}

# Check prerequisites
if (-not (Test-Command "cargo")) {
    Write-Host "❌ Rust/Cargo not found. Please install Rust first." -ForegroundColor Red
    exit 1
}

# Build if requested
if ($Build) {
    Write-Host "📦 Building all components..." -ForegroundColor Yellow
    & .\scripts\build-all.ps1
    
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Build failed" -ForegroundColor Red
        exit 1
    }
}

# Clean if requested
if ($Clean) {
    Write-Host "🧹 Cleaning build directories..." -ForegroundColor Yellow
    Remove-Item -Recurse -Force "target" -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force "apps/*/target" -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force "apps/qt_client/build" -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force "plugins/build" -ErrorAction SilentlyContinue
    Write-Host "✅ Clean completed" -ForegroundColor Green
}

# Check binaries
$NatsPath = "nats-server-v2.10.25-windows-amd64/nats-server.exe"
$AgentPath = "apps/agent/target/release/agent.exe"
$ServerPath = "apps/server/target/release/server.exe"
$QtPath = "apps/qt_client/build/Release/qt_client.exe"

$MissingBinaries = @()

if (-not (Test-Path $NatsPath)) { $MissingBinaries += "NATS Server" }
if (-not (Test-Path $AgentPath)) { $MissingBinaries += "Agent" }
if (-not (Test-Path $ServerPath)) { $MissingBinaries += "Server" }
if (-not (Test-Path $QtPath)) { $MissingBinaries += "Qt Client" }

if ($MissingBinaries.Count -gt 0) {
    Write-Host "❌ Missing binaries:" -ForegroundColor Red
    foreach ($Binary in $MissingBinaries) {
        Write-Host "  - $Binary" -ForegroundColor White
    }
    Write-Host "💡 Run with -Build parameter to build components" -ForegroundColor Yellow
    exit 1
}

Write-Host "✅ All binaries found" -ForegroundColor Green

# Create logs directory
if (-not (Test-Path "logs")) {
    New-Item -ItemType Directory -Force "logs" | Out-Null
}

# Function to cleanup processes
function Cleanup-Processes {
    param($NatsProcess, $ServerProcess, $AgentProcess, $QtProcess)
    
    Write-Host "`n🛑 Stopping all components..." -ForegroundColor Yellow
    
    $Processes = @($NatsProcess, $ServerProcess, $AgentProcess, $QtProcess)
    foreach ($Process in $Processes) {
        if ($Process -and -not $Process.HasExited) {
            Stop-Process -Id $Process.Id -Force -ErrorAction SilentlyContinue
            Write-Host "✅ Process $($Process.Id) stopped" -ForegroundColor Green
        }
    }
    
    Write-Host "👋 All components stopped" -ForegroundColor Cyan
}

try {
    # Start NATS server
    $NatsProcess = Start-Component -Component "NATS Server" -Path $NatsPath -Arguments "-m 8222 -p 4222"
    
    if (-not $NatsProcess) {
        Write-Host "❌ Failed to start NATS server" -ForegroundColor Red
        exit 1
    }
    
    # Wait for NATS to start
    Write-Host "⏳ Waiting for NATS to start..." -ForegroundColor Yellow
    $NatsReady = $false
    for ($i = 1; $i -le 30; $i++) {
        Start-Sleep -Seconds 1
        try {
            $response = Test-NetConnection -ComputerName localhost -Port 4222 -InformationLevel Quiet
            if ($response.TcpTestSucceeded) {
                Write-Host "✅ NATS server is ready" -ForegroundColor Green
                $NatsReady = $true
                break
            }
        } catch {
            # Continue waiting
        }
        
        if ($i % 5 -eq 0) {
            Write-Host "Attempt $i/30: Checking NATS on port 4222..." -ForegroundColor Gray
        }
    }
    
    if (-not $NatsReady) {
        Write-Host "❌ NATS server failed to start within 30 seconds" -ForegroundColor Red
        Cleanup-Processes -NatsProcess $NatsProcess -ServerProcess $null -AgentProcess $null -QtProcess $null
        exit 1
    }
    
    # Start server
    $ServerProcess = Start-Component -Component "Server" -Path $ServerPath -Arguments "--config $Config"
    
    # Start Qt client
    $QtProcess = Start-Component -Component "Qt Client" -Path $QtPath
    
    # Start agent
    $AgentProcess = Start-Component -Component "Agent" -Path $AgentPath -Arguments "--config $Config"
    
    Write-Host ""
    Write-Host "🎉 Mini MSP Agent started successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "📊 Running components:" -ForegroundColor Cyan
    Write-Host "NATS:    localhost:4222 (monitoring: localhost:8222)" -ForegroundColor White
    Write-Host "Server:  Running (PID: $($ServerProcess.Id))" -ForegroundColor White
    Write-Host "Qt GUI:  Running (PID: $($QtProcess.Id))" -ForegroundColor White
    Write-Host "Agent:   Running (PID: $($AgentProcess.Id))" -ForegroundColor White
    Write-Host ""
    Write-Host "🔧 Press Ctrl+C to stop all components" -ForegroundColor Yellow
    
    # Set up cleanup on script exit
    $cleanup = {
        Cleanup-Processes -NatsProcess $NatsProcess -ServerProcess $ServerProcess -AgentProcess $AgentProcess -QtProcess $QtProcess
    }
    
    # Wait for Ctrl+C
    try {
        while ($true) {
            Start-Sleep -Seconds 1
            
            # Check critical processes
            if ($NatsProcess.HasExited) {
                Write-Host "❌ NATS server died unexpectedly" -ForegroundColor Red
                break
            }
            
            if ($AgentProcess.HasExited) {
                Write-Host "⚠️  Agent died unexpectedly" -ForegroundColor Yellow
            }
            
            if ($ServerProcess.HasExited) {
                Write-Host "⚠️  Server died unexpectedly" -ForegroundColor Yellow
            }
            
            if ($QtProcess.HasExited) {
                Write-Host "⚠️  Qt GUI died unexpectedly" -ForegroundColor Yellow
            }
        }
    }
    finally {
        & $cleanup
    }
}
catch {
    Write-Host "❌ Error during startup: $_" -ForegroundColor Red
    Cleanup-Processes -NatsProcess $null -ServerProcess $null -AgentProcess $null -QtProcess $null
    exit 1
}
