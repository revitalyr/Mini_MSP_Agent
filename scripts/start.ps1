# Mini MSP Agent Startup Script for Windows
# Запускает бинарник агента и веб-сервер

param(
    [string]$ServerPort = "8081",
    [string]$AgentConfig = "configs/config.toml",
    [switch]$Build = $false
)

# Handle Unix-style arguments passed as positional parameters
if ($ServerPort -eq "--build" -or $ServerPort -eq "-build") {
    $Build = $true
    $ServerPort = "8081"
}
if ($AgentConfig -eq "--build" -or $AgentConfig -eq "-build") {
    $Build = $true
    $AgentConfig = "configs/config.toml"
}

# Validate port is a number
if ($ServerPort -notmatch '^\d+$') {
    Write-Host "Error: ServerPort must be a number, got: $ServerPort" -ForegroundColor Red
    Write-Host "Usage: .\scripts\start.ps1 [[-ServerPort] <port>] [-AgentConfig <config>] [-Build]" -ForegroundColor Yellow
    Write-Host "       .\scripts\start.ps1 -Build              # Build and run with defaults" -ForegroundColor Yellow
    Write-Host "       .\scripts\start.ps1 8081 -Build         # Build and run on port 8081" -ForegroundColor Yellow
    exit 1
}

Write-Host "🚀 Starting Mini MSP Agent on Windows" -ForegroundColor Green

# Функция проверки существования команды
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

# Проверка предпосылок
if (-not (Test-Command "cargo")) {
    Write-Host "❌ Rust/Cargo не найден. Пожалуйста установите Rust сначала." -ForegroundColor Red
    exit 1
}

