#ifndef SEMANTIC_TYPES_H
#define SEMANTIC_TYPES_H

#include <stdint.h>
#include <stdbool.h>
#include "plugin_interface_common.h"

#ifdef __cplusplus
extern "C" {
#endif

// =============================================================================
// 🏷️ SEMANTIC TYPE ALIASES
// =============================================================================

// 📊 Size and count types
typedef uint64_t file_size_t;        // Размер файла в байтах
typedef uint64_t timestamp_t;        // Unix timestamp в миллисекундах
typedef uint32_t file_count_t;       // Количество файлов
typedef uint32_t call_count_t;       // Количество вызовов
typedef uint32_t error_count_t;      // Количество ошибок
typedef uint16_t path_length_t;      // Длина пути
typedef uint8_t  percentage_t;       // Процент (0-100)

// 🔤 String types with semantic meaning
typedef char* path_string_t;         // Путь к файлу/директории
typedef char* error_message_t;       // Сообщение об ошибке
typedef char* plugin_name_t;         // Имя плагина
typedef char* plugin_version_t;      // Версия плагина
typedef char* plugin_description_t;  // Описание плагина
typedef char* command_name_t;        // Имя команды
typedef char* command_params_t;     // Параметры команды

// 📋 Specialized structures
typedef struct {
    file_count_t m_total_files;      // Общее количество файлов
    file_count_t m_total_directories; // Общее количество директорий
    file_size_t m_total_size_bytes;  // Общий размер в байтах
    file_count_t m_hidden_files;    // Количество скрытых файлов
    file_count_t m_hidden_directories; // Количество скрытых директорий
    timestamp_t m_scan_timestamp;   // Время сканирования
    percentage_t m_scan_progress;   // Прогресс сканирования (0-100%)
} directory_stats_t;

typedef struct {
    path_string_t m_name;            // Имя файла/директории
    path_string_t m_full_path;        // Полный путь
    file_size_t m_size_bytes;        // Размер в байтах
    timestamp_t m_modification_time;  // Время модификации
    timestamp_t m_creation_time;      // Время создания
    bool m_is_hidden;                // Флаг скрытия
    bool m_is_directory;             // Флаг директории
    char m_permissions[16];          // Права доступа
} directory_entry_t;

typedef struct {
    plugin_name_t m_plugin_name;      // Имя плагина
    call_count_t m_successful_calls;  // Успешные вызовы
    error_count_t m_failed_calls;     // Неудачные вызовы
    timestamp_t m_last_call_time;    // Время последнего вызова
    file_size_t m_total_data_processed; // Всего обработано данных
} plugin_metrics_t;

typedef struct {
    error_message_t m_message;        // Сообщение об ошибке
    error_count_t m_error_code;      // Код ошибки
    timestamp_t m_timestamp;         // Время ошибки
    plugin_name_t m_plugin_source;   // Источник ошибки
} error_info_t;

typedef struct {
    command_name_t m_name;            // Имя команды
    command_params_t m_params;        // Параметры команды
    timestamp_t m_execution_time;     // Время выполнения
    plugin_result_t m_result;        // Результат выполнения
    error_info_t m_error_info;        // Информация об ошибке (если есть)
} command_execution_t;

// 📊 Buffer and data types
typedef struct {
    void* m_data;                     // Указатель на данные
    file_size_t m_size;              // Размер данных
    file_size_t m_capacity;           // Емкость буфера
    bool m_is_dynamic;               // Динамическое выделение
} data_buffer_t;

typedef struct {
    path_string_t m_base_path;        // Базовый путь
    file_count_t m_max_depth;        // Максимальная глубина
    bool m_recursive;                // Рекурсивный поиск
    char* m_file_pattern;            // Шаблон файла
    char* m_exclude_pattern;         // Шаблон исключения
} scan_config_t;

// 🔄 Event types
typedef struct {
    plugin_event_type_t m_type;       // Тип события
    plugin_name_t m_source_plugin;    // Источник плагина
    timestamp_t m_timestamp;         // Время события
    void* m_event_data;              // Данные события
    file_size_t m_data_size;         // Размер данных события
} plugin_event_info_t;

// 🎯 Configuration types
typedef struct {
    path_string_t m_config_file;      // Путь к файлу конфигурации
    bool m_auto_reload;              // Автоперезагрузка конфигурации
    timestamp_t m_last_modified;      // Время последней модификации
    void* m_config_data;             // Данные конфигурации
    file_size_t m_config_size;        // Размер конфигурации
} plugin_config_t;

// 📈 Performance metrics
typedef struct {
    timestamp_t m_start_time;         // Время начала
    timestamp_t m_end_time;           // Время окончания
    file_size_t m_bytes_processed;   // Обработано байт
    call_count_t m_operations_count;  // Количество операций
    percentage_t m_cpu_usage;        // Использование CPU
    file_size_t m_memory_usage;      // Использование памяти
} performance_metrics_t;

#ifdef __cplusplus
}
#endif

#endif // SEMANTIC_TYPES_H
