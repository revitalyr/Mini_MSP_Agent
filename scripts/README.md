# Mini MSP Agent Startup Scripts

Скрипты для запуска бинарника агента и веб-сервера Mini MSP Agent.

## 📁 Файлы

- `start.ps1` - PowerShell скрипт для Windows
- `start.bat` - Batch файл для Windows (проще вариант)
- `start.sh` - Bash скрипт для Linux/macOS

## 🚀 Использование

### Windows (PowerShell)

```powershell
# Базовый запуск (без сборки плагинов)
.\scripts\start.ps1

# Полная сборка включая плагины
.\scripts\start.ps1 -Build

# С указанием порта и полной сборкой
.\scripts\start.ps1 -ServerPort 9090 -Build
```

### Windows (Batch)

```cmd
# Простой запуск (без сборки плагинов)
scripts\start.bat

# Полная сборка включая плагины
scripts\start.bat
```

### Linux/macOS

```bash
# Базовый запуск (без сборки плагинов)
./scripts/start.sh

# Полная сборка включая плагины
./scripts/start.sh --build

# С параметрами
./scripts/start.sh --port 9090 --build
```

## ⚙️ Что делают скрипты

1. **Проверка зависимостей** - проверяют наличие Rust/Cargo, CMake, Ninja
2. **Сборка проекта** (опционально) - `cargo build`
3. **Сборка C++ плагинов** (с флагом -Build) - CMake + Ninja
4. **Создание конфигурации** - создают `configs/config.toml` если отсутствует
5. **Запуск сервера** - веб-сервер на указанном порту (8080 по умолчанию)
6. **Запуск агента** - агент с указанной конфигурацией
7. **Мониторинг** - следят за работой процессов
8. **Очистка** - корректно останавливают процессы при выходе

## 📊 Доступные URL

После запуска доступны:

- 📊 **Панель управления**: `http://localhost:8080/static/plugin_control.html`
- 📋 **Список агентов**: `http://localhost:8080/agents`
- ❤️ **Проверка здоровья**: `http://localhost:8080/health`

## ⚙️ Конфигурация

Скрипты создают конфигурационный файл `configs/config.toml` с настройками по умолчанию:

```toml
[agent]
id = "windows-agent-001"  # или "unix-agent-001"
name = "Windows Agent"     # или "Unix Agent"
version = "1.0.0"

[server]
url = "http://localhost:8080"
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
```

## 🔧 Требования

- **Rust** и **Cargo** установлены
- **CMake** (для сборки плагинов)
- **Ninja** (для сборки плагинов на Windows/Linux)
- **Собранный проект** (или флаг `--build`/`-Build`)
- **Порты** 8080 (или другой указанный) должны быть свободны

### 📦 Зависимости для плагинов

**Windows:**
- Visual Studio Build Tools или Visual Studio
- CMake
- Ninja

**Linux:**
```bash
# Ubuntu/Debian
sudo apt install cmake ninja-build

# CentOS/RHEL
sudo yum install cmake ninja-build

# Arch Linux
sudo pacman -S cmake ninja
```

**macOS:**
```bash
# Install via Homebrew
brew install cmake ninja
```

## 🛑 Остановка

- **Windows**: Нажмите `Ctrl+C` в консоли
- **Linux/macOS**: Нажмите `Ctrl+C` в терминале

Скрипты корректно остановят все процессы.

## 📝 Логи

- Логи сервера: вывод в консоль
- Логи агента: `logs/agent.log`
- Директория `logs/` создается автоматически

## 🐛 Устранение проблем

### Сервер не запускается
```bash
# Проверьте что порт свободен
netstat -an | grep 8080  # Linux/macOS
netstat -an | findstr 8080  # Windows

# Попробуйте другой порт
./scripts/start.sh --port 9090
```

### Агент не запускается
```bash
# Проверьте конфигурацию
cat configs/config.toml

# Проверьте логи
tail -f logs/agent.log
```

### Проблемы с правами (Linux/macOS)
```bash
# Сделайте скрипт исполняемым
chmod +x scripts/start.sh
```

## 🎯 Примеры использования

### Разработка
```bash
# Запуск с пересборкой при каждом запуске
./scripts/start.sh --build
```

### Продакшн
```bash
# Запуск с кастомным портом и конфигурацией
./scripts/start.sh --port 9090 --config configs/prod.toml
```

### Тестирование
```bash
# Запуск на другом порту чтобы не конфликтовать
./scripts/start.sh --port 8081
```