# Сборка проекта если нужно
if ($Build) {
    Write-Host "📦 Сборка проекта..." -ForegroundColor Yellow
    & cargo build --release
    
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Ошибка сборки Rust проекта" -ForegroundColor Red
        exit 1
    }
    
    # Сборка C++ плагинов
    Write-Host "🔧 Сборка C++ плагинов..." -ForegroundColor Yellow
    $PluginDir = "plugins"
    $BuildDir = "$PluginDir\build"
    $AgentPluginDir = "plugins"  # Изменено на "plugins" как ожидает агент
    
    # Создание директорий
    if (-not (Test-Path $BuildDir)) {
        New-Item -ItemType Directory -Path $BuildDir | Out-Null
    }
    if (-not (Test-Path "../$AgentPluginDir")) {
        New-Item -ItemType Directory -Path "../$AgentPluginDir" | Out-Null
    }
    
    # Сборка плагинов через CMake
    Push-Location $PluginDir
    try {
        # Use platform-aware build script
        $BuildScript = Join-Path $PluginDir "build_platform_plugins.ps1"
        
        if (Test-Path $BuildScript) {
            Write-Host "🔧 Using platform-aware plugin builder..." -ForegroundColor Yellow
            & $BuildScript
        } else {
            Write-Host "⚠️ Platform-aware builder not found, using fallback..." -ForegroundColor Yellow
            
            # Проверяем наличие уже собранных плагинов
            $existingPlugins = @("modern_system_plugin.dll", "modern_directory_info_plugin.dll")
            $allPluginsExist = $true
            
            foreach ($plugin in $existingPlugins) {
                if (-not (Test-Path $plugin)) {
                    $allPluginsExist = $false
                    break
                }
            }
            
            if ($allPluginsExist) {
                Write-Host "✅ Плагины уже собраны, пропускаем CMake сборку" -ForegroundColor Green
            } else {
                # Fallback to original build logic
                Write-Host "⚠️ Using fallback build method..." -ForegroundColor Yellow
                # Проверяем наличие CMake и Ninja
                $cmake = Get-Command cmake -ErrorAction SilentlyContinue
                $ninja = Get-Command ninja -ErrorAction SilentlyContinue
                
                if (-not $cmake) {
                    Write-Host "⚠️ CMake не найден. Пропускаю сборку плагинов." -ForegroundColor Yellow
                } elseif (-not $ninja) {
                    Write-Host "⚠️ Ninja не найден. Пропускаю сборку плагинов." -ForegroundColor Yellow
                } else {
                    # Создаем build директорию если нужно
                    if (-not (Test-Path "build")) {
                        New-Item -ItemType Directory -Path "build" | Out-Null
                    }
                    
                    Push-Location "build"
                    try {
                        # Очистка кэша CMake если нужно
                        if (Test-Path "CMakeCache.txt") {
                            Write-Host "🧹 Очистка кэша CMake..." -ForegroundColor Yellow
                            Remove-Item "CMakeCache.txt" -Force
                            Remove-Item "CMakeFiles" -Recurse -Force -ErrorAction SilentlyContinue
                        }
                        
                        # Находим компилятор для Windows
                        $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
                        if (Test-Path $vswhere) {
                            $vsPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
                            $vcVarsPath = "$vsPath\VC\Auxiliary\Build\vcvars64.bat"
                            
                            if (Test-Path $vcVarsPath) {
                                Write-Host "🔧 Настройка окружения Visual Studio..." -ForegroundColor Yellow
                                
                                # Запускаем vcvars64.bat и импортируем окружение
                                $vcVarsOutput = & cmd /c "`"$vcVarsPath`" && set" 2>&1 | Out-String
                                $vcVarsLines = $vcVarsOutput -split "`r`n"
                                
                                foreach ($line in $vcVarsLines) {
                                    if ($line -match "^(.+?)=(.*)$") {
                                        $varName = $matches[1]
                                        $varValue = $matches[2]
                                        Set-Item -Path "env:$varName" -Value $varValue
                                    }
                                }
                                
                                # Используем современный CMakeLists.txt для C++23 плагинов с Ninja
                                Write-Host "🔧 Конфигурация CMake (современные C++23 плагины с Ninja)..." -ForegroundColor Yellow
                                & cmake .. -DCMAKE_BUILD_TYPE=Release -G "Ninja"
                                
                                if ($LASTEXITCODE -eq 0) {
                                    Write-Host "🔧 Сборка плагинов с Ninja..." -ForegroundColor Yellow
                                    & ninja 2>&1
                                    Write-Host "Ninja exit code: $LASTEXITCODE" -ForegroundColor Yellow
                                    
                                    if ($LASTEXITCODE -eq 0) {
                                        # Копирование плагинов в корневую директорию
                                        Write-Host "📋 Копирование плагинов..." -ForegroundColor Yellow
                                        Copy-Item "*.dll" "..\" -Force -ErrorAction SilentlyContinue
                                    } else {
                                        Write-Host "❌ Ошибка сборки плагинов с Ninja" -ForegroundColor Red
                                    }
                                } else {
                                    Write-Host "❌ Ошибка конфигурации CMake с Ninja" -ForegroundColor Red
                                }
                            } else {
                                Write-Host "❌ vcvars64.bat не найден" -ForegroundColor Red
                            }
                        } else {
                            Write-Host "❌ Visual Studio не найдена" -ForegroundColor Red
                        }
                    }
                    catch {
                        Write-Host "❌ Ошибка при сборке плагинов: $_" -ForegroundColor Red
                    }
                    finally {
                        Pop-Location
                    }
                }
            }
        }
    }
    finally {
        Pop-Location
    }
}

# Paths to binaries - always use release
$ServerPath = "target/release/server.exe"
$AgentPath = "target/release/simple_agent.exe"

# Проверка существования бинарников
if (-not (Test-Path $ServerPath)) {
    Write-Host "❌ Сервер не найден: $ServerPath" -ForegroundColor Red
    Write-Host "💡 Запустите с параметром -Build для сборки проекта" -ForegroundColor Yellow
    exit 1
}

if (-not (Test-Path $AgentPath)) {
    Write-Host "❌ Агент не найден: $AgentPath" -ForegroundColor Red
    Write-Host "💡 Запустите с параметром -Build для сборки проекта" -ForegroundColor Yellow
    exit 1
}

Write-Host "✅ Бинарники найдены:" -ForegroundColor Green
Write-Host "   Сервер: $ServerPath" -ForegroundColor Gray
Write-Host "   Агент:  $AgentPath" -ForegroundColor Gray

# Проверка конфигурации агента
if (-not (Test-Path $AgentConfig)) {
    Write-Host "⚠️ Конфигурационный файл агента не найден: $AgentConfig" -ForegroundColor Yellow
    Write-Host "💡 Создаю конфигурацию по умолчанию..." -ForegroundColor Yellow
    
    # Создание директории configs если не существует
    if (-not (Test-Path "configs")) {
        New-Item -ItemType Directory -Name "configs" | Out-Null
    }
    
    # Создание конфигурации по умолчанию
    $DefaultConfig = @"
# Mini MSP Agent Configuration
server_url = "http://localhost:$ServerPort"
ws_url = "ws://localhost:$ServerPort/ws"
broker_url = "nats://localhost:4222"
interval = 30
agent_id = "windows-agent-001"
log_level = "info"
log_dir = "logs"
disable_signature_check = false
allowed_commands = ["ps", "top", "df", "free", "uptime", "whoami", "id", "uname", "date", "ls", "cat", "grep", "wc", "head", "tail", "netstat", "ss", "ip", "echo"]
max_file_size = 1048576
command_timeout_secs = 60
"@
    
    $DefaultConfig | Out-File -FilePath $AgentConfig -Encoding UTF8
    Write-Host "✅ Конфигурация создана: $AgentConfig" -ForegroundColor Green
}

# Создание директории для логов
if (-not (Test-Path "logs")) {
    New-Item -ItemType Directory -Name "logs" | Out-Null
}

# Start NATS server
Write-Host "NATS: Starting NATS server..." -ForegroundColor Yellow
$NatsPath = ".\nats-server-v2.10.25-windows-amd64\nats-server.exe"
if (-not (Test-Path $NatsPath)) {
    Write-Host "NATS: NATS server not found at $NatsPath" -ForegroundColor Red
    Write-Host "NATS: Downloading NATS server..." -ForegroundColor Yellow
    Invoke-WebRequest -Uri "https://github.com/nats-io/nats-server/releases/download/v2.10.25/nats-server-v2.10.25-windows-amd64.zip" -OutFile "nats-server.zip"
    Expand-Archive -Path "nats-server.zip" -DestinationPath "."
    Remove-Item "nats-server.zip"
}

Write-Host "NATS: Starting NATS on ports 4222 (clients) and 8222 (monitoring)..." -ForegroundColor Green
$NatsProcess = Start-Process -FilePath $NatsPath -ArgumentList "--jetstream", "-p", "4222", "-m", "8222" -PassThru -WindowStyle Hidden

# Wait for NATS to start
Write-Host "NATS: Waiting for NATS to start..." -ForegroundColor Yellow
Start-Sleep -Seconds 3

# Check if NATS is running
try {
    $null = Test-NetConnection -ComputerName "localhost" -Port 4222 -InformationLevel Quiet -ErrorAction Stop
    Write-Host "NATS: NATS server started successfully" -ForegroundColor Green
}
catch {
    Write-Host "NATS: Failed to start NATS server" -ForegroundColor Red
    Stop-Process -Id $NatsProcess.Id -Force -ErrorAction SilentlyContinue
    exit 1
}

# Проверка доступности порта
try {
    $null = Invoke-WebRequest -Uri "http://localhost:$ServerPort/health" -TimeoutSec 2 -ErrorAction Stop
    Write-Host "Port $ServerPort already in use by another server" -ForegroundColor Red
    Write-Host "Choose different port: .\scripts\start.ps1 -Port 8081" -ForegroundColor Yellow
    exit 1
}
catch {
    # Port is free, continue
}

# Запуск сервера в фоновом режиме
Write-Host "🖥️ Запуск веб-сервера на порту $ServerPort..." -ForegroundColor Yellow
$ServerProcess = Start-Process -FilePath $ServerPath -ArgumentList "--port", $ServerPort -PassThru -WindowStyle Hidden

# Wait for server to start
Write-Host "SERVER: Waiting for server to start..." -ForegroundColor Yellow
$MaxWaitTime = 30
$WaitTime = 0
$ServerStarted = $false

while ($WaitTime -lt $MaxWaitTime -and -not $ServerStarted) {
    Start-Sleep -Seconds 1
    $WaitTime++
    
    try {
        Write-Host "SERVER: Attempt ${WaitTime}/${MaxWaitTime}: Checking http://localhost:$ServerPort/health..." -ForegroundColor Gray
        $response = Invoke-WebRequest -Uri "http://localhost:$ServerPort/health" -TimeoutSec 3 -ErrorAction Stop
        if ($response.StatusCode -eq 200) {
            $ServerStarted = $true
            Write-Host "SERVER: Server started successfully on http://localhost:$ServerPort" -ForegroundColor Green
            Write-Host "SERVER: Health check response: $($response.Content)" -ForegroundColor Cyan
        }
    }
    catch {
        Write-Host "SERVER: Attempt ${WaitTime}/${MaxWaitTime}: Server not responding yet..." -ForegroundColor Yellow
    }
}

if (-not $ServerStarted) {
    Write-Host "❌ Сервер не запустился или недоступен после $MaxWaitTime секунд" -ForegroundColor Red
    Write-Host "🔍 Проверка логов сервера..." -ForegroundColor Yellow
    
    # Показываем последние строки из логов если они есть
    if (Test-Path "logs") {
        Get-ChildItem "logs\*.log" -ErrorAction SilentlyContinue | ForEach-Object {
            Write-Host "📄 Лог файл: $($_.Name)" -ForegroundColor Cyan
            Get-Content $_.FullName | Select-Object -Last 10 | ForEach-Object { Write-Host "   $_" -ForegroundColor Gray }
        }
    }
    
    Stop-Process -Id $ServerProcess.Id -Force -ErrorAction SilentlyContinue
    exit 1
}

# Start agent (simple_agent doesn't need config parameters)
Write-Host "AGENT: Starting agent..." -ForegroundColor Green
$agentProcess = Start-Process -FilePath $AgentPath -PassThru -WindowStyle Normal

# Wait for agent to start
Write-Host "AGENT: Waiting for agent to initialize..." -ForegroundColor Yellow
Start-Sleep -Seconds 5

# Check if agent is running
if (-not $agentProcess.HasExited) {
    Write-Host "AGENT: Agent started successfully" -ForegroundColor Green
} else {
    Write-Host "AGENT: Failed to start agent" -ForegroundColor Red
    Stop-Process -Id $NatsProcess.Id -Force -ErrorAction SilentlyContinue
    Stop-Process -Id $ServerProcess.Id -Force -ErrorAction SilentlyContinue
    exit 1
}

Write-Host "SYSTEM: All components started successfully!" -ForegroundColor Green
Write-Host "NATS:    nats://localhost:4222" -ForegroundColor Cyan
Write-Host "Server:  http://localhost:$ServerPort" -ForegroundColor Cyan
Write-Host "Agent:   Connected and active" -ForegroundColor Cyan
Write-Host ""
Write-Host "Available endpoints:" -ForegroundColor Yellow
Write-Host "  Health check: http://localhost:$ServerPort/health" -ForegroundColor White
Write-Host "  Agent list:   http://localhost:$ServerPort/agents" -ForegroundColor White
Write-Host "  WebSocket:    http://localhost:$ServerPort/ws" -ForegroundColor White
Write-Host "  Static files: http://localhost:$ServerPort/static/" -ForegroundColor White
Write-Host ""
Write-Host "REAL DEMO INTERFACE:" -ForegroundColor Magenta
Write-Host "  Opening real-time monitoring dashboard..." -ForegroundColor Yellow

# Open real demo page in browser
Start-Process "http://localhost:8081" -ErrorAction SilentlyContinue
Start-Process "$PSScriptRoot\..\real_demo.html" -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "Press Ctrl+C to stop" -ForegroundColor Yellow

# Ожидание Ctrl+C для остановки
try {
    while ($true) {
        Start-Sleep -Seconds 1
        
        # Проверка что процессы еще работают
        if ($NatsProcess.HasExited) {
            Write-Host "NATS: NATS server stopped unexpectedly" -ForegroundColor Red
            break
        }
        
        if ($ServerProcess.HasExited) {
            Write-Host "SERVER: Web server stopped unexpectedly" -ForegroundColor Red
            break
        }
        
        if ($AgentProcess.HasExited) {
            Write-Host "AGENT: Agent stopped unexpectedly" -ForegroundColor Red
            break
        }
    }
}
finally {
    Write-Host "SHUTDOWN: Stopping all services..." -ForegroundColor Yellow
    
    # Stop all processes gracefully
    if (-not $NatsProcess.HasExited) {
        Stop-Process -Id $NatsProcess.Id -Force -ErrorAction SilentlyContinue
        Write-Host "✅ NATS server stopped" -ForegroundColor Green
    }
    
    if (-not $ServerProcess.HasExited) {
        Stop-Process -Id $ServerProcess.Id -Force -ErrorAction SilentlyContinue
        Write-Host "✅ Сервер остановлен" -ForegroundColor Green
    }
    
    if (-not $AgentProcess.HasExited) {
        Stop-Process -Id $AgentProcess.Id -Force -ErrorAction SilentlyContinue
        Write-Host "✅ Агент остановлен" -ForegroundColor Green
    }
    
    Write-Host "👋 Работа завершена" -ForegroundColor Green
}