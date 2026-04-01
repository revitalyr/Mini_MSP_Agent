/**
 * @file file_listener_plugin.c
 * @brief File Listener Plugin for Mini MSP Agent
 * 
 * Provides real-time file content monitoring and change detection
 * including file append operations and content modifications.
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
#include <tchar.h>
#else
#include <sys/inotify.h>
#include <unistd.h>
#include <errno.h>
#include <sys/stat.h>
#endif

// Plugin information
static plugin_info_t file_listener_plugin_info = {
    .name = "file_listener",
    .version = "1.0.0",
    .description = "Monitors file content changes and append operations in real-time"
};

/**
 * @brief File change event structure
 */
typedef struct {
    char file_path[512];
    uint64_t timestamp;
    uint32_t change_type;
    uint64_t old_size;
    uint64_t new_size;
    char* added_content;
    size_t added_content_size;
    char* removed_content;
    size_t removed_content_size;
} file_change_event_t;

/**
 * @brief File listener configuration
 */
typedef struct {
    char file_path[512];
    bool monitor_content;
    bool track_size_changes;
    bool capture_diffs;
    uint32_t poll_interval_ms;
    uint64_t max_content_capture;
} file_listener_config_t;

/**
 * @brief File state tracking
 */
typedef struct {
    char file_path[512];
    uint64_t last_size;
    time_t last_modified;
    char* last_content;
    size_t content_size;
    bool is_active;
} file_state_t;

// Plugin state
static bool listener_active = false;
static file_listener_config_t* listener_configs = NULL;
static size_t listener_count = 0;
static file_state_t* file_states = NULL;
static size_t file_state_count = 0;
static file_change_event_t* change_buffer = NULL;
static size_t change_buffer_size = 0;
static size_t change_buffer_index = 0;

#ifdef _WIN32
static HANDLE* file_handles = NULL;
static HANDLE poll_timer = NULL;
#else
static int inotify_fd = -1;
static int* watch_descriptors = NULL;
#endif

// Plugin implementation
static bool file_listener_init(void) {
#ifdef _WIN32
    // Initialize Windows file monitoring
    listener_active = false;
#else
    // Initialize Linux inotify
    inotify_fd = inotify_init();
    if (inotify_fd == -1) {
        return false;
    }
    listener_active = false;
#endif
    return true;
}

static void file_listener_cleanup(void) {
    file_listener_stop_monitoring();
    
    if (listener_configs) {
        free(listener_configs);
        listener_configs = NULL;
    }
    listener_count = 0;
    
    if (file_states) {
        for (size_t i = 0; i < file_state_count; i++) {
            if (file_states[i].last_content) {
                free(file_states[i].last_content);
            }
        }
        free(file_states);
        file_states = NULL;
    }
    file_state_count = 0;
    
    if (change_buffer) {
        for (size_t i = 0; i < change_buffer_index; i++) {
            if (change_buffer[i].added_content) {
                free(change_buffer[i].added_content);
            }
            if (change_buffer[i].removed_content) {
                free(change_buffer[i].removed_content);
            }
        }
        free(change_buffer);
        change_buffer = NULL;
    }
    change_buffer_size = 0;
    change_buffer_index = 0;
    
#ifdef _WIN32
    if (file_handles) {
        for (size_t i = 0; i < listener_count; i++) {
            if (file_handles[i] != INVALID_HANDLE_VALUE) {
                CloseHandle(file_handles[i]);
            }
        }
        free(file_handles);
        file_handles = NULL;
    }
    
    if (poll_timer) {
        CloseHandle(poll_timer);
        poll_timer = NULL;
    }
#else
    if (watch_descriptors) {
        for (size_t i = 0; i < listener_count; i++) {
            if (watch_descriptors[i] != -1) {
                inotify_rm_watch(inotify_fd, watch_descriptors[i]);
            }
        }
        free(watch_descriptors);
        watch_descriptors = NULL;
    }
    
    if (inotify_fd != -1) {
        close(inotify_fd);
        inotify_fd = -1;
    }
#endif
}

