#!/bin/bash
# Mini MSP Agent Universal Startup Script
# Запускает бинарник агента и веб-сервер

set -e

# Параметры по умолчанию
SERVER_PORT="8080"
AGENT_CONFIG="configs/config.toml"
BUILD=false

# Парсинг аргументов
while [[ $# -gt 0 ]]; do
    case $1 in
        --port)
            SERVER_PORT="$2"
            shift 2
            ;;
        --config)
            AGENT_CONFIG="$2"
            shift 2
            ;;
        --build)
            BUILD=true
            shift
            ;;
        -h|--help)
            echo "Использование: $0 [опции]"
            echo "Опции:"
            echo "  --port PORT      Порт веб-сервера (по умолчанию: 8080)"
            echo "  --config FILE    Конфигурационный файл агента (по умолчанию: configs/config.toml)"
            echo "  --build          Собрать проект перед запуском"
            echo "  -h, --help       Показать эту справку"
            exit 0
            ;;
        *)
            echo "Неизвестный параметр: $1"
            exit 1
            ;;
    esac
done

echo "🚀 Starting Mini MSP Agent"

# Функция проверки существования команды
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Проверка предпосылок
if ! command_exists cargo; then
    echo "❌ Rust/Cargo не найден. Пожалуйста установите Rust сначала."
    exit 1
fi

# Сборка проекта если нужно
if [ "$BUILD" = true ]; then
    echo "📦 Сборка проекта..."
    cargo build
    if [ $? -ne 0 ]; then
        echo "❌ Ошибка сборки проекта"
        exit 1
    fi
    
    # Сборка C++ плагинов
    echo "🔧 Сборка C++ плагинов..."
    if [ -f "plugins/build.sh" ]; then
        cd plugins
        chmod +x build.sh
        ./build.sh
        if [ $? -ne 0 ]; then
            echo "⚠️ Ошибка сборки плагинов, но продолжаем..."
        fi
        cd ..
    else
        echo "⚠️ build.sh не найден, пропускаю сборку плагинов"
    fi
fi

# Пути к бинарникам
SERVER_PATH="target/debug/server"
AGENT_PATH="target/debug/agent"

# Проверка существования бинарников
if [ ! -f "$SERVER_PATH" ]; then
    echo "❌ Сервер не найден: $SERVER_PATH"
    echo "💡 Запустите с параметром --build для сборки проекта"
    exit 1
fi

if [ ! -f "$AGENT_PATH" ]; then
    echo "❌ Агент не найден: $AGENT_PATH"
    echo "💡 Запустите с параметром --build для сборки проекта"
    exit 1
fi

# Проверка конфигурации агента
if [ ! -f "$AGENT_CONFIG" ]; then
    echo "⚠️ Конфигурационный файл агента не найден: $AGENT_CONFIG"
    echo "💡 Создаю конфигурацию по умолчанию..."
    
    # Создание директории configs если не существует
    mkdir -p configs
    
    # Создание конфигурации по умолчанию
    cat > "$AGENT_CONFIG" << EOF
# Mini MSP Agent Configuration
[agent]
id = "unix-agent-001"
name = "Unix Agent"
version = "1.0.0"

[server]
url = "http://localhost:$SERVER_PORT"
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
EOF
    
    echo "✅ Конфигурация создана: $AGENT_CONFIG"
fi

# Создание директории для логов
mkdir -p logs

# Запуск сервера в фоновом режиме
echo "🖥️ Запуск веб-сервера на порту $SERVER_PORT..."
"$SERVER_PATH" --port "$SERVER_PORT" &
SERVER_PID=$!

# Ожидание запуска сервера
echo "⏳ Ожидание запуска сервера..."
sleep 3

# Проверка доступности сервера
if curl -s "http://localhost:$SERVER_PORT/health" > /dev/null; then
    echo "✅ Сервер запущен на http://localhost:$SERVER_PORT"
else
    echo "❌ Сервер не запустился или недоступен"
    kill $SERVER_PID 2>/dev/null
    exit 1
fi

# Запуск агента
echo "🤖 Запуск агента с конфигурацией: $AGENT_CONFIG"
"$AGENT_PATH" --config "$AGENT_CONFIG" &
AGENT_PID=$!

echo "✅ Сервер и агент запущены!"
echo "📊 Панель управления: http://localhost:$SERVER_PORT/static/plugin_control.html"
echo "📋 Список агентов: http://localhost:$SERVER_PORT/agents"
echo "🔧 Нажмите Ctrl+C для остановки"

# Функция очистки
cleanup() {
    echo "🛑 Остановка сервисов..."
    kill $SERVER_PID 2>/dev/null
    kill $AGENT_PID 2>/dev/null
    echo "✅ Сервисы остановлены"
    echo "👋 Работа завершена"
    exit 0
}

# Установка обработчика сигналов
trap cleanup INT TERM

# Ожидание процессов
wait
