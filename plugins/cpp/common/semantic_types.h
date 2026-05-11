#ifndef SEMANTIC_TYPES_H
#define SEMANTIC_TYPES_H

#include <stdint.h>
#include <stdbool.h>
#include "plugin_interface_common.h"
#include "type_aliases.h"
#include "constants.h"

#ifdef __cplusplus
extern "C" {
#endif

// =============================================================================
// MEMORY OWNERSHIP SEMANTICS
// =============================================================================
//
// CRITICAL: FFI boundary memory management rules
//
// [READONLY POINTERS - NOT OWNED]
// - PathString, ErrorMessage, PluginName, PluginVersion, etc.
//   These are const char* pointers that the RECEIVER must NOT free.
//   Memory is owned by the caller and must remain valid during the call.
//
// [OWNED POINTERS - MUST BE FREED]
// - string_buffer_t: The owner must free m_data using free() when done.
// - data_buffer_t: If m_is_dynamic=true, owner must free m_data.
// - video_frame_t: Owner must free m_data.
// - plugin_event_info_t: Owner must free m_event_data.
// - plugin_config_t: Owner must free m_config_data.
//
// [FIXED-SIZE ARRAYS - NO FREEING]
// - char m_permissions[MAX_PERMISSION_LEN]: Stack-allocated, no free needed
// - char m_last_event[MAX_EVENT_MSG_LEN]: Stack-allocated, no free needed
// - All other fixed-size char arrays: Stack-allocated, no free needed
//
// [THREAD SAFETY]
// - These structures are designed for pass-by-value semantics.
// - Avoid concurrent writes to data_buffer_t.m_data.
// - string_buffer_t is NOT thread-safe for concurrent writes.
//
// [BOUNDARY CHECKING]
// - Always verify m_length < m_capacity before writing to string_buffer_t
// - Always verify m_length < MAX_*_LEN before writing to fixed arrays
// - Use strncpy with explicit length limits for string operations
// =============================================================================

// =============================================================================
// LENGTH-AWARE STRING TYPES FOR FFI SAFETY
// =============================================================================

typedef struct {
    const char* m_data;              // Pointer to string data
    PathLength m_length;             // String length (without null terminator)
} string_view_t;                    // Readonly view - does NOT own memory

typedef struct {
    char* m_data;                    // Pointer to string data
    PathLength m_length;             // String length
    PathLength m_capacity;           // Allocated buffer capacity
} string_buffer_t;                   // Mutable buffer - owns memory

// =============================================================================
// SPECIALIZED STRUCTURES
// =============================================================================

typedef struct {
    FileCount m_total_files;         // Total number of files
    DirectoryCount m_total_directories; // Total number of directories
    FileSize m_total_size_bytes;     // Total size in bytes
    FileCount m_hidden_files;        // Number of hidden files
    FileCount m_hidden_directories;  // Number of hidden directories
    Timestamp m_scan_timestamp;      // Scan timestamp
    Percentage m_scan_progress;      // Scan progress (0-100%)
} directory_stats_t;

typedef struct {
    PathString m_name;               // File/directory name
    PathString m_full_path;          // Full path
    FileSize m_size_bytes;           // Size in bytes
    Timestamp m_modification_time;    // Modification time
    Timestamp m_creation_time;       // Creation time
    IsHidden m_is_hidden;            // Hidden flag
    IsDirectory m_is_directory;      // Directory flag
    char m_permissions[kMaxPermissionLen]; // Permissions (BOUNDARY: strncpy limit)
} directory_entry_t;

typedef struct {
    PathString m_path;
    FileCount m_total_files;
    DirectoryCount m_total_directories;
    FileSize m_total_size_bytes;
    FileCount m_hidden_files;
    FileCount m_hidden_directories;
    Timestamp m_scan_timestamp;
    Percentage m_scan_progress;
} directory_info_data_t;

typedef struct {
    PathString m_path;
    CallCount m_events_count;
    Percentage m_buffer_usage;
    char m_last_event[kMaxEventMsgLen]; // Last event (BOUNDARY: strncpy limit)
    Timestamp m_timestamp;
} event_data_t;

typedef struct {
    WatcherCount m_active_watchers;
    NotificationCount m_total_notifications;
    CpuUsage m_cpu_usage;
    MemorySize m_memory_usage_kb;
} watchers_data_t;

typedef struct {
    PathString m_path;
    FileSize m_size;
    char m_encoding[kMaxEncodingLen]; // Encoding (BOUNDARY: strncpy limit)
    IsLocked m_is_locked;
    Timestamp m_last_access;
} file_reader_data_t;

typedef struct {
    float m_temperature;
    float m_humidity;
    float m_pressure;
    Timestamp m_timestamp;
} sensor_data_t;

typedef struct {
    Width m_width;
    Height m_height;
    FrameRate m_fps;
    char m_codec[kMaxCodecNameLen]; // Codec (BOUNDARY: strncpy limit)
    Timestamp m_timestamp;
} camera_data_t;

typedef struct {
    char m_status[kMaxStatusLen]; // Status (BOUNDARY: strncpy limit)
    LoadIndex m_load_index;
    ItemCount m_processed_items;
} processing_results_t;

typedef struct {
    uint8_t* m_data;                 // Pointer to binary data (frame)
    FileSize m_size;                 // Data size
    Width m_width;
    Height m_height;
    Timestamp m_timestamp;
} video_frame_t;

typedef struct {
    PluginName m_plugin_name;        // Plugin name
    CallCount m_successful_calls;     // Successful calls
    ErrorCount m_failed_calls;        // Failed calls
    Timestamp m_last_call_time;      // Last call time
    FileSize m_total_data_processed; // Total data processed
} plugin_metrics_t;

typedef struct {
    ErrorMessage m_message;          // Error message
    ErrorCode m_error_code;           // Error code
    Timestamp m_timestamp;           // Error timestamp
    PluginName m_plugin_source;      // Error source plugin
} error_info_t;

typedef struct {
    CommandName m_name;               // Command name
    string_buffer_t m_params;        // Command parameters (owns memory)
    Timestamp m_execution_time;      // Execution time
    plugin_result_t m_result;        // Execution result
    error_info_t m_error_info;        // Error information (if any)
} command_execution_t;

// =============================================================================
// BUFFER AND DATA TYPES
// =============================================================================

typedef struct {
    void* m_data;                     // Pointer to data
    BufferSize m_size;               // Data size
    BufferSize m_capacity;           // Buffer capacity
    IsDynamic m_is_dynamic;          // Dynamic allocation flag
} data_buffer_t;

typedef struct {
    PathString m_base_path;          // Base path
    DepthLevel m_max_depth;           // Maximum depth
    Recursive m_recursive;           // Recursive search flag
    string_buffer_t m_file_pattern;   // File pattern (owns memory)
    string_buffer_t m_exclude_pattern; // Exclude pattern (owns memory)
} scan_config_t;

// =============================================================================
// EVENT TYPES
// =============================================================================

typedef struct {
    plugin_event_type_t m_type;       // Event type
    PluginName m_source_plugin;      // Source plugin
    Timestamp m_timestamp;           // Event timestamp
    void* m_event_data;              // Event data
    BufferSize m_data_size;          // Event data size
} plugin_event_info_t;

// =============================================================================
// CONFIGURATION TYPES
// =============================================================================

typedef struct {
    ConfigPath m_config_file;         // Configuration file path
    AutoReload m_auto_reload;         // Auto-reload configuration flag
    Timestamp m_last_modified;        // Last modification time
    void* m_config_data;              // Configuration data
    BufferSize m_config_size;         // Configuration size
} plugin_config_t;

// =============================================================================
// PERFORMANCE METRICS
// =============================================================================

typedef struct {
    Timestamp m_start_time;          // Start time
    Timestamp m_end_time;            // End time
    FileSize m_bytes_processed;      // Bytes processed
    CallCount m_operations_count;    // Number of operations
    Percentage m_cpu_usage;          // CPU usage
    MemorySize m_memory_usage;       // Memory usage
} performance_metrics_t;

#ifdef __cplusplus
}
#endif

#endif // SEMANTIC_TYPES_H