static plugin_info_t* file_listener_get_plugin_info(void) {
    return &file_listener_plugin_info;
}

static bool file_listener_get_system_metrics(system_metrics_t* metrics) {
    // Not applicable for file listener plugin
    return false;
}

static bool file_listener_get_processes(process_info_t** processes, size_t* count) {
    // Not applicable for file listener plugin
    return false;
}

static bool file_listener_execute_command(const char* command, command_result_t* result) {
    // Not applicable for file listener plugin
    return false;
}

static bool file_listener_read_file(const char* path, file_content_t* content) {
    // Not applicable for file listener plugin
    return false;
}

static bool file_listener_get_system_info(system_info_t* info) {
    // Not applicable for file listener plugin
    return false;
}

/**
 * @brief Read file content with size limit
 */
static char* read_file_content_limited(const char* path, size_t* size, size_t max_size) {
    FILE* fp = fopen(path, "rb");
    if (!fp) {
        *size = 0;
        return NULL;
    }
    
    // Get file size
    fseek(fp, 0, SEEK_END);
    long file_size = ftell(fp);
    fseek(fp, 0, SEEK_SET);
    
    if (file_size < 0) {
        fclose(fp);
        *size = 0;
        return NULL;
    }
    
    size_t read_size = (size_t)file_size;
    if (read_size > max_size) {
        read_size = max_size;
    }
    
    char* content = (char*)malloc(read_size + 1);
    if (!content) {
        fclose(fp);
        *size = 0;
        return NULL;
    }
    
    size_t bytes_read = fread(content, 1, read_size, fp);
    content[bytes_read] = '\0';
    
    fclose(fp);
    *size = bytes_read;
    return content;
}

/**
 * @brief Calculate simple diff between two strings
 */
static void calculate_simple_diff(const char* old_content, size_t old_size,
                               const char* new_content, size_t new_size,
                               char** added, size_t* added_size,
                               char** removed, size_t* removed_size) {
    *added = NULL;
    *added_size = 0;
    *removed = NULL;
    *removed_size = 0;
    
    // Simple implementation: find common prefix and suffix
    size_t common_prefix = 0;
    size_t min_len = old_size < new_size ? old_size : new_size;
    
    while (common_prefix < min_len && old_content[common_prefix] == new_content[common_prefix]) {
        common_prefix++;
    }
    
    size_t common_suffix = 0;
    while (common_suffix < min_len - common_prefix &&
           old_content[old_size - 1 - common_suffix] == new_content[new_size - 1 - common_suffix]) {
        common_suffix++;
    }
    
    // Extract added content
    size_t added_len = new_size - common_prefix - common_suffix;
    if (added_len > 0) {
        *added = (char*)malloc(added_len + 1);
        if (*added) {
            memcpy(*added, new_content + common_prefix, added_len);
            (*added)[added_len] = '\0';
            *added_size = added_len;
        }
    }
    
    // Extract removed content
    size_t removed_len = old_size - common_prefix - common_suffix;
    if (removed_len > 0) {
        *removed = (char*)malloc(removed_len + 1);
        if (*removed) {
            memcpy(*removed, old_content + common_prefix, removed_len);
            (*removed)[removed_len] = '\0';
            *removed_size = removed_len;
        }
    }
}

/**
 * @brief Find file state by path
 */
static file_state_t* find_file_state(const char* path) {
    for (size_t i = 0; i < file_state_count; i++) {
        if (strcmp(file_states[i].file_path, path) == 0) {
            return &file_states[i];
        }
    }
    return NULL;
}

/**
 * @brief Add file to monitoring
 */
