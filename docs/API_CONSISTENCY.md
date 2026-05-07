--- I Consistency Guide

## Обзор

Для обеспечения однозначности и согласованности API между Rust-сервером и Vue-фронтендом используется **OpenAPI (Swagger)** с генерацией документации прямо из кода Rust с помощью `utoipa`.

## 🏗️ Архитектура

```
┌─────────────────┐     OpenAPI Spec     ┌─────────────────┐
│  Rust Server    │ ───────────────────► │   Swagger UI    │
│   (utoipa)      │    /api-docs/        │   (/swagger-ui) │
└─────────────────┘     openapi.json     └─────────────────┘
         │
         │         ┌─────────────────┐
         └────────►│ TypeScript      │
                   │ Client (генерация)│
                   └─────────────────┘
```

## 📁 Структура

```
apps/server/src/api/
├── docs.rs          # OpenAPI спецификация и типы
├── agents.rs        # Эндпоинты агентов с utoipa макросами
├── plugins.rs       # Эндпоинты плагинов с utoipa макросами
└── ...

scripts/
├── generate-api-client.sh  # Генерация TypeScript клиента
└── ...

.spectral.yaml       # Конфигурация валидации OpenAPI
```

## 🚀 Использование

### 1. Документация API

После запуска сервера Swagger UI доступен по адресу:
```
http://localhost:8080/swagger-ui
```

Raw OpenAPI JSON:
```
http://localhost:8080/api-docs/openapi.json
```

### 2. Генерация TypeScript клиента

```bash
# Запустить сервер
./scripts/start.sh

# В другом терминале - сгенерировать клиент
./scripts/generate-api-client.sh
```

Сгенерированный клиент будет находиться в `web/src/api/generated/`.

### 3. Использование в Vue

```typescript
import { AgentsApi, Configuration } from '@/api/generated';

// Создать API клиент
const api = new AgentsApi(new Configuration({
  basePath: ''  // Относительный путь
}));

// Типобезопасные вызовы
const agents = await api.listAgents();
const response = await api.sendCommand(agentId, {
  command: 'GetSystemInfo',
  type: 'GetSystemInfo'
});
```

## 📝 Добавление новых эндпоинтов

### На сервере (Rust)

```rust
use crate::api::docs::{ErrorResponse, YourResponseType};

/// Описание эндпоинта
#[utoipa::path(
    get,  // или post, put, delete
    path = "/your/path",
    tag = "category",
    params(
        ("param_name" = String, Path, description = "Описание")
    ),
    request_body = YourRequestType,  // Для POST/PUT
    responses(
        (status = 200, description = "Успех", body = YourResponseType),
        (status = 404, description = "Не найдено", body = ErrorResponse),
        (status = 500, description = "Ошибка сервера", body = ErrorResponse)
    )
)]
pub async fn your_handler(
    State(state): State<Arc<AppState>>,
    Path(param): Path<String>,
) -> Result<Json<YourResponseType>, StatusCode> {
    // Реализация
}
```

### Добавление в OpenAPI спецификацию

В `apps/server/src/api/docs.rs`:

```rust
#[openapi(
    paths(
        // ... существующие пути
        super::your_module::your_handler,  // Добавить новый
    ),
    components(
        schemas(
            // ... существующие типы
            YourRequestType,
            YourResponseType,
        )
    ),
)]
pub struct ApiDoc;
```

## 🔍 Валидация OpenAPI

Установка:
```bash
npm install -g @stoplight/spectral-cli
```

Проверка:
```bash
spectral lint http://localhost:8080/api-docs/openapi.json
```

Конфигурация в `.spectral.yaml`:
- Валидация структуры OpenAPI
- Проверка именования путей (kebab-case)
- Обязательные описания для операций

## 🔄 CI/CD Интеграция

Рекомендуется добавить в pipeline:

```yaml
# .github/workflows/api-check.yml
name: API Consistency Check

on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Start server
        run: |
          cargo build -p mini_msp_server
          ./target/debug/server &
          sleep 5
      
      - name: Validate OpenAPI
        run: |
          npm install -g @stoplight/spectral-cli
          spectral lint http://localhost:8080/api-docs/openapi.json
      
      - name: Check TypeScript generation
        run: |
          npm install -g @openapitools/openapi-generator-cli
          ./scripts/generate-api-client.sh
```

## 🎯 Преимущества подхода

1. **Единый источник правды** - API документация генерируется из кода
2. **Типобезопасность** - TypeScript клиент с полными типами
3. **Актуальность** - Документация всегда синхронизирована с кодом
4. **Валидация** - Автоматическая проверка структуры API
5. **UX** - Интерактивная документация Swagger UI

## 📚 Дополнительные инструменты

| Инструмент | Назначение |
|------------|-----------|
| `utoipa` | Генерация OpenAPI из Rust макросов |
| `utoipa-swagger-ui` | Встраивание Swagger UI в Axum |
| `spectral` | Валидация OpenAPI спецификации |
| `openapi-generator` | Генерация клиентов для разных языков |

## 🔗 Полезные ссылки

- [utoipa docs](https://docs.rs/utoipa/)
- [OpenAPI Specification](https://swagger.io/specification/)
- [Spectral Documentation](https://docs.stoplight.io/docs/spectral/)
