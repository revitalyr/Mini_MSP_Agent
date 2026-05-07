# Mini MSP Qt6 Client

Нативный Qt6 клиент для Mini MSP Agent, работающий напрямую с NATS без необходимости веб-сервера.

## Архитектура

```
┌─────────────────────────────────────────────────────────────┐
│                    Qt6 Client (C++/Qt6)                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │  MainWindow  │  │  NatsClient  │  │  AgentModel  │       │
│  │   (UI Qt6)   │  │  (NATS C)    │  │  (Qt Model)  │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└──────────────────────────┬──────────────────────────────────┘
                           │ NATS Protocol
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                      NATS Server                             │
└──────────────┬──────────────────────────────┬───────────────┘
               │                              │
               ▼                              ▼
┌─────────────────────────┐      ┌─────────────────────────┐
│   Mini MSP Agent (Rust) │      │   Mini MSP Server       │
│   ┌─────────────────┐   │      │   (API/HTTP/WS)         │
│   │  Plugin Loader  │   │      └─────────────────────────┘
│   │  NATS Client    │   │
│   └─────────────────┘   │
└─────────────────────────┘
```

## Требования

### Обязательные
- Qt6 (Core, Gui, Widgets, Network)
- CMake >= 3.20
- C++20 компилятор (GCC 11+, Clang 14+, MSVC 2022+)
- cnats (NATS C клиент) - автоматически скачивается через FetchContent

### Опционально
- zstd для декомпрессии сообщений
- nlohmann/json - автоматически скачивается

## Сборка

### Linux (Ubuntu/Debian)

```bash
# Установка зависимостей
sudo apt update
sudo apt install -y build-essential cmake qt6-base-dev qt6-base-dev-tools

# Сборка
cd apps/qt_client
mkdir build && cd build
cmake ..
cmake --build -j$(nproc)

# Запуск
./MiniMSPQtClient
```

### macOS

```bash
# Установка Qt6 через Homebrew
brew install qt@6 cmake

# Сборка
cd apps/qt_client
mkdir build && cd build
cmake .. -DCMAKE_PREFIX_PATH=$(brew --prefix qt@6)
cmake --build -j$(sysctl -n hw.ncpu)

# Запуск
./MiniMSPQtClient
```

### Windows (Visual Studio 2022)

```powershell
# Qt6 должен быть установлен (через Qt Online Installer или vcpkg)
# vcpkg install qt6-base

cd apps/qt_client
mkdir build && cd build
cmake .. -DCMAKE_PREFIX_PATH=C:\Qt\6.5.0\msvc2019_64
cmake --build . --config Release

# Запуск
Release\MiniMSPQtClient.exe
```

## Использование

1. Запустите NATS сервер:
   ```bash
   nats-server -js
   ```

2. Запустите Mini MSP Agent:
   ```bash
   cd /mnt/d/work/Mini_MSP_Agent
   cargo run --package simple_agent
   ```

3. Запустите Qt клиент:
   ```bash
   ./MiniMSPQtClient
   ```

4. Нажмите "Connect" для подключения к NATS

5. Выберите агента из списка и используйте кнопки команд

## Функции

- ✅ Прямое подключение к NATS (без HTTP/WebSocket)
- ✅ Отображение списка агентов с метриками (CPU/RAM/Disk)
- ✅ Отправка команд агентам
- ✅ Получение heartbeats в реальном времени
- ✅ Поддержка сжатия zstd для больших сообщений
- ✅ Системный трей с уведомлениями
- ✅ Работа в фоновом режиме

## Отличия от Web клиента

| Функция | Web Client | Qt6 Client |
|---------|-----------|------------|
| Протокол | HTTP/WebSocket | NATS |
| Зависимость от Server | Да (HTTP API) | Нет (прямое NATS) |
| Real-time updates | WebSocket | NATS Pub/Sub |
| Системный трей | Нет | Да |
| Нативный look&feel | Нет | Да |
| Автономная работа | Нет | Да |

## Конфигурация

По умолчанию клиент подключается к `nats://localhost:4222`.

Можно изменить URL в поле ввода перед подключением или через конфигурационный файл (TODO).

## Технические детали

### NATS Subjects

- `agent.heartbeat` - подписка на heartbeats от всех агентов
- `agent.{id}.commands` - отправка команд конкретному агенту
- `agent.{id}.responses` - получение ответов от агента

### Сжатие

Клиент автоматически распаковывает zstd-сжатые сообщения (проверка заголовка `Content-Encoding: zstd`).

## Лицензия

MIT License - см. основной LICENSE файл проекта.
