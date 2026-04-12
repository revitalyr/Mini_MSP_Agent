# Enhanced startup script with real API server
[Parameter(Mandatory=$false)]
[string]$Build = "false"

[Parameter(Mandatory=$false)]
[string]$ConfigPath = "config\agent.toml"

Write-Host "🚀 Starting Mini MSP Agent with Real Data API on Windows" -ForegroundColor Cyan

# Check if Python is available
$pythonCmd = Get-Command python -ErrorAction SilentlyContinue
if (-not $pythonCmd) {
    $pythonCmd = Get-Command python3 -ErrorAction SilentlyContinue
}

if (-not $pythonCmd) {
    Write-Host "❌ Python not found. Please install Python 3.7+ to run API server" -ForegroundColor Red
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
    Write-Host "🔨 Building project..." -ForegroundColor Yellow
    
    # Build server
    Push-Location server
    try {
        cargo build --release
        Write-Host "✅ Server built successfully" -ForegroundColor Green
    } catch {
        Write-Host "❌ Server build failed: $_" -ForegroundColor Red
        Pop-Location
        exit 1
    }
    Pop-Location
    
    # Build agent
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

# Check binaries
$ServerPath = "target\release\server.exe"
$AgentPath = "target\release\simple_agent.exe"

if (-not (Test-Path $ServerPath)) {
    Write-Host "❌ Server not found: $ServerPath" -ForegroundColor Red
    Write-Host "💡 Run with -Build parameter to build the project" -ForegroundColor Yellow
    exit 1
}

if (-not (Test-Path $AgentPath)) {
    Write-Host "❌ Agent not found: $AgentPath" -ForegroundColor Red
    Write-Host "💡 Run with -Build parameter to build the project" -ForegroundColor Yellow
    exit 1
}

Write-Host "✅ Binaries found:" -ForegroundColor Green
Write-Host "   Server: $ServerPath" -ForegroundColor White
Write-Host "   Agent:  $AgentPath" -ForegroundColor White

# Function to cleanup processes
function Cleanup-Processes {
    param($NatsProcess, $ServerProcess, $ApiProcess, $AgentProcess)
    
    Write-Host "`nSHUTDOWN: Stopping all services..." -ForegroundColor Yellow
    
    if ($ApiProcess -and -not $ApiProcess.HasExited) {
        Stop-Process -Id $ApiProcess.Id -Force -ErrorAction SilentlyContinue
        Write-Host "✅ API Server stopped" -ForegroundColor Green
    }
    
    if ($AgentProcess -and -not $AgentProcess.HasExited) {
        Stop-Process -Id $AgentProcess.Id -Force -ErrorAction SilentlyContinue
        Write-Host "✅ Agent stopped" -ForegroundColor Green
    }
    
    if ($ServerProcess -and -not $ServerProcess.HasExited) {
        Stop-Process -Id $ServerProcess.Id -Force -ErrorAction SilentlyContinue
        Write-Host "✅ Server stopped" -ForegroundColor Green
    }
    
    if ($NatsProcess -and -not $NatsProcess.HasExited) {
        Stop-Process -Id $NatsProcess.Id -Force -ErrorAction SilentlyContinue
        Write-Host "✅ NATS stopped" -ForegroundColor Green
    }
    
    Write-Host "👋 Work completed" -ForegroundColor Cyan
}

try {
    # Start NATS server
    Write-Host "NATS: Starting NATS server..." -ForegroundColor Yellow
    $NatsProcess = Start-Process -FilePath "nats-server-v2.10.25-windows-amd64\nats-server.exe" -ArgumentList "-m", "8222", "-p", "4222" -PassThru
    
    # Wait for NATS to start
    Write-Host "NATS: Waiting for NATS to start..." -ForegroundColor Yellow
    $natsReady = $false
    for ($i = 1; $i -le 30; $i++) {
        Start-Sleep -Seconds 1
        try {
            $response = Test-NetConnection -ComputerName localhost -Port 4222 -InformationLevel Quiet
            if ($response.TcpTestSucceeded) {
                Write-Host "NATS: NATS server started successfully" -ForegroundColor Green
                $natsReady = $true
                break
            }
        } catch {
            # Continue waiting
        }
        
        if ($i % 5 -eq 0) {
            Write-Host "NATS: Attempt $i/30: Checking NATS on port 4222..." -ForegroundColor Gray
        }
    }
    
    if (-not $natsReady) {
        Write-Host "❌ NATS failed to start within 30 seconds" -ForegroundColor Red
        Cleanup-Processes -NatsProcess $null $null $null $null
        exit 1
    }

    # Start API server
    Write-Host "API: Starting real data API server..." -ForegroundColor Yellow
    $ApiProcess = Start-Process -FilePath $pythonCmd -ArgumentList "api_server.py" -WorkingDirectory $PSScriptRoot -PassThru
    
    # Wait for API server to start
    Write-Host "API: Waiting for API server to start..." -ForegroundColor Yellow
    $apiReady = $false
    for ($i = 1; $i -le 15; $i++) {
        Start-Sleep -Seconds 1
        try {
            $response = Test-NetConnection -ComputerName localhost -Port 5000 -InformationLevel Quiet
            if ($response.TcpTestSucceeded) {
                Write-Host "API: API server started successfully" -ForegroundColor Green
                $apiReady = $true
                break
            }
        } catch {
            # Continue waiting
        }
        
        if ($i % 3 -eq 0) {
            Write-Host "API: Attempt $i/15: Checking API on port 5000..." -ForegroundColor Gray
        }
    }
    
    if (-not $apiReady) {
        Write-Host "❌ API server failed to start within 15 seconds" -ForegroundColor Red
        Cleanup-Processes -NatsProcess $null $null $null $null
        exit 1
    }

    # Start main server
    Write-Host "SERVER: Starting main web server on port 8081..." -ForegroundColor Yellow
    $ServerProcess = Start-Process -FilePath $ServerPath -ArgumentList "-c", $ConfigPath -PassThru -WindowStyle Hidden
    
    # Wait for server to start
    Write-Host "SERVER: Waiting for server to start..." -ForegroundColor Yellow
    $serverReady = $false
    for ($i = 1; $i -le 30; $i++) {
        Start-Sleep -Seconds 1
        try {
            $response = Invoke-RestMethod -Uri "http://localhost:8081/health" -Method GET -TimeoutSec 2
            if ($response.StatusCode -eq 200) {
                Write-Host "SERVER: Server started successfully on http://localhost:8081" -ForegroundColor Green
                $serverReady = $true
                break
            }
        } catch {
            # Continue waiting
        }
        
        if ($i % 5 -eq 0) {
            Write-Host "SERVER: Attempt $i/30: Checking http://localhost:8081/health..." -ForegroundColor Gray
        }
    }
    
    if (-not $serverReady) {
        Write-Host "❌ Server failed to start within 30 seconds" -ForegroundColor Red
        Cleanup-Processes -NatsProcess $null $null $null $null
        exit 1
    }
    
    $serverHealth = Invoke-RestMethod -Uri "http://localhost:8081/health" -Method GET
    Write-Host "SERVER: Health check response: $($serverHealth.Content)" -ForegroundColor Green

    # Start agent
    Write-Host "AGENT: Starting agent with real data reporting..." -ForegroundColor Yellow
    $AgentProcess = Start-Process -FilePath $AgentPath -PassThru -WindowStyle Normal
    
    # Wait for agent to start
    Write-Host "AGENT: Waiting for agent to initialize..." -ForegroundColor Yellow
    Start-Sleep -Seconds 5
    
    # Check if agent is running
    if (-not $AgentProcess.HasExited) {
        Write-Host "AGENT: Agent started successfully" -ForegroundColor Green
    } else {
        Write-Host "AGENT: Failed to start agent" -ForegroundColor Red
        Cleanup-Processes -NatsProcess $ServerProcess $ApiProcess $null
        exit 1
    }

    Write-Host "SYSTEM: All components started successfully!" -ForegroundColor Green
    Write-Host "NATS:    nats://localhost:4222" -ForegroundColor Cyan
    Write-Host "Server:  http://localhost:8081" -ForegroundColor Cyan
    Write-Host "API:     http://localhost:5000" -ForegroundColor Magenta
    Write-Host "Agent:   Connected and reporting real data" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Available endpoints:" -ForegroundColor Yellow
    Write-Host "  Main server: http://localhost:8081/health" -ForegroundColor White
    Write-Host "  WebSocket:    http://localhost:8081/ws" -ForegroundColor White
    Write-Host "  Static files: http://localhost:8081/static/" -ForegroundColor White
    Write-Host ""
    Write-Host "REAL DATA API:" -ForegroundColor Magenta
    Write-Host "  Dashboard:    http://localhost:5000" -ForegroundColor White
    Write-Host "  Agents API:  http://localhost:5000/api/agents" -ForegroundColor White
    Write-Host "  Metrics API:  http://localhost:5000/api/metrics" -ForegroundColor White
    Write-Host "  Commands API: http://localhost:5000/api/command" -ForegroundColor White
    Write-Host ""
    Write-Host "Opening real-time dashboard in browser..." -ForegroundColor Yellow

    # Open dashboard in browser
    Start-Process "http://localhost:5000" -ErrorAction SilentlyContinue

    Write-Host ""
    Write-Host "🔧 Press Ctrl+C to stop all services" -ForegroundColor Yellow

    # Set up cleanup on script exit
    $cleanup = {
        Cleanup-Processes -NatsProcess $ServerProcess $ApiProcess $AgentProcess
    }
    
    # Wait for Ctrl+C
    try {
        while ($true) {
            Start-Sleep -Seconds 1
            
            # Check if processes are still running
            if ($NatsProcess.HasExited) {
                Write-Host "NATS: NATS server stopped unexpectedly" -ForegroundColor Red
                break
            }
            
            if ($ServerProcess.HasExited) {
                Write-Host "SERVER: Server stopped unexpectedly" -ForegroundColor Red
                break
            }
            
            if ($ApiProcess.HasExited) {
                Write-Host "API: API server stopped unexpectedly" -ForegroundColor Red
                break
            }
            
            if ($AgentProcess.HasExited) {
                Write-Host "AGENT: Agent stopped unexpectedly" -ForegroundColor Red
                break
            }
        }
    }
    finally {
        & $cleanup
    }

} catch {
    Write-Host "❌ Error during startup: $_" -ForegroundColor Red
    Cleanup-Processes -NatsProcess $ServerProcess $ApiProcess $AgentProcess
    exit 1
}
