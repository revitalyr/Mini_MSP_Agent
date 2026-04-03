#include <filesystem>
#include <chrono>
#include <string>
#include <vector>
#include <cstring>
#include <turbojpeg.h>
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

sensor_data_t* PLUGIN_CALL get_sensor_data(void) {
    auto* data = (sensor_data_t*)malloc(sizeof(sensor_data_t));
    if (!data) return nullptr;
    
    data->m_temperature = 22.5f + (rand() % 100) / 50.0f;
    data->m_humidity = 45.0f + (rand() % 100) / 20.0f;
    data->m_pressure = 1013.25f;
    data->m_timestamp = current_timestamp_ms();
    return data;
}

camera_data_t* PLUGIN_CALL get_camera_data(void) {
    auto* data = (camera_data_t*)malloc(sizeof(camera_data_t));
    if (!data) return nullptr;
    
    data->m_width = 1920;
    data->m_height = 1080;
    data->m_fps = 30;
    strncpy(data->m_codec, "h264", 16);
    data->m_timestamp = current_timestamp_ms();
    return data;
}

video_frame_t* PLUGIN_CALL get_video_frame(void) {
    auto* frame = (video_frame_t*)malloc(sizeof(video_frame_t));
    if (!frame) return nullptr;
    memset(frame, 0, sizeof(video_frame_t));

    const uint32_t w = 320, h = 240;
    file_size_t raw_size = w * h * 3;
    std::vector<uint8_t> rgb_buffer(raw_size);

    // Генерация тестового изображения (градиент)
    timestamp_t ts = current_timestamp_ms();
    for (uint32_t i = 0; i < raw_size; i++) {
        rgb_buffer[i] = (uint8_t)(i + (uint32_t)ts) % 255;
    }

    // Сжатие в JPEG с помощью TurboJPEG
    tjhandle compressor = tjInitCompress();
    unsigned char* jpeg_buf = nullptr;
    unsigned long jpeg_size = 0;

    if (tjCompress2(compressor, rgb_buffer.data(), w, 0, h, TJPF_RGB,
                    &jpeg_buf, &jpeg_size, TJSAMP_420, 75, TJFLAG_FASTDCT) == 0) 
    {
        // Копируем сжатые данные в буфер, который будет передан Rust
        frame->m_data = (uint8_t*)malloc(jpeg_size);
        if (frame->m_data) {
            memcpy(frame->m_data, jpeg_buf, jpeg_size);
            frame->m_size = (file_size_t)jpeg_size;
        }
    }

    // Освобождаем временный буфер TurboJPEG и дескриптор
    tjFree(jpeg_buf);
    tjDestroy(compressor);

    frame->m_width = w;
    frame->m_height = h;
    frame->m_timestamp = ts;

    return frame;
}

processing_results_t* PLUGIN_CALL get_processing_results(void) {
    auto* data = (processing_results_t*)malloc(sizeof(processing_results_t));
    if (!data) return nullptr;
    
    strncpy(data->m_status, "Processing stream...", 64);
    data->m_load_index = 0.75f;
    data->m_processed_items = 1500;
    return data;
}

void PLUGIN_CALL free_memory(void* ptr) {
    if (!ptr) return;
    // В реальном плагине нужно проверять тип структуры, 
    // чтобы освободить вложенные строки m_path перед освобождением самого указателя.
    
    // Пример для video_frame_t:
    // video_frame_t* frame = (video_frame_t*)ptr;
    // if (frame->m_data) free(frame->m_data);
    // if (frame->m_path) free(frame->m_path);

    free(ptr);
}

} // extern "C"