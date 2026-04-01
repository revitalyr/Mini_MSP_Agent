/**
 * @file folder_watcher_plugin.c
 * @brief Folder Watcher Plugin for Mini MSP Agent
 * 
 * Provides real-time folder monitoring and change detection
 * including file creation, modification, deletion, and renaming.
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
static plugin_info_t folder_watcher_plugin_info = {
    .name = "folder_watcher",
    .version = "1.0.0",
    .description = "Monitors folder changes and events in real-time"
};

/**
 * @brief File event types
 */
typedef enum {
    FILE_EVENT_CREATED = 1,
    FILE_EVENT_MODIFIED = 2,
    FILE_EVENT_DELETED = 3,
    FILE_EVENT_RENAMED = 4,
    FILE_EVENT_ACCESS = 5
} file_event_type_t;

/**
 * @brief File event structure
 */
typedef struct {
    file_event_type_t event_type;
    char old_path[512];
    char new_path[512];
    uint64_t timestamp;
    uint32_t file_size;
    bool is_directory;
    char file_extension[32];
} file_event_t;

/**
 * @brief Watch configuration
 */
typedef struct {
    char path[512];
    bool recursive;
    bool watch_subdirectories;
    uint32_t buffer_size;
    bool notify_filters[6]; // Corresponds to file_event_type_t
} watch_config_t;

// Plugin state
static bool watching_active = false;
static watch_config_t* watch_configs = NULL;
static size_t watch_count = 0;
static file_event_t* event_buffer = NULL;
static size_t event_buffer_size = 0;
static size_t event_buffer_index = 0;

#ifdef _WIN32
static HANDLE* watch_handles = NULL;
static OVERLAPPED* overlapped_array = NULL;
static BYTE* buffer_array = NULL;
#else
static int inotify_fd = -1;
static int* watch_descriptors = NULL;
#endif

// Plugin implementation
static bool folder_watcher_init(void) {
#ifdef _WIN32
    // Initialize Windows file watching subsystem
    watching_active = false;
#else
    // Initialize Linux inotify
    inotify_fd = inotify_init();
    if (inotify_fd == -1) {
        return false;
    }
    watching_active = false;
#endif
    return true;
}

