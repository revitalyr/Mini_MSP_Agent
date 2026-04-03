# Mini MSP Agent Startup Script for Windows
# Запускает бинарник агента и веб-сервер

param(
    [string]$ServerPort = "8080",
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
    & cargo build --quiet 2>$null
    
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
    if (-not (Test-Path $AgentPluginDir)) {
        New-Item -ItemType Directory -Path $AgentPluginDir | Out-Null
    }
    
    # Сборка плагинов через CMake
    Push-Location $BuildDir
    try {
        # Проверяем наличие CMake и Ninja
        $cmake = Get-Command cmake -ErrorAction SilentlyContinue
        $ninja = Get-Command ninja -ErrorAction SilentlyContinue
        
        if (-not $cmake) {
            Write-Host "⚠️ CMake не найден. Пропускаю сборку плагинов." -ForegroundColor Yellow
        } elseif (-not $ninja) {
            Write-Host "⚠️ Ninja не найден. Пропускаю сборку плагинов." -ForegroundColor Yellow
        } else {
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
                    $vcVarsOutput = & cmd /c "`"$vcVarsPath`" && set" | Out-String
                    $vcVarsLines = $vcVarsOutput -split "`r`n"
                    
                    foreach ($line in $vcVarsLines) {
                        if ($line -match "^(.+?)=(.*)$") {
                            $varName = $matches[1]
                            $varValue = $matches[2]
                            Set-Item -Path "env:$varName" -Value $varValue
                        }
                    }
                    
                    # Пробуем простой CMakeLists.txt сначала
                    Write-Host "🔧 Конфигурация CMake (простой плагин)..." -ForegroundColor Yellow
                    # Копируем простой CMakeLists.txt временно
                    Copy-Item "../simple_CMakeLists.txt" "CMakeLists.txt" -Force
                    & cmake . -DCMAKE_BUILD_TYPE=Release -G "Ninja" 2>$null
                    
                    if ($LASTEXITCODE -eq 0) {
                        Write-Host "🔧 Сборка плагинов с Ninja..." -ForegroundColor Yellow
                        & ninja 2>$null
                        
                        if ($LASTEXITCODE -eq 0) {
                            # Создаем директорию для плагинов если нужно
                            if (-not (Test-Path "$AgentPluginDir")) {
                                New-Item -ItemType Directory -Path "$AgentPluginDir" -Force | Out-Null
                            }
                            
                            # Копирование плагинов
                            if (Test-Path "plugins/system_plugin.dll") {
                                Copy-Item "plugins/system_plugin.dll" "$AgentPluginDir\" -Force
                                Write-Host "✅ system_plugin.dll скопирован" -ForegroundColor Green
                            }
                            
                            Write-Host "✅ Плагины собраны и скопированы" -ForegroundColor Green
                        } else {
                            Write-Host "❌ Ошибка сборки плагинов с Ninja" -ForegroundColor Red
                        }
                    } else {
                        Write-Host "❌ Ошибка конфигурации CMake с Ninja" -ForegroundColor Red
                    }
                } else {
                    Write-Host "❌ vcvars64.bat не найден: $vcVarsPath" -ForegroundColor Red
                }
            } else {
                Write-Host "❌ Visual Studio не найдена" -ForegroundColor Red
            }
        }
    }
    finally {
        Pop-Location
    }
}

# Пути к бинарникам
$ServerPath = "target/debug/server.exe"
$AgentPath = "target/debug/agent.exe"

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
[agent]
id = "windows-agent-001"
name = "Windows Agent"
version = "1.0.0"

[server]
url = "http://localhost:$ServerPort"
api_key = ""

[logging]
level = "info"
file = "logs/agent.log"

[plugins]
enabled = true
directory = "plugins"

[system]
monitor_interval = 5
max_memory_usage = 512
"@
    
    $DefaultConfig | Out-File -FilePath $AgentConfig -Encoding UTF8
    Write-Host "✅ Конфигурация создана: $AgentConfig" -ForegroundColor Green
}

# Создание директории для логов
if (-not (Test-Path "logs")) {
    New-Item -ItemType Directory -Name "logs" | Out-Null
}

# Запуск сервера в фоновом режиме
Write-Host "🖥️ Запуск веб-сервера на порту $ServerPort..." -ForegroundColor Yellow
$ServerProcess = Start-Process -FilePath $ServerPath -ArgumentList "--port", $ServerPort -PassThru -WindowStyle Hidden

# Ожидание запуска сервера
Write-Host "⏳ Ожидание запуска сервера..." -ForegroundColor Yellow
Start-Sleep -Seconds 3

# Проверка доступности сервера
try {
    $Response = Invoke-WebRequest -Uri "http://localhost:$ServerPort/health" -TimeoutSec 5 -ErrorAction Stop
    Write-Host "✅ Сервер запущен на http://localhost:$ServerPort" -ForegroundColor Green
}
catch {
    Write-Host "❌ Сервер не запустился или недоступен" -ForegroundColor Red
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
