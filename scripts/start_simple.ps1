# Simple startup script for Mini MSP Agent Demo
# Works without NATS - direct API communication

[Parameter(Mandatory=$false)]
[string]$Build = "false"

Write-Host "🚀 Starting Mini MSP Agent Simple Demo on Windows" -ForegroundColor Cyan

# Check if Python is available
$pythonCmd = Get-Command python -ErrorAction SilentlyContinue
if (-not $pythonCmd) {
    $pythonCmd = Get-Command python3 -ErrorAction SilentlyContinue
}

if (-not $pythonCmd) {
    Write-Host "❌ Python not found. Please install Python 3.7+ to run demo server" -ForegroundColor Red
    exit 1
}

Write-Host "🐍 Found Python: $($pythonCmd.Name)" -ForegroundColor Green

# Install required Python packages if needed
Write-Host "📦 Checking Python dependencies..." -ForegroundColor Yellow
try {
    & $pythonCmd -m pip install flask flask-cors requests -q
    Write-Host "✅ Python dependencies installed" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to install Python dependencies: $_" -ForegroundColor Red
    exit 1
}

# Build project if requested
if ($Build -eq "true") {
    Write-Host "🔨 Building agent..." -ForegroundColor Yellow
    
    Push-Location simple_agent
    try {
        cargo build --release
        Write-Host "✅ Agent built successfully" -ForegroundColor Green
    } catch {
        Write-Host "❌ Agent build failed: $_" -ForegroundColor Red
        Pop-Location
        exit 1
    }
    Pop-Location
    
    Write-Host "🎉 Build completed!" -ForegroundColor Green
}

# Check binary
$AgentPath = "target\release\simple_agent.exe"

if (-not (Test-Path $AgentPath)) {
    Write-Host "❌ Agent not found: $AgentPath" -ForegroundColor Red
    Write-Host "💡 Run with -Build parameter to build the project" -ForegroundColor Yellow
    exit 1
}

Write-Host "✅ Agent binary found: $AgentPath" -ForegroundColor Green

# Function to cleanup processes
function Cleanup-Processes {
    param($DemoProcess)
    
    Write-Host "`nSHUTDOWN: Stopping demo server..." -ForegroundColor Yellow
    
    if ($DemoProcess -and -not $DemoProcess.HasExited) {
        Stop-Process -Id $DemoProcess.Id -Force -ErrorAction SilentlyContinue
        Write-Host "✅ Demo server stopped" -ForegroundColor Green
    }
    
    Write-Host "👋 Demo completed" -ForegroundColor Cyan
}

try {
    # Start demo API server
    Write-Host "DEMO: Starting simple demo server..." -ForegroundColor Yellow
    $DemoProcess = Start-Process -FilePath $pythonCmd -ArgumentList "demo_simple.py" -WorkingDirectory $PSScriptRoot -PassThru
    
    # Wait for demo server to start
    Write-Host "DEMO: Waiting for demo server to start..." -ForegroundColor Yellow
    $demoReady = $false
    for ($i = 1; $i -le 10; $i++) {
        Start-Sleep -Seconds 1
        try {
            $response = Test-NetConnection -ComputerName localhost -Port 5001 -InformationLevel Quiet
            if ($response.TcpTestSucceeded) {
                Write-Host "DEMO: Demo server started successfully" -ForegroundColor Green
                $demoReady = $true
                break
            }
        } catch {
            # Continue waiting
        }
        
        if ($i % 2 -eq 0) {
            Write-Host "DEMO: Attempt $i/10: Checking demo on port 5001..." -ForegroundColor Gray
        }
    }
    
    if (-not $demoReady) {
        Write-Host "❌ Demo server failed to start within 10 seconds" -ForegroundColor Red
        Cleanup-Processes -DemoProcess $null
        exit 1
    }

    # Start agent (optional - for demonstration)
    Write-Host "AGENT: Starting agent for demonstration..." -ForegroundColor Yellow
    $AgentProcess = Start-Process -FilePath $AgentPath -PassThru -WindowStyle Normal
    
    Write-Host "DEMO: Waiting for agent to initialize..." -ForegroundColor Yellow
    Start-Sleep -Seconds 3
    
    if (-not $AgentProcess.HasExited) {
        Write-Host "AGENT: Agent started successfully" -ForegroundColor Green
    } else {
        Write-Host "AGENT: Agent failed to start" -ForegroundColor Red
    }

    Write-Host "SYSTEM: Simple demo started successfully!" -ForegroundColor Green
    Write-Host "DEMO:  http://localhost:5001" -ForegroundColor Magenta
    Write-Host "AGENT: Running in background" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Available endpoints:" -ForegroundColor Yellow
    Write-Host "  Demo Dashboard: http://localhost:5001" -ForegroundColor White
    Write-Host "  Agents API:    http://localhost:5001/api/agents" -ForegroundColor White
    Write-Host "  Commands API:   http://localhost:5001/api/command" -ForegroundColor White
    Write-Host "  Metrics API:    http://localhost:5001/api/metrics" -ForegroundColor White
    Write-Host ""
    Write-Host "🎯 Features:" -ForegroundColor Green
    Write-Host "  ✅ Real-time agent data" -ForegroundColor White
    Write-Host "  ✅ Plugin information display" -ForegroundColor White
    Write-Host "  ✅ System metrics" -ForegroundColor White
    Write-Host "  ✅ Command processing" -ForegroundColor White
    Write-Host "  ✅ No NATS dependency" -ForegroundColor White
    Write-Host "  ✅ Interactive controls" -ForegroundColor White
    Write-Host ""
    Write-Host "🌐 Opening demo dashboard in browser..." -ForegroundColor Yellow

    # Open dashboard in browser
    Start-Process "http://localhost:5001" -ErrorAction SilentlyContinue

    Write-Host ""
    Write-Host "🔧 Press Ctrl+C to stop demo" -ForegroundColor Yellow

    # Set up cleanup on script exit
    $cleanup = {
        Cleanup-Processes -DemoProcess $null
    }
    
    # Wait for Ctrl+C
    try {
        while ($true) {
            Start-Sleep -Seconds 1
            
            # Check if demo process is still running
            if ($DemoProcess.HasExited) {
                Write-Host "DEMO: Demo server stopped unexpectedly" -ForegroundColor Red
                break
            }
        }
    }
    finally {
        & $cleanup
    }

} catch {
    Write-Host "❌ Error during startup: $_" -ForegroundColor Red
    Cleanup-Processes -DemoProcess $null
    exit 1
}
