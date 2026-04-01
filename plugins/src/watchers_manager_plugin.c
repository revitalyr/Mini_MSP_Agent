/**
 * @file watchers_manager_plugin.c
 * @brief Watchers Manager Plugin for Mini MSP Agent
 * 
 * Provides centralized management for multiple file and folder watchers
 * with unified event handling and configuration management.
 * 
 * @author Mini MSP Agent Team
 * @version 1.0.0
 * @date 2026
 */

#include "../include/plugin_interface.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <time.h>

#ifdef _WIN32
#include <windows.h>
#else
#include <sys/inotify.h>
#include <unistd.h>
#include <errno.h>
#endif

// Plugin information
static plugin_info_t watchers_manager_plugin_info = {
    .name = "watchers_manager",
    .version = "1.0.0",
    .description = "Centralized management for file and folder watchers"
};

/**
 * @brief Watcher types
 */
typedef enum {
    WATCHER_TYPE_FILE = 1,
    WATCHER_TYPE_FOLDER = 2,
    WATCHER_TYPE_VOLUME = 3
} watcher_type_t;

/**
 * @brief Watcher status
 */
typedef enum {
    WATCHER_STATUS_INACTIVE = 0,
    WATCHER_STATUS_ACTIVE = 1,
    WATCHER_STATUS_ERROR = 2,
    WATCHER_STATUS_PAUSED = 3
} watcher_status_t;

/**
 * @brief Unified event structure
 */
typedef struct {
    uint64_t event_id;
    uint32_t watcher_id;
    watcher_type_t watcher_type;
    char event_type[32];
    char source_path[512];
    char destination_path[512];
    uint64_t timestamp;
    char description[256];
    uint32_t event_data_size;
    void* event_data;
} unified_event_t;

/**
 * @brief Watcher configuration
 */
typedef struct {
    uint32_t watcher_id;
    watcher_type_t type;
    char target_path[512];
    bool recursive;
    bool monitor_content;
    bool track_size_changes;
    uint32_t buffer_size;
    char event_filters[256];
    uint32_t priority;
    watcher_status_t status;
    time_t created_time;
    time_t last_activity;
    uint64_t events_processed;
    uint64_t events_dropped;
} watcher_config_t;

/**
 * @brief Watcher statistics
 */
typedef struct {
    uint32_t total_watchers;
    uint32_t active_watchers;
    uint32_t inactive_watchers;
    uint32_t error_watchers;
    uint64_t total_events;
    uint64_t events_last_hour;
    uint64_t events_last_day;
    uint64_t events_dropped;
    time_t manager_start_time;
} watcher_statistics_t;

// Plugin state
static watcher_config_t* watcher_configs = NULL;
static size_t watcher_count = 0;
static unified_event_t* event_queue = NULL;
static size_t event_queue_size = 0;
static size_t event_queue_head = 0;
static size_t event_queue_tail = 0;
static uint32_t next_watcher_id = 1;
static uint64_t next_event_id = 1;
static bool manager_active = false;
static watcher_statistics_t manager_stats;

#ifdef _WIN32
static HANDLE* watcher_handles = NULL;
static HANDLE manager_thread = NULL;
static CRITICAL_SECTION manager_lock;
#else
static int inotify_fd = -1;
static int* watch_descriptors = NULL;
static pthread_t manager_thread;
static pthread_mutex_t manager_lock;
#endif

// Plugin implementation
static bool watchers_manager_init(void) {
    memset(&manager_stats, 0, sizeof(watcher_statistics_t));
    manager_stats.manager_start_time = time(NULL);
    
#ifdef _WIN32
    InitializeCriticalSection(&manager_lock);
#else
    pthread_mutex_init(&manager_lock, NULL);
#endif
    
    return true;
}

