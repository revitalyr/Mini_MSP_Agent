#include <filesystem>
#include <chrono>
#include <string>
#include <vector>
#include <cstring>
#include "../include/plugin_interface.h"

#ifdef _WIN32
#include <windows.h>
#define STRDUP _strdup
#else
#define STRDUP strdup
#endif

namespace fs = std::filesystem;

// Хелпер для получения текущего времени в мс
uint64_t current_timestamp_ms() {
    return std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::system_clock::now().time_since_epoch()
    ).count();
}

extern "C" {

directory_info_data_t* PLUGIN_CALL get_directory_info_data(const char* path, bool recursive, bool show_hidden, uint32_t max_depth) {
    auto* stats = (directory_info_data_t*)malloc(sizeof(directory_info_data_t));
    if (!stats) return nullptr;
    memset(stats, 0, sizeof(directory_info_data_t));

    try {
        fs::path p(path);
        if (!fs::exists(p) || !fs::is_directory(p)) return stats;

        stats->m_path = STRDUP(path);
        
        auto iterate = [&](auto& it) {
            for (const auto& entry : it) {
                // Проверка на максимальную глубину (упрощенно)
                if (it.depth() > (int)max_depth) continue;

                bool is_hidden = false;
#ifdef _WIN32
                DWORD attrs = GetFileAttributesA(entry.path().string().c_str());
                is_hidden = (attrs != INVALID_FILE_ATTRIBUTES && (attrs & FILE_ATTRIBUTE_HIDDEN));
#else
                is_hidden = entry.path().filename().string()[0] == '.';
#endif

                if (is_hidden && !show_hidden) continue;

                if (entry.is_directory()) {
                    stats->m_total_directories++;
                    if (is_hidden) stats->m_hidden_directories++;
                } else if (entry.is_regular_file()) {
                    stats->m_total_files++;
                    stats->m_total_size_bytes += entry.file_size();
                    if (is_hidden) stats->m_hidden_files++;
                }
            }
        };

        if (recursive) {
            fs::recursive_directory_iterator it(p, fs::directory_options::skip_permission_denied);
            iterate(it);
        } else {
            fs::directory_iterator it(p, fs::directory_options::skip_permission_denied);
            // directory_iterator не имеет .depth(), используем обертку или простое условие
            for (const auto& entry : it) {
                if (entry.is_directory()) stats->m_total_directories++;
                else stats->m_total_files++;
            }
        }

        stats->m_scan_timestamp = current_timestamp_ms();
        stats->m_scan_progress = 100;

    } catch (const std::exception&) {
        // Логирование ошибки
    }

    return stats;
}

event_data_t* PLUGIN_CALL get_event_data(const char* path) {
    auto* data = (event_data_t*)malloc(sizeof(event_data_t));
    if (!data) return nullptr;
    
    data->m_path = STRDUP(path);
    data->m_events_count = 42; // Пример статических данных
    data->m_buffer_usage = 15;
    strncpy(data->m_last_event, "FileModified", 64);
    data->m_timestamp = current_timestamp_ms();
    
    return data;
}

watchers_data_t* PLUGIN_CALL get_watchers_data(void) {
    auto* data = (watchers_data_t*)malloc(sizeof(watchers_data_t));
    if (!data) return nullptr;

    data->m_active_watchers = 5;
    data->m_total_notifications = 128;
    data->m_cpu_usage = 0.5f;
    data->m_memory_usage_kb = 2048;

    return data;
}

file_reader_data_t* PLUGIN_CALL get_file_reader_data(const char* path) {
    auto* data = (file_reader_data_t*)malloc(sizeof(file_reader_data_t));
    if (!data) return nullptr;

    data->m_path = STRDUP(path);
    data->m_size = fs::exists(path) ? fs::file_size(path) : 0;
    strncpy(data->m_encoding, "UTF-8", 32);
    data->m_is_locked = false;
    data->m_last_access = current_timestamp_ms();

    return data;
}

void PLUGIN_CALL free_memory(void* ptr) {
    if (!ptr) return;
    // В реальном плагине нужно проверять тип структуры, 
    // чтобы освободить вложенные строки m_path перед освобождением самого указателя.
    // Для примера освобождаем просто блок:
    free(ptr);
}

} // extern "C"