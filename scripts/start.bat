@echo off
REM Mini MSP Agent Startup Script for Windows (Batch version)
REM Запускает бинарник агента и веб-сервер

echo 🚀 Starting Mini MSP Agent on Windows

REM Проверка наличия cargo
where cargo >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo ❌ Rust/Cargo не найден. Пожалуйста установите Rust сначала.
    pause
    exit /b 1
)

REM Сборка проекта
echo 📦 Building project...
cargo build
if %ERRORLEVEL% neq 0 (
    echo ❌ Ошибка сборки проекта
    pause
    exit /b 1
)

REM Проверка существования бинарников
if not exist "target\debug\server.exe" (
    echo ❌ Сервер не найден: target\debug\server.exe
    pause
    exit /b 1
)

if not exist "target\debug\mini_msp_agent.exe" (
    echo ❌ Агент не найден: target\debug\mini_msp_agent.exe
    pause
    exit /b 1
)

REM Создание директорий
if not exist "configs" mkdir configs
if not exist "logs" mkdir logs

REM Создание конфигурации если не существует
if not exist "configs\config.toml" (
    echo 💡 Создаю конфигурацию по умолчанию...
    (
        echo # Mini MSP Agent Configuration
        echo [agent]
        echo id = "windows-agent-001"
        echo name = "Windows Agent"
        echo version = "1.0.0"
        echo.
        echo [server]
        echo url = "http://localhost:8080"
        echo api_key = ""
        echo.
        echo [logging]
        echo level = "info"
        echo file = "logs/agent.log"
        echo.
        echo [plugins]
        echo enabled = true
        echo directory = "plugins"
        echo.
        echo [system]
        echo monitor_interval = 5
        echo max_memory_usage = 512
    ) > configs\config.toml
    echo ✅ Конфигурация создана: configs\config.toml
)

REM Запуск сервера
echo 🖥️ Запуск веб-сервера на порту 8080...
start "Mini MSP Server" /MIN target\debug\server.exe --port 8080

REM Ожидание запуска сервера
echo ⏳ Ожидание запуска сервера...
timeout /t 3 /nobreak >nul

REM Проверка доступности сервера
curl -s http://localhost:8080/health >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo ❌ Сервер не запустился или недоступен
    taskkill /f /im server.exe >nul 2>&1
    pause
    exit /b 1
)

echo ✅ Сервер запущен на http://localhost:8080

REM Запуск агента
echo 🤖 Запуск агента...
start "Mini MSP Agent" /MIN target\debug\mini_msp_agent.exe --config configs\config.toml

echo.
echo ✅ Сервер и агент запущены!
echo 📊 Панель управления: http://localhost:8080/static/plugin_control.html
echo 📋 Список агентов: http://localhost:8080/agents
echo.
echo 🔧 Нажмите любую клавишу для остановки...
pause >nul

REM Остановка процессов
echo 🛑 Остановка сервисов...
taskkill /f /im server.exe >nul 2>&1
taskkill /f /im mini_msp_agent.exe >nul 2>&1
echo ✅ Сервисы остановлены
echo 👋 Работа завершена
pause