static bool add_file_listener(const char* path, const file_listener_config_t* config) {
    if (!path || !config) return false;
    
    file_listener_config_t* new_configs = (file_listener_config_t*)realloc(
        listener_configs, (listener_count + 1) * sizeof(file_listener_config_t));
    if (!new_configs) {
        return false;
    }
    
    listener_configs = new_configs;
    memcpy(&listener_configs[listener_count], config, sizeof(file_listener_config_t));
    strncpy(listener_configs[listener_count].file_path, path, 
           sizeof(listener_configs[listener_count].file_path) - 1);
    
    // Add file state tracking
    file_state_t* new_states = (file_state_t*)realloc(
        file_states, (file_state_count + 1) * sizeof(file_state_t));
    if (!new_states) {
        return false;
    }
    
    file_states = new_states;
    file_state_t* state = &file_states[file_state_count];
    memset(state, 0, sizeof(file_state_t));
    strncpy(state->file_path, path, sizeof(state->file_path) - 1);
    
    // Initialize file state
    struct stat st;
    if (stat(path, &st) == 0) {
        state->last_size = st.st_size;
        state->last_modified = st.st_mtime;
        
        if (config->monitor_content) {
            state->last_content = read_file_content_limited(
                path, &state->content_size, config->max_content_capture);
        }
    }
    
    state->is_active = true;
    file_state_count++;
    listener_count++;
    
    return true;
}

/**
 * @brief Start file monitoring
 */
static bool file_listener_start_monitoring(void) {
    if (listener_active) {
        return true; // Already monitoring
    }
    
    if (listener_count == 0) {
        return false; // No files to monitor
    }
    
#ifdef _WIN32
    // Allocate resources for Windows monitoring
    file_handles = (HANDLE*)malloc(listener_count * sizeof(HANDLE));
    if (!file_handles) {
        return false;
    }
    
    // Open file handles for monitoring
    for (size_t i = 0; i < listener_count; i++) {
        file_handles[i] = CreateFileA(
            listener_configs[i].file_path,
            FILE_READ_ATTRIBUTES | FILE_READ_DATA,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            NULL,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OVERLAPPED,
            NULL
        );
        
        if (file_handles[i] == INVALID_HANDLE_VALUE) {
            file_handles[i] = NULL; // Will be skipped in monitoring
        }
    }
    
    // Create polling timer
    poll_timer = CreateWaitableTimer(NULL, FALSE, NULL);
    if (!poll_timer) {
        return false;
    }
#else
    // Allocate watch descriptors for Linux
    watch_descriptors = (int*)malloc(listener_count * sizeof(int));
    if (!watch_descriptors) {
        return false;
    }
    
    // Add watches for each file
    for (size_t i = 0; i < listener_count; i++) {
        uint32_t mask = IN_MODIFY | IN_MOVE_SELF | IN_DELETE_SELF;
        
        watch_descriptors[i] = inotify_add_watch(inotify_fd, 
                                                listener_configs[i].file_path, mask);
        if (watch_descriptors[i] == -1) {
            watch_descriptors[i] = -1; // Will be skipped in monitoring
        }
    }
#endif
    
    listener_active = true;
    return true;
}

/**
 * @brief Stop file monitoring
 */
static bool file_listener_stop_monitoring(void) {
    if (!listener_active) {
        return true; // Not monitoring
    }
    
#ifdef _WIN32
    // Close file handles
    if (file_handles) {
        for (size_t i = 0; i < listener_count; i++) {
            if (file_handles[i] && file_handles[i] != INVALID_HANDLE_VALUE) {
                CloseHandle(file_handles[i]);
                file_handles[i] = INVALID_HANDLE_VALUE;
            }
        }
    }
    
    if (poll_timer) {
        CloseHandle(poll_timer);
        poll_timer = NULL;
    }
#else
    // Remove inotify watches
    if (watch_descriptors) {
        for (size_t i = 0; i < listener_count; i++) {
            if (watch_descriptors[i] != -1) {
                inotify_rm_watch(inotify_fd, watch_descriptors[i]);
                watch_descriptors[i] = -1;
            }
        }
    }
#endif
    
    listener_active = false;
    return true;
}

/**
 * @brief Get file change events
 */