static void watchers_manager_cleanup(void) {
    watchers_manager_stop_all();
    
    if (watcher_configs) {
        free(watcher_configs);
        watcher_configs = NULL;
    }
    watcher_count = 0;
    
    if (event_queue) {
        free(event_queue);
        event_queue = NULL;
    }
    event_queue_size = 0;
    
#ifdef _WIN32
    if (watcher_handles) {
        free(watcher_handles);
        watcher_handles = NULL;
    }
    
    if (manager_thread) {
        WaitForSingleObject(manager_thread, INFINITE);
        CloseHandle(manager_thread);
        manager_thread = NULL;
    }
    
    DeleteCriticalSection(&manager_lock);
#else
    if (watch_descriptors) {
        free(watch_descriptors);
        watch_descriptors = NULL;
    }
    
    if (manager_thread) {
        pthread_join(manager_thread, NULL);
        memset(&manager_thread, 0, sizeof(pthread_t));
    }
    
    pthread_mutex_destroy(&manager_lock);
#endif
}

static plugin_info_t* watchers_manager_get_plugin_info(void) {
    return &watchers_manager_plugin_info;
}

static bool watchers_manager_get_system_metrics(system_metrics_t* metrics) {
    // Not applicable for watchers manager plugin
    return false;
}

static bool watchers_manager_get_processes(process_info_t** processes, size_t* count) {
    // Not applicable for watchers manager plugin
    return false;
}

static bool watchers_manager_execute_command(const char* command, command_result_t* result) {
    // Not applicable for watchers manager plugin
    return false;
}

static bool watchers_manager_read_file(const char* path, file_content_t* content) {
    // Not applicable for watchers manager plugin
    return false;
}

static bool watchers_manager_get_system_info(system_info_t* info) {
    // Not applicable for watchers manager plugin
    return false;
}

/**
 * @brief Find watcher by ID
 */
static watcher_config_t* find_watcher_by_id(uint32_t watcher_id) {
    for (size_t i = 0; i < watcher_count; i++) {
        if (watcher_configs[i].watcher_id == watcher_id) {
            return &watcher_configs[i];
        }
    }
    return NULL;
}

/**
 * @brief Add event to queue
 */
static bool add_event_to_queue(const unified_event_t* event) {
    if (!event || event_queue_size == 0) {
        return false;
    }
    
#ifdef _WIN32
    EnterCriticalSection(&manager_lock);
#else
    pthread_mutex_lock(&manager_lock);
#endif
    
    // Check if queue is full
    if ((event_queue_tail + 1) % event_queue_size == event_queue_head) {
        manager_stats.events_dropped++;
#ifdef _WIN32
        LeaveCriticalSection(&manager_lock);
#else
        pthread_mutex_unlock(&manager_lock);
#endif
        return false; // Queue full
    }
    
    // Add event to queue
    memcpy(&event_queue[event_queue_tail], event, sizeof(unified_event_t));
    event_queue_tail = (event_queue_tail + 1) % event_queue_size;
    
    manager_stats.total_events++;
    manager_stats.events_last_hour++;
    manager_stats.events_last_day++;
    
#ifdef _WIN32
    LeaveCriticalSection(&manager_lock);
#else
    pthread_mutex_unlock(&manager_lock);
#endif
    
    return true;
}

/**
 * @brief Get events from queue
 */
static bool get_events_from_queue(unified_event_t** events, size_t* count, uint32_t max_events) {
    if (!events || !count) return false;
    
    *events = NULL;
    *count = 0;
    
#ifdef _WIN32
    EnterCriticalSection(&manager_lock);
#else
    pthread_mutex_lock(&manager_lock);
#endif
    
    size_t available_events = 0;
    size_t temp_head = event_queue_head;
    
    // Count available events
    while (temp_head != event_queue_tail && available_events < max_events) {
        available_events++;
        temp_head = (temp_head + 1) % event_queue_size;
    }
    
    if (available_events == 0) {
#ifdef _WIN32
        LeaveCriticalSection(&manager_lock);
#else
        pthread_mutex_unlock(&manager_lock);
#endif
        return true; // No events
    }
    
    // Allocate memory for events
    *events = (unified_event_t*)malloc(available_events * sizeof(unified_event_t));
    if (!*events) {
#ifdef _WIN32
        LeaveCriticalSection(&manager_lock);
#else
        pthread_mutex_unlock(&manager_lock);
#endif
        return false;
    }
    
    // Copy events from queue
    for (size_t i = 0; i < available_events; i++) {
        memcpy(&(*events)[i], &event_queue[event_queue_head], sizeof(unified_event_t));
        event_queue_head = (event_queue_head + 1) % event_queue_size;
    }
    
    *count = available_events;
    
#ifdef _WIN32
    LeaveCriticalSection(&manager_lock);
#else
    pthread_mutex_unlock(&manager_lock);
#endif
    
    return true;
}

