#ifndef WATCHERS_MANAGER_PLATFORM_H
#define WATCHERS_MANAGER_PLATFORM_H

#include "../include/plugin_interface_common.h"
#include "../include/semantic_types.h"
#include <stdint.h>
#include <stdbool.h>

#ifdef _WIN32
#include <windows.h>
#endif

// =============================================================================
// 👁️ WATCHER TYPES AND STRUCTURES
// =============================================================================

// Watcher event types
typedef enum {
    WATCHER_EVENT_UNKNOWN = 0,
    WATCHER_EVENT_FILE_CREATED = 1,
    WATCHER_EVENT_FILE_DELETED = 2,
    WATCHER_EVENT_FILE_MODIFIED = 3,
    WATCHER_EVENT_FILE_RENAMED = 4,
    WATCHER_EVENT_DIRECTORY_CREATED = 5,
    WATCHER_EVENT_DIRECTORY_DELETED = 6,
    WATCHER_EVENT_DIRECTORY_MODIFIED = 7
} watcher_event_type_t;

// Watcher event structure
typedef struct {
    watcher_event_type_t type;
    char filename[512];
    timestamp_t timestamp;
    uint32_t file_size;
    char old_name[512]; // For rename events
} watcher_event_t;

// Platform-specific data structures
#ifdef _WIN32
#include <windows.h>
typedef struct {
    HANDLE directory_handle;
    OVERLAPPED overlap;
    BYTE* buffer;
    DWORD buffer_size;
} windows_watcher_data_t;
#else
#include <sys/inotify.h>
#include <unistd.h>
typedef struct {
    int inotify_fd;
    int watch_descriptor;
} linux_watcher_data_t;
#endif

// Watcher context structure
typedef struct {
    char watch_path[512];
    bool is_active;
    void* platform_data;
    uint32_t event_count;
    timestamp_t start_time;
} watcher_context_t;

// Watcher statistics
typedef struct {
    uint32_t total_events;
    uint32_t events_last_hour;
    uint32_t events_last_day;
    uint32_t events_dropped;
    timestamp_t manager_start_time;
    uint32_t active_watchers;
} watcher_statistics_t;

// =============================================================================
// 🔧 PLATFORM-SPECIFIC FUNCTION DECLARATIONS
// =============================================================================

#ifdef _WIN32
plugin_result_t windows_init_watcher(watcher_context_t* context);
plugin_result_t windows_start_watching(watcher_context_t* context);
plugin_result_t windows_process_events(watcher_context_t* context, watcher_event_t* events, size_t* event_count);
plugin_result_t windows_stop_watching(watcher_context_t* context);
plugin_result_t windows_cleanup_watcher(watcher_context_t* context);
#define platform_init_watcher windows_init_watcher
#define platform_start_watching windows_start_watching
#define platform_process_events windows_process_events
#define platform_stop_watching windows_stop_watching
#define platform_cleanup_watcher windows_cleanup_watcher
#else
plugin_result_t linux_init_watcher(watcher_context_t* context);
plugin_result_t linux_start_watching(watcher_context_t* context);
plugin_result_t linux_process_events(watcher_context_t* context, watcher_event_t* events, size_t* event_count);
plugin_result_t linux_stop_watching(watcher_context_t* context);
plugin_result_t linux_cleanup_watcher(watcher_context_t* context);
#define platform_init_watcher linux_init_watcher
#define platform_start_watching linux_start_watching
#define platform_process_events linux_process_events
#define platform_stop_watching linux_stop_watching
#define platform_cleanup_watcher linux_cleanup_watcher
#endif

#endif // WATCHERS_MANAGER_PLATFORM_H
