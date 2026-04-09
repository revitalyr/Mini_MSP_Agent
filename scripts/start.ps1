# Mini MSP Agent Startup Script for Windows
# Запускает бинарник агента и веб-сервер

param(
    [string]$ServerPort = "8081",
    [string]$AgentConfig = "configs/config.toml",
    [switch]$Build = $false
)

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

# Пути к бинарникам
if ($Build) {
    $ServerPath = "target/release/server.exe"
    $AgentPath = "target/release/agent.exe"
} else {
    $ServerPath = "target/debug/server.exe"
    $AgentPath = "target/debug/agent.exe"
}

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

# Проверка доступности порта
try {
    $PortCheck = Test-NetConnection -ComputerName "localhost" -Port $ServerPort -InformationLevel Quiet -ErrorAction Stop
    if ($PortCheck) {
        Write-Host "❌ Порт $ServerPort уже используется" -ForegroundColor Red
        Write-Host "💡 Выберите другой порт: .\scripts\start.ps1 -Port 8081" -ForegroundColor Yellow
        exit 1
    }
}
catch {
    # Порт свободен, продолжаем
}

# Запуск сервера в фоновом режиме
Write-Host "🖥️ Запуск веб-сервера на порту $ServerPort..." -ForegroundColor Yellow
$ServerProcess = Start-Process -FilePath $ServerPath -ArgumentList "--port", $ServerPort -PassThru -WindowStyle Hidden

# Ожидание запуска сервера
Write-Host "⏳ Ожидание запуска сервера..." -ForegroundColor Yellow
$MaxWaitTime = 10
$WaitTime = 0
$ServerStarted = $false

while ($WaitTime -lt $MaxWaitTime -and -not $ServerStarted) {
    Start-Sleep -Seconds 1
    $WaitTime++
    
    try {
        $null = Invoke-WebRequest -Uri "http://localhost:$ServerPort/health" -TimeoutSec 2 -ErrorAction Stop
        $ServerStarted = $true
        Write-Host "✅ Сервер запущен на http://localhost:$ServerPort" -ForegroundColor Green
    }
    catch {
        Write-Host "🔄 Попытка ${WaitTime}/${MaxWaitTime}: сервер еще не готов..." -ForegroundColor Yellow
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

# Запуск агента
Write-Host "🤖 Запуск агента с конфигурацией: $AgentConfig" -ForegroundColor Yellow
$AgentProcess = Start-Process -FilePath $AgentPath -ArgumentList "--config", $AgentConfig, "--plugin-dir", "plugins" -PassThru -WindowStyle Hidden

Write-Host "✅ Сервер и агент запущены!" -ForegroundColor Green
Write-Host "📊 Панель управления: http://localhost:$ServerPort/static/plugin_control.html" -ForegroundColor Cyan
Write-Host "📋 Список агентов: http://localhost:$ServerPort/agents" -ForegroundColor Cyan
Write-Host "🔧 Нажмите Ctrl+C для остановки" -ForegroundColor Yellow

# Ожидание Ctrl+C для остановки
try {
    while ($true) {
        Start-Sleep -Seconds 1
        
        # Проверка что процессы еще работают
        if ($ServerProcess.HasExited) {
            Write-Host "❌ Сервер остановился" -ForegroundColor Red
            break
        }
        
        if ($AgentProcess.HasExited) {
            Write-Host "❌ Агент остановился" -ForegroundColor Red
            break
        }
    }
}
finally {
    Write-Host "🛑 Остановка сервисов..." -ForegroundColor Yellow
    
    # Остановка процессов
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