/**
 * @brief Add new watcher
 */
static uint32_t add_watcher(const watcher_config_t* config) {
    if (!config) return 0;
    
    watcher_config_t* new_configs = (watcher_config_t*)realloc(
        watcher_configs, (watcher_count + 1) * sizeof(watcher_config_t));
    if (!new_configs) {
        return 0;
    }
    
    watcher_configs = new_configs;
    watcher_config_t* new_watcher = &watcher_configs[watcher_count];
    
    memcpy(new_watcher, config, sizeof(watcher_config_t));
    new_watcher->watcher_id = next_watcher_id++;
    new_watcher->status = WATCHER_STATUS_INACTIVE;
    new_watcher->created_time = time(NULL);
    new_watcher->last_activity = 0;
    new_watcher->events_processed = 0;
    new_watcher->events_dropped = 0;
    
    watcher_count++;
    
    // Update statistics
    manager_stats.total_watchers++;
    manager_stats.inactive_watchers++;
    
    return new_watcher->watcher_id;
}

/**
 * @brief Remove watcher
 */
static bool remove_watcher(uint32_t watcher_id) {
    watcher_config_t* watcher = find_watcher_by_id(watcher_id);
    if (!watcher) {
        return false;
    }
    
    // Stop watcher if active
    if (watcher->status == WATCHER_STATUS_ACTIVE) {
        // Implementation would stop the actual watching
        watcher->status = WATCHER_STATUS_INACTIVE;
    }
    
    // Remove from array (simplified - just mark as removed)
    watcher->status = WATCHER_STATUS_INACTIVE;
    
    // Update statistics
    if (watcher->status == WATCHER_STATUS_ACTIVE) {
        manager_stats.active_watchers--;
    } else if (watcher->status == WATCHER_STATUS_ERROR) {
        manager_stats.error_watchers--;
    } else {
        manager_stats.inactive_watchers--;
    }
    
    manager_stats.total_watchers--;
    
    return true;
}

/**
 * @brief Start watcher
 */
static bool start_watcher(uint32_t watcher_id) {
    watcher_config_t* watcher = find_watcher_by_id(watcher_id);
    if (!watcher || watcher->status == WATCHER_STATUS_ACTIVE) {
        return false;
    }
    
    // Implementation would start actual watching based on type
    watcher->status = WATCHER_STATUS_ACTIVE;
    watcher->last_activity = time(NULL);
    
    // Update statistics
    manager_stats.active_watchers++;
    if (watcher->status == WATCHER_STATUS_INACTIVE) {
        manager_stats.inactive_watchers--;
    } else if (watcher->status == WATCHER_STATUS_ERROR) {
        manager_stats.error_watchers--;
    }
    
    return true;
}

/**
 * @brief Stop watcher
 */
static bool stop_watcher(uint32_t watcher_id) {
    watcher_config_t* watcher = find_watcher_by_id(watcher_id);
    if (!watcher || watcher->status != WATCHER_STATUS_ACTIVE) {
        return false;
    }
    
    // Implementation would stop actual watching
    watcher->status = WATCHER_STATUS_INACTIVE;
    
    // Update statistics
    manager_stats.active_watchers--;
    manager_stats.inactive_watchers++;
    
    return true;
}

/**
 * @brief Pause watcher
 */
static bool pause_watcher(uint32_t watcher_id) {
    watcher_config_t* watcher = find_watcher_by_id(watcher_id);
    if (!watcher || watcher->status != WATCHER_STATUS_ACTIVE) {
        return false;
    }
    
    watcher->status = WATCHER_STATUS_PAUSED;
    
    // Update statistics
    manager_stats.active_watchers--;
    manager_stats.inactive_watchers++;
    
    return true;
}

/**
 * @brief Resume watcher
 */
static bool resume_watcher(uint32_t watcher_id) {
    watcher_config_t* watcher = find_watcher_by_id(watcher_id);
    if (!watcher || watcher->status != WATCHER_STATUS_PAUSED) {
        return false;
    }
    
    watcher->status = WATCHER_STATUS_ACTIVE;
    watcher->last_activity = time(NULL);
    
    // Update statistics
    manager_stats.active_watchers++;
    manager_stats.inactive_watchers--;
    
    return true;
}

/**
 * @brief Get all watcher configurations
 */
static bool get_all_watchers(watcher_config_t** watchers, size_t* count) {
    if (!watchers || !count) return false;
    
    *watchers = watcher_configs;
    *count = watcher_count;
    return true;
}

/**
 * @brief Get watcher statistics
 */
static bool get_watcher_statistics(watcher_statistics_t* stats) {
    if (!stats) return false;
    
    // Update time-based statistics
    time_t current_time = time(NULL);
    time_t hours_ago = current_time - 3600;
    time_t days_ago = current_time - 86400;
    
    // Reset counters if needed (simplified)
    if (current_time - manager_stats.manager_start_time > 3600) {
        manager_stats.events_last_hour = 0;
    }
    
    if (current_time - manager_stats.manager_start_time > 86400) {
        manager_stats.events_last_day = 0;
    }
    
    memcpy(stats, &manager_stats, sizeof(watcher_statistics_t));
    return true;
}

/**
 * @brief Start all watchers
 */
static bool watchers_manager_start_all(void) {
    if (manager_active) {
        return true; // Already active
    }
    
    // Initialize event queue
    event_queue_size = 10000; // Configurable
    event_queue = (unified_event_t*)malloc(event_queue_size * sizeof(unified_event_t));
    if (!event_queue) {
        return false;
    }
    
    event_queue_head = 0;
    event_queue_tail = 0;
    
    // Start all inactive watchers
    for (size_t i = 0; i < watcher_count; i++) {
        if (watcher_configs[i].status == WATCHER_STATUS_INACTIVE) {
            start_watcher(watcher_configs[i].watcher_id);
        }
    }
    
    manager_active = true;
    return true;
}

/**
 * @brief Stop all watchers
 */
static bool watchers_manager_stop_all(void) {
    if (!manager_active) {
        return true; // Not active
    }
    
    // Stop all active watchers
    for (size_t i = 0; i < watcher_count; i++) {
        if (watcher_configs[i].status == WATCHER_STATUS_ACTIVE) {
            stop_watcher(watcher_configs[i].watcher_id);
        }
    }
    
    manager_active = false;
    return true;
}

static void watchers_manager_free_memory(void* ptr) {
    if (ptr) {
        free(ptr);
    }
}

// Plugin interface
static plugin_interface_t watchers_manager_interface = {
    .get_plugin_info = watchers_manager_get_plugin_info,
    .init = watchers_manager_init,
    .cleanup = watchers_manager_cleanup,
    .get_system_metrics = watchers_manager_get_system_metrics,
    .get_processes = watchers_manager_get_processes,
    .execute_command = watchers_manager_execute_command,
    .read_file = watchers_manager_read_file,
    .get_system_info = watchers_manager_get_system_info,
    .free_memory = watchers_manager_free_memory
};

// Plugin entry point
PLUGIN_EXPORT plugin_interface_t* PLUGIN_CALL get_plugin_interface(void) {
    return &watchers_manager_interface;
}
