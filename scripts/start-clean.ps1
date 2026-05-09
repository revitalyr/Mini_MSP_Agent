# Mini MSP Agent Startup Script - Clean Version (No Web Interface)
# Запускает агент и NATS сервер без веб-интерфейса

param(
    [string]$AgentConfig = "configs/config.toml",
    [switch]$Build = $false
)

Write-Host "🚀 Starting Mini MSP Agent (Clean Version) on Windows" -ForegroundColor Green

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
    & cargo build --release --manifest-path src/agent/simple/Cargo.toml
    
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Ошибка сборки Rust проекта" -ForegroundColor Red
        exit 1
    }
    
    # Сборка C++ плагинов
    Write-Host "🔧 Сборка C++ плагинов..." -ForegroundColor Yellow
    $PluginDir = "plugins"
    $BuildDir = "$PluginDir\build"
    
    if (Test-Path $BuildDir) {
        Remove-Item -Recurse -Force $BuildDir
    }
    
    & cmake -S $PluginDir -B $BuildDir -A x64
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Ошибка конфигурации CMake" -ForegroundColor Red
        exit 1
    }
    
    & cmake --build $BuildDir --config Release
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Ошибка сборки C++ плагинов" -ForegroundColor Red
        exit 1
    }
    
    Write-Host "✅ Сборка завершена" -ForegroundColor Green
}

# Проверка бинарников
$AgentPath = "src/agent/simple/target/release/simple_agent.exe"
$NatsPath = "nats-server-v2.10.25-windows-amd64/nats-server.exe"

if (-not (Test-Path $AgentPath)) {
    Write-Host "❌ Агент не найден: $AgentPath" -ForegroundColor Red
    Write-Host "💡 Запустите с параметром -Build для сборки" -ForegroundColor Yellow
    exit 1
}

if (-not (Test-Path $NatsPath)) {
    Write-Host "❌ NATS сервер не найден: $NatsPath" -ForegroundColor Red
    exit 1
}

Write-Host "✅ Бинарники найдены" -ForegroundColor Green

# Функция очистки
function Cleanup-Processes {
    param($NatsProcess, $AgentProcess)
    
    Write-Host "`nОСТАНОВКА: Остановка всех процессов..." -ForegroundColor Yellow
    
    if ($NatsProcess -and -not $NatsProcess.HasExited) {
        Stop-Process -Id $NatsProcess.Id -Force -ErrorAction SilentlyContinue
        Write-Host "✅ NATS сервер остановлен" -ForegroundColor Green
    }
    
    if ($AgentProcess -and -not $AgentProcess.HasExited) {
        Stop-Process -Id $AgentProcess.Id -Force -ErrorAction SilentlyContinue
        Write-Host "✅ Агент остановлен" -ForegroundColor Green
    }
    
    Write-Host "👋 Работа завершена" -ForegroundColor Cyan
}

try {
    # Запуск NATS сервера
    Write-Host "NATS: Запуск NATS сервера..." -ForegroundColor Yellow
    $NatsProcess = Start-Process -FilePath $NatsPath -ArgumentList "-m 8222 -p 4222" -PassThru
    
    # Ожидание запуска NATS
    Write-Host "NATS: Ожидание запуска NATS сервера..." -ForegroundColor Yellow
    $natsReady = $false
    for ($i = 1; $i -le 30; $i++) {
        Start-Sleep -Seconds 1
        try {
            $response = Test-NetConnection -ComputerName localhost -Port 4222 -InformationLevel Quiet
            if ($response.TcpTestSucceeded) {
                Write-Host "NATS: Сервер запущен успешно" -ForegroundColor Green
                $natsReady = $true
                break
            }
        } catch {
            # Продолжаем ожидание
        }
        
        if ($i % 5 -eq 0) {
            Write-Host "NATS: Попытка $i/30: Проверка порта 4222..." -ForegroundColor Gray
        }
    }
    
    if (-not $natsReady) {
        Write-Host "❌ NATS сервер не запустился за 30 секунд" -ForegroundColor Red
        Cleanup-Processes -NatsProcess $null -AgentProcess $null
        exit 1
    }

    # Запуск агента
    Write-Host "AGENT: Запуск агента..." -ForegroundColor Yellow
    $AgentProcess = Start-Process -FilePath $AgentPath -ArgumentList "--config $AgentConfig" -PassThru -WindowStyle Normal
    
    Write-Host "AGENT: Ожидание инициализации агента..." -ForegroundColor Yellow
    Start-Sleep -Seconds 3
    
    if (-not $AgentProcess.HasExited) {
        Write-Host "AGENT: Агент запущен успешно" -ForegroundColor Green
    } else {
        Write-Host "AGENT: Агент не запустился" -ForegroundColor Red
    }

    Write-Host "SYSTEM: Система запущена успешно!" -ForegroundColor Green
    Write-Host "NATS:   localhost:4222 (мониторинг: localhost:8222)" -ForegroundColor Cyan
    Write-Host "AGENT:  Запущен в фоновом режиме" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "🔧 Нажмите Ctrl+C для остановки" -ForegroundColor Yellow

    # Настройка очистки при выходе
    $cleanup = {
        Cleanup-Processes -NatsProcess $NatsProcess -AgentProcess $AgentProcess
    }
    
    # Ожидание Ctrl+C
    try {
        while ($true) {
            Start-Sleep -Seconds 1
            
            # Проверка статуса процессов
            if ($NatsProcess.HasExited) {
                Write-Host "NATS: Сервер остановлен неожиданно" -ForegroundColor Red
                break
            }
            
            if ($AgentProcess.HasExited) {
                Write-Host "AGENT: Агент остановился неожиданно" -ForegroundColor Red
                break
            }
        }
    }
    finally {
        & $cleanup
    }

} catch {
    Write-Host "❌ Ошибка при запуске: $_" -ForegroundColor Red
    Cleanup-Processes -NatsProcess $null -AgentProcess $null
    exit 1
}