static bool get_file_change_events(file_change_event_t** events, size_t* count, uint32_t max_events) {
    if (!events || !count) return false;
    
    *events = NULL;
    *count = 0;
    
    if (!listener_active || change_buffer_index == 0) {
        return true; // No events
    }
    
    size_t events_to_return = change_buffer_index;
    if (events_to_return > max_events) {
        events_to_return = max_events;
    }
    
    *events = (file_change_event_t*)malloc(events_to_return * sizeof(file_change_event_t));
    if (!*events) {
        return false;
    }
    
    memcpy(*events, change_buffer, events_to_return * sizeof(file_change_event_t));
    *count = events_to_return;
    
    // Clear returned events from buffer
    if (events_to_return < change_buffer_index) {
        memmove(change_buffer, 
               change_buffer + events_to_return, 
               (change_buffer_index - events_to_return) * sizeof(file_change_event_t));
    }
    change_buffer_index -= events_to_return;
    
    return true;
}

/**
 * @brief Process file system events (internal function)
 */
static void process_file_changes(void) {
    if (!listener_active) return;
    
    for (size_t i = 0; i < file_state_count; i++) {
        file_state_t* state = &file_states[i];
        if (!state->is_active) continue;
        
        const file_listener_config_t* config = NULL;
        for (size_t j = 0; j < listener_count; j++) {
            if (strcmp(listener_configs[j].file_path, state->file_path) == 0) {
                config = &listener_configs[j];
                break;
            }
        }
        
        if (!config) continue;
        
        // Check file status
        struct stat st;
        if (stat(state->file_path, &st) == 0) {
            bool has_changes = false;
            file_change_event_t event = {0};
            
            // Check for size changes
            if (config->track_size_changes && st.st_size != state->last_size) {
                has_changes = true;
                event.change_type |= 0x01; // SIZE_CHANGED
                event.old_size = state->last_size;
                event.new_size = st.st_size;
            }
            
            // Check for content changes
            if (config->monitor_content && st.st_mtime != state->last_modified) {
                has_changes = true;
                event.change_type |= 0x02; // CONTENT_CHANGED
                
                if (config->capture_diffs && state->last_content) {
                    size_t new_content_size;
                    char* new_content = read_file_content_limited(
                        state->file_path, &new_content_size, config->max_content_capture);
                    
                    if (new_content) {
                        calculate_simple_diff(
                            state->last_content, state->content_size,
                            new_content, new_content_size,
                            &event.added_content, &event.added_content_size,
                            &event.removed_content, &event.removed_content_size);
                        
                        free(new_content);
                    }
                    
                    // Update stored content
                    free(state->last_content);
                    state->last_content = read_file_content_limited(
                        state->file_path, &state->content_size, config->max_content_capture);
                }
            }
            
            if (has_changes) {
                strncpy(event.file_path, state->file_path, sizeof(event.file_path) - 1);
                event.timestamp = time(NULL);
                
                // Add to change buffer
                if (change_buffer_index < change_buffer_size) {
                    memcpy(&change_buffer[change_buffer_index], &event, sizeof(file_change_event_t));
                    change_buffer_index++;
                    if (change_buffer_index >= change_buffer_size) {
                        change_buffer_index = 0; // Wrap around
                    }
                }
            }
            
            // Update state
            state->last_size = st.st_size;
            state->last_modified = st.st_mtime;
        }
    }
}

static void file_listener_free_memory(void* ptr) {
    if (ptr) {
        free(ptr);
    }
}

// Plugin interface
static plugin_interface_t file_listener_interface = {
    .get_plugin_info = file_listener_get_plugin_info,
    .init = file_listener_init,
    .cleanup = file_listener_cleanup,
    .get_system_metrics = file_listener_get_system_metrics,
    .get_processes = file_listener_get_processes,
    .execute_command = file_listener_execute_command,
    .read_file = file_listener_read_file,
    .get_system_info = file_listener_get_system_info,
    .free_memory = file_listener_free_memory
};

// Plugin entry point
PLUGIN_EXPORT plugin_interface_t* PLUGIN_CALL get_plugin_interface(void) {
    return &file_listener_interface;
}