static void folder_watcher_cleanup(void) {
    folder_watcher_stop_watching();
    
    if (watch_configs) {
        free(watch_configs);
        watch_configs = NULL;
    }
    watch_count = 0;
    
    if (event_buffer) {
        free(event_buffer);
        event_buffer = NULL;
    }
    event_buffer_size = 0;
    event_buffer_index = 0;
    
#ifdef _WIN32
    if (watch_handles) {
        for (size_t i = 0; i < watch_count; i++) {
            if (watch_handles[i] != INVALID_HANDLE_VALUE) {
                CloseHandle(watch_handles[i]);
            }
        }
        free(watch_handles);
        watch_handles = NULL;
    }
    
    if (overlapped_array) {
        free(overlapped_array);
        overlapped_array = NULL;
    }
    
    if (buffer_array) {
        free(buffer_array);
        buffer_array = NULL;
    }
#else
    if (watch_descriptors) {
        for (size_t i = 0; i < watch_count; i++) {
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

static plugin_info_t* folder_watcher_get_plugin_info(void) {
    return &folder_watcher_plugin_info;
}

static bool folder_watcher_get_system_metrics(system_metrics_t* metrics) {
    // Not applicable for folder watcher plugin
    return false;
}

static bool folder_watcher_get_processes(process_info_t** processes, size_t* count) {
    // Not applicable for folder watcher plugin
    return false;
}

static bool folder_watcher_execute_command(const char* command, command_result_t* result) {
    // Not applicable for folder watcher plugin
    return false;
}

static bool folder_watcher_read_file(const char* path, file_content_t* content) {
    // Not applicable for folder watcher plugin
    return false;
}

static bool folder_watcher_get_system_info(system_info_t* info) {
    // Not applicable for folder watcher plugin
    return false;
}

/**
 * @brief Add folder to watch list
 */
static bool add_folder_watch(const char* path, bool recursive) {
    if (!path) return false;
    
    watch_config_t* new_configs = (watch_config_t*)realloc(watch_configs, 
                                                         (watch_count + 1) * sizeof(watch_config_t));
    if (!new_configs) {
        return false;
    }
    
    watch_configs = new_configs;
    watch_config_t* config = &watch_configs[watch_count];
    
    strncpy(config->path, path, sizeof(config->path) - 1);
    config->recursive = recursive;
    config->watch_subdirectories = recursive;
    config->buffer_size = 65536; // 64KB buffer
    
    // Enable all event types by default
    for (int i = 0; i < 6; i++) {
        config->notify_filters[i] = true;
    }
    
    watch_count++;
    return true;
}

/**
 * @brief Start watching all configured folders
 */
static bool folder_watcher_start_watching(void) {
    if (watching_active) {
        return true; // Already watching
    }
    
    if (watch_count == 0) {
        return false; // No folders to watch
    }
    
#ifdef _WIN32
    // Allocate resources for Windows watching
    watch_handles = (HANDLE*)malloc(watch_count * sizeof(HANDLE));
    overlapped_array = (OVERLAPPED*)malloc(watch_count * sizeof(OVERLAPPED));
    buffer_array = (BYTE*)malloc(watch_count * 65536);
    
    if (!watch_handles || !overlapped_array || !buffer_array) {
        return false;
    }
    
    // Initialize each watch
    for (size_t i = 0; i < watch_count; i++) {
        memset(&overlapped_array[i], 0, sizeof(OVERLAPPED));
        
        DWORD notifyFilter = FILE_NOTIFY_CHANGE_FILE_NAME |
                          FILE_NOTIFY_CHANGE_DIR_NAME |
                          FILE_NOTIFY_CHANGE_ATTRIBUTES |
                          FILE_NOTIFY_CHANGE_SIZE |
                          FILE_NOTIFY_CHANGE_LAST_WRITE |
                          FILE_NOTIFY_CHANGE_CREATION;
        
        watch_handles[i] = CreateFileA(
            watch_configs[i].path,
            FILE_LIST_DIRECTORY,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            NULL,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
            NULL
        );
        
        if (watch_handles[i] == INVALID_HANDLE_VALUE) {
            continue;
        }
        
        // Start async read
        ReadDirectoryChangesW(
            watch_handles[i],
            &buffer_array[i * 65536],
            65536,
            watch_configs[i].recursive,
            notifyFilter,
            NULL,
            &overlapped_array[i],
            NULL
        );
    }
#else
    // Allocate watch descriptors for Linux
    watch_descriptors = (int*)malloc(watch_count * sizeof(int));
    if (!watch_descriptors) {
        return false;
    }
    
    // Add watches for each folder
    for (size_t i = 0; i < watch_count; i++) {
        uint32_t mask = IN_CREATE | IN_MODIFY | IN_DELETE | IN_MOVED_FROM | IN_MOVED_TO;
        
        watch_descriptors[i] = inotify_add_watch(inotify_fd, watch_configs[i].path, mask);
        if (watch_descriptors[i] == -1) {
            // Failed to add watch, but continue with others
            watch_descriptors[i] = -1;
        }
    }
#endif
    
    watching_active = true;
    return true;
}

/**
 * @brief Stop watching all folders
 */
static bool folder_watcher_stop_watching(void) {
    if (!watching_active) {
        return true; // Not watching
    }
    
#ifdef _WIN32
    // Cancel all pending operations
    for (size_t i = 0; i < watch_count; i++) {
        if (watch_handles[i] != INVALID_HANDLE_VALUE) {
            CancelIoEx(watch_handles[i], &overlapped_array[i]);
            CloseHandle(watch_handles[i]);
            watch_handles[i] = INVALID_HANDLE_VALUE;
        }
    }
#else
    // Remove all inotify watches
    for (size_t i = 0; i < watch_count; i++) {
        if (watch_descriptors[i] != -1) {
            inotify_rm_watch(inotify_fd, watch_descriptors[i]);
            watch_descriptors[i] = -1;
        }
    }
#endif
    
    watching_active = false;
    return true;
}

/**
 * @brief Get file events from buffer
 */
static bool get_file_events(file_event_t** events, size_t* count, uint32_t max_events) {
    if (!events || !count) return false;
    
    *events = NULL;
    *count = 0;
    
    if (!watching_active || event_buffer_index == 0) {
        return true; // No events
    }
    
    // Return events from buffer
    size_t events_to_return = event_buffer_index;
    if (events_to_return > max_events) {
        events_to_return = max_events;
    }
    
    *events = (file_event_t*)malloc(events_to_return * sizeof(file_event_t));
    if (!*events) {
        return false;
    }
    
    memcpy(*events, event_buffer, events_to_return * sizeof(file_event_t));
    *count = events_to_return;
    
    // Clear returned events from buffer
    if (events_to_return < event_buffer_index) {
        memmove(event_buffer, 
               event_buffer + events_to_return, 
               (event_buffer_index - events_to_return) * sizeof(file_event_t));
    }
    event_buffer_index -= events_to_return;
    
    return true;
}

/**
 * @brief Process file system events (internal function)
 */
static void process_file_system_events(void) {
    if (!watching_active) return;
    
#ifdef _WIN32
    // Check for completed operations
    for (size_t i = 0; i < watch_count; i++) {
        if (watch_handles[i] == INVALID_HANDLE_VALUE) continue;
        
        DWORD bytesTransferred;
        if (GetOverlappedResult(watch_handles[i], &overlapped_array[i], &bytesTransferred, FALSE)) {
            // Process the notification
            BYTE* buffer = &buffer_array[i * 65536];
            FILE_NOTIFY_INFORMATION* notify = (FILE_NOTIFY_INFORMATION*)buffer;
            
            while (true) {
                // Add event to buffer
                if (event_buffer_index < event_buffer_size) {
                    file_event_t* event = &event_buffer[event_buffer_index];
                    memset(event, 0, sizeof(file_event_t));
                    
                    event->timestamp = time(NULL);
                    
                    switch (notify->Action) {
                        case FILE_ACTION_ADDED:
                            event->event_type = FILE_EVENT_CREATED;
                            break;
                        case FILE_ACTION_REMOVED:
                            event->event_type = FILE_EVENT_DELETED;
                            break;
                        case FILE_ACTION_MODIFIED:
                            event->event_type = FILE_EVENT_MODIFIED;
                            break;
                        case FILE_ACTION_RENAMED_OLD_NAME:
                            event->event_type = FILE_EVENT_RENAMED;
                            break;
                        case FILE_ACTION_RENAMED_NEW_NAME:
                            event->event_type = FILE_EVENT_RENAMED;
                            break;
                    }
                    
                    // Convert wide character path to multi-byte
                    WideCharToMultiByte(CP_UTF8, 0, notify->FileName, 
                                     notify->FileNameLength / sizeof(WCHAR),
                                     event->new_path, sizeof(event->new_path) - 1, 
                                     NULL, NULL);
                    
                    event_buffer_index++;
                    if (event_buffer_index >= event_buffer_size) {
                        event_buffer_index = 0; // Wrap around
                    }
                }
                
                if (notify->NextEntryOffset == 0) break;
                notify = (FILE_NOTIFY_INFORMATION*)((BYTE*)notify + notify->NextEntryOffset);
            }
            
            // Restart the read operation
            ReadDirectoryChangesW(
                watch_handles[i],
                &buffer_array[i * 65536],
                65536,
                watch_configs[i].recursive,
                FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_DIR_NAME |
                FILE_NOTIFY_CHANGE_ATTRIBUTES | FILE_NOTIFY_CHANGE_SIZE |
                FILE_NOTIFY_CHANGE_LAST_WRITE | FILE_NOTIFY_CHANGE_CREATION,
                NULL,
                &overlapped_array[i],
                NULL
            );
        }
    }
#else
    // Process inotify events
    char buffer[4096];
    ssize_t length = read(inotify_fd, buffer, sizeof(buffer));
    
    if (length > 0) {
        size_t i = 0;
        while (i < (size_t)length) {
            struct inotify_event* event = (struct inotify_event*)&buffer[i];
            
            if (event_buffer_index < event_buffer_size) {
                file_event_t* fileEvent = &event_buffer[event_buffer_index];
                memset(fileEvent, 0, sizeof(file_event_t));
                
                fileEvent->timestamp = time(NULL);
                fileEvent->is_directory = (event->mask & IN_ISDIR) != 0;
                
                if (event->mask & IN_CREATE) {
                    fileEvent->event_type = FILE_EVENT_CREATED;
                } else if (event->mask & IN_MODIFY) {
                    fileEvent->event_type = FILE_EVENT_MODIFIED;
                } else if (event->mask & IN_DELETE) {
                    fileEvent->event_type = FILE_EVENT_DELETED;
                } else if (event->mask & IN_MOVED_FROM) {
                    fileEvent->event_type = FILE_EVENT_RENAMED;
                } else if (event->mask & IN_MOVED_TO) {
                    fileEvent->event_type = FILE_EVENT_RENAMED;
                }
                
                // Find the watch config for this event
                for (size_t j = 0; j < watch_count; j++) {
                    if (watch_descriptors[j] == event->wd) {
                        snprintf(fileEvent->new_path, sizeof(fileEvent->new_path), 
                                "%s/%s", watch_configs[j].path, event->name);
                        break;
                    }
                }
                
                // Extract file extension
                const char* dot = strrchr(event->name, '.');
                if (dot) {
                    strncpy(fileEvent->file_extension, dot + 1, 
                           sizeof(fileEvent->file_extension) - 1);
                }
                
                event_buffer_index++;
                if (event_buffer_index >= event_buffer_size) {
                    event_buffer_index = 0; // Wrap around
                }
            }
            
            i += sizeof(struct inotify_event) + event->len;
        }
    }
#endif
}

static void folder_watcher_free_memory(void* ptr) {
    if (ptr) {
        free(ptr);
    }
}

// Plugin interface
static plugin_interface_t folder_watcher_interface = {
    .get_plugin_info = folder_watcher_get_plugin_info,
    .init = folder_watcher_init,
    .cleanup = folder_watcher_cleanup,
    .get_system_metrics = folder_watcher_get_system_metrics,
    .get_processes = folder_watcher_get_processes,
    .execute_command = folder_watcher_execute_command,
    .read_file = folder_watcher_read_file,
    .get_system_info = folder_watcher_get_system_info,
    .free_memory = folder_watcher_free_memory
};

// Plugin entry point
PLUGIN_EXPORT plugin_interface_t* PLUGIN_CALL get_plugin_interface(void) {
    return &folder_watcher_interface;
}
