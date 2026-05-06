# Boost.DLL Plugin System Migration Guide

## Overview

Миграция с legacy `libloading` на современную систему плагинов на базе **Boost.DLL** с C++23.

## Архитектура

```
┌─────────────────┐     FFI (C API)      ┌─────────────────────────┐
│  Rust Server    │ ◄──────────────────► │  BoostPluginManager    │
│                 │   msp_manager_*    │  (C++23, Boost.DLL)     │
│  boost_plugin.rs│                      │                         │
└─────────────────┘                      │  ┌─────────────────┐  │
                                         │  │ SystemInfoPlugin│  │
                                         │  │ (C++23)         │  │
                                         │  └─────────────────┘  │
                                         └─────────────────────────┘
```

## Новые компоненты

### C++ часть

| Файл | Назначение |
|------|-----------|
| `plugins/cpp/include/boost_plugin_api.hpp` | Modern C++23 plugin API |
| `plugins/cpp/src/system_info_plugin.cpp` | SystemInfo плагин на Boost.DLL |
| `plugins/cpp/src/plugin_manager.cpp` | BoostPluginManager реализация |
| `plugins/cpp/src/c_api_wrapper.cpp` | C API для Rust FFI |
| `plugins/cpp/CMakeLists.txt.boost` | CMake для сборки |

### Rust часть

| Файл | Назначение |
|------|-----------|
| `apps/server/src/boost_plugin.rs` | FFI bindings и Registry |
| `apps/server/src/api/agents.rs` | Интеграция в командную обработку |

## Сборка

### Требования

- CMake 3.25+
- Boost 1.82+ (filesystem, dll)
- C++23 компилятор:
  - GCC 13+ / Clang 17+ (Linux)
  - MSVC 2022+ (Windows)
  - Xcode 15+ (macOS)

### Сборка C++ библиотек

```bash
cd plugins/cpp

# Linux
export BOOST_ROOT=/usr/local/boost  # или путь к вашему Boost
cmake --preset linux-release -C CMakeLists.txt.boost
cmake --build --preset linux-release

# Или напрямую:
mkdir -p build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release \
         -DBoost_ROOT=/usr/local/boost \
         -f ../CMakeLists.txt.boost
make -j$(nproc)
```

### Результат сборки

```
plugins/cpp/build/
├── plugins/
│   ├── SystemInfoPlugin.so      # Плагин системной информации
│   └── ...
└── lib/
    └── libBoostPluginManager.so  # Менеджер плагинов (для линковки)
```

### Линковка с Rust

Добавить в `apps/server/build.rs`:

```rust
fn main() {
    // Линкуем C++ библиотеку
    println!("cargo:rustc-link-search=native=plugins/cpp/build/lib");
    println!("cargo:rustc-link-lib=dylib=BoostPluginManager");
    
    // Линкуем Boost
    println!("cargo:rustc-link-search=native=/usr/local/boost/lib");
    println!("cargo:rustc-link-lib=dylib=boost_filesystem");
    println!("cargo:rustc-link-lib=dylib=boost_system");
    
    // C++ стандартная библиотека
    println!("cargo:rustc-link-lib=dylib=stdc++");
}
```

## Использование

### Выполнение команд

Теперь при отправке команды на агент:

1. **Сначала** пробуются Boost.DLL плагины (локально на сервере)
2. **Затем** команда отправляется агенту через NATS/WebSocket

```rust
// Пример: GetSystemInfo
let result = registry.execute_command_auto("GetSystemInfo", None);
// Если плагин найден и успешно выполнил команду - возвращается результат
// Иначе - команда отправляется агенту
```

### API ответы

Ответы теперь содержат поле `source` для отслеживания:

```json
{
  "success": true,
  "agent_id": "...",
  "source": "boost_plugin",  // или "agent"
  "data": { ... }
}
```

## Добавление нового плагина

### Шаг 1: C++ плагин

```cpp
#include "boost_plugin_api.hpp"

class MyPlugin : public msp::plugins::IPlugin {
public:
    std::string name() const override { return "MyPlugin"; }
    std::string version() const override { return "1.0.0"; }
    
    PluginResult<CommandResult> execute_command(
        std::string_view command,
        std::span<const std::byte> params) override {
        // Реализация
    }
};

MSP_DEFINE_PLUGIN(MyPlugin)
```

### Шаг 2: CMake

```cmake
add_library(MyPlugin SHARED src/my_plugin.cpp)
target_link_libraries(MyPlugin PRIVATE boost_plugin_api)
```

### Шаг 3: Сборка и деплой

```bash
cd plugins/cpp/build
make MyPlugin
cp plugins/MyPlugin.so ../../../plugins/
```

## Отличия от legacy системы

| Аспект | Legacy (libloading) | Boost.DLL |
|--------|-------------------|-----------|
| API | C-style функции | C++ классы с наследованием |
| Типизация | Ручная (void*) | Type-safe |
| Потокобезопасность | Ручная | Встроенная (shared_mutex) |
| JSON API | Нет | Встроенный |
| Auto-routing | Нет | Да (по command name) |
| Metrics | Нет | Встроенные |

## Тестирование

```bash
# Проверка загрузки плагина
curl -X POST http://localhost:8080/agents/{id}/command \
  -H "Content-Type: application/json" \
  -d '{"type": "GetSystemInfo"}'

# Ожидаемый ответ (через boost_plugin):
{
  "success": true,
  "source": "boost_plugin",
  "data": {
    "platform": "linux",
    "hostname": "server-01",
    ...
  }
}
```

## Rollback

Если нужно вернуться к legacy системе:

```rust
// В api/agents.rs закомментировать блок "Try Boost.DLL plugins first"
// Оставить только fallback к агенту
```

## Troubleshooting

### Проблема: `cannot find -lBoostPluginManager`

**Решение**: Убедитесь что C++ библиотека собрана и путь указан в `build.rs`

### Проблема: `undefined symbol: _ZN5boost...`

**Решение**: Проверьте что Boost библиотеки линкуются в правильном порядке

### Проблема: Плагин не загружается

**Проверки**:
1. `ldd SystemInfoPlugin.so` - все зависимости найдены?
2. `nm -D SystemInfoPlugin.so | grep msp_create_plugin` - символы экспортированы?
3. Проверьте `api_version` совместимость (должно быть "2.0")
