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
        --config|-c)
            AGENT_CONFIG="$2"
            shift 2
            ;;
        --build|-Build)
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
    echo "🔧 Сборка C++ плагинов с preset linux-clang-20-debug..."
    if [ -f "src/plugins/CMakeLists.txt" ]; then
        pushd src/plugins > /dev/null
        cmake --preset linux-clang-20-debug
        if ! cmake --build --preset linux-clang-20-debug; then
            echo "❌ Ошибка сборки плагинов"
            popd > /dev/null
            exit 1
        fi
        mkdir -p ../agent/plugins
        find build/linux-clang-20-debug -name "*.so" -exec cp {} ../agent/plugins/ \;
        echo "✅ Плагины собраны и скопированы в agent/plugins"
        popd > /dev/null
    else
        echo "⚠️ CMakeLists.txt не найден в src/plugins/, пропускаю сборку плагинов"
    fi
fi

# Пути к бинарникам
SERVER_PATH="target/debug/server"
AGENT_PATH="target/debug/simple_agent"

# Создание директории для логов (ранее, до запуска процессов)
mkdir -p logs

# Запуск NATS broker
echo "📡 Запуск NATS broker на порту 4222..."
if command -v nats-server >/dev/null 2>&1; then
    nats-server -p 4222 -m 8222 &
    NATS_PID=$!
    echo "✅ NATS broker запущен (PID: $NATS_PID)"
    sleep 2
else
    echo "❌ NATS server не найден. Установите с: curl -sf https://binaries.nats.dev/nats-io/nats-server/v2.10.25/nats-server-v2.10.25-linux-amd64.tar.gz | tar xz && sudo mv nats-server-v2.10.25-linux-amd64/nats-server /usr/local/bin/"
    exit 1
fi

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

# Создание директории для логов
mkdir -p logs

# Проверка конфигурации агента
if [ ! -f "$AGENT_CONFIG" ]; then
    echo "⚠️ Конфигурационный файл агента не найден: $AGENT_CONFIG"
    echo "💡 Создаю конфигурацию по умолчанию..."
    
    # Создание директории configs если не существует
    mkdir -p configs
    
    # Создание конфигурации по умолчанию
    cat > "$AGENT_CONFIG" << EOF
# Mini MSP Agent Configuration
server_url = "http://localhost:$SERVER_PORT"
ws_url = "ws://localhost:$SERVER_PORT/ws"
broker_url = "nats://localhost:4222"
interval = 30
agent_id = "unix-agent-001"
log_level = "info"
log_dir = "logs"
disable_signature_check = false
allowed_commands = ["ps", "top", "df", "free", "uptime", "whoami", "id", "uname", "date", "ls", "cat", "grep", "wc", "head", "tail", "netstat", "ss", "ip", "echo"]
max_file_size = 1048576
command_timeout_secs = 60
EOF
    
    echo "✅ Конфигурация создана: $AGENT_CONFIG"
fi

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
"$AGENT_PATH" --config "$AGENT_CONFIG" --plugin-dir src/agent/plugins &
AGENT_PID=$!

echo "✅ Сервер и агент запущены!"
echo "📊 Dashboard: http://localhost:$SERVER_PORT/static/plugin_control.html"
echo "📋 Список агентов: http://localhost:$SERVER_PORT/agents"
echo "🔧 Нажмите Ctrl+C для остановки"

# Функция очистки
cleanup() {
    echo "🛑 Остановка сервисов..."
    kill $SERVER_PID 2>/dev/null
    kill $AGENT_PID 2>/dev/null
    kill $NATS_PID 2>/dev/null
    echo "✅ Сервисы остановлены"
    echo "👋 Работа завершена"
    exit 0
}

# Установка обработчика сигналов
trap cleanup INT TERM

# Ожидание процессов
wait
