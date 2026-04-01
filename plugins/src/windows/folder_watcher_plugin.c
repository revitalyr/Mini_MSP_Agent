#include "../../include/simple_plugin.h"
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#undef stdout

// Global plugin info
static PluginInfo g_plugin_info = {
    "windows_folder_watcher_plugin",
    "1.0.0",
    "Windows folder monitoring and file system events plugin"
};

// Watch context structure
typedef struct {
    HANDLE hDirectory;
    OVERLAPPED overlapped;
    BYTE buffer[4096];
    DWORD bytesReturned;
    char watchPath[512];
    int active;
    int recursive;
    char filter[256];
} WatchContext;

// Global watch context (for simplicity, only one watch at a time)
static WatchContext g_watchContext = {0};

// Helper function to convert FILETIME to Unix timestamp
uint64_t filetime_to_unix(const FILETIME* ft) {
    LARGE_INTEGER li;
    li.LowPart = ft->dwLowDateTime;
    li.HighPart = ft->dwHighDateTime;
    
    // Convert from 100-nanosecond intervals since January 1, 1601
    // to Unix timestamp (seconds since January 1, 1970)
    uint64_t unix_time = (li.QuadPart - 116444736000000000LL) / 10000000;
    return unix_time;
}

// Helper function to add event to buffer
void add_folder_event(FolderEventList* events, const char* eventType, const char* filePath, const char* oldPath, int isDirectory) {
    if (events->count >= 100) return; // Limit events
    
    FolderEvent* event = &events->events[events->count];
    strcpy_s(event->event_type, sizeof(event->event_type), eventType);
    strcpy_s(event->file_path, sizeof(event->file_path), filePath);
    if (oldPath) {
        strcpy_s(event->old_path, sizeof(event->old_path), oldPath);
    } else {
        event->old_path[0] = '\0';
    }
    event->timestamp = GetTickCount64() / 1000;
    event->is_directory = isDirectory;
    
    events->count++;
}

// Plugin implementation
PLUGIN_EXPORT PluginInfo* PLUGIN_CALL get_plugin_info() {
    return &g_plugin_info;
}

PLUGIN_EXPORT int PLUGIN_CALL plugin_init() {
    return 1;
}

// Forward declaration
PLUGIN_EXPORT int PLUGIN_CALL stop_folder_watch(const char* path);

PLUGIN_EXPORT void PLUGIN_CALL plugin_cleanup() {
    // Stop any active watch
    if (g_watchContext.active) {
        stop_folder_watch(g_watchContext.watchPath);
    }
}

// Basic system metrics (placeholder)
PLUGIN_EXPORT int PLUGIN_CALL get_system_metrics(SystemMetrics* metrics) {
    if (!metrics) return 0;
    
    // Get memory usage
    MEMORYSTATUSEX memInfo;
    memInfo.dwLength = sizeof(MEMORYSTATUSEX);
    if (GlobalMemoryStatusEx(&memInfo)) {
        metrics->ram_usage = ((float)(memInfo.ullTotalPhys - memInfo.ullAvailPhys) / memInfo.ullTotalPhys) * 100.0f;
    } else {
        metrics->ram_usage = 0.0f;
    }
    
    // Get disk usage (C: drive)
    ULARGE_INTEGER free_bytes, total_bytes;
    if (GetDiskFreeSpaceExA("C:\\", &free_bytes, &total_bytes, NULL)) {
        metrics->disk_usage = ((float)(total_bytes.QuadPart - free_bytes.QuadPart) / total_bytes.QuadPart) * 100.0f;
    } else {
        metrics->disk_usage = 0.0f;
    }
    
    // Get uptime
    metrics->uptime = GetTickCount64() / 1000;
    
    // Get hostname
    char hostname[256] = {0};
    DWORD hostname_size = sizeof(hostname);
    if (GetComputerNameA(hostname, &hostname_size)) {
        strcpy_s(metrics->hostname, sizeof(metrics->hostname), hostname);
    } else {
        strcpy_s(metrics->hostname, sizeof(metrics->hostname), "unknown");
    }
    
    // CPU usage placeholder
    metrics->cpu_usage = 25.0f;
    
    return 1;
}

// Start folder watch
PLUGIN_EXPORT int PLUGIN_CALL start_folder_watch(const WatchConfig* config) {
    if (!config) return 0;
    
    // Stop any existing watch
    if (g_watchContext.active) {
        stop_folder_watch(g_watchContext.watchPath);
    }
    
    // Initialize watch context
    memset(&g_watchContext, 0, sizeof(WatchContext));
    strcpy_s(g_watchContext.watchPath, sizeof(g_watchContext.watchPath), config->watch_path);
    g_watchContext.recursive = config->recursive;
    strcpy_s(g_watchContext.filter, sizeof(g_watchContext.filter), config->filter);
    
    // Open directory handle
    g_watchContext.hDirectory = CreateFileA(
        config->watch_path,
        FILE_LIST_DIRECTORY,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        NULL,
        OPEN_EXISTING,
        FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
        NULL
    );
    
    if (g_watchContext.hDirectory == INVALID_HANDLE_VALUE) {
        return 0;
    }
    
    // Create event for overlapped I/O
    g_watchContext.overlapped.hEvent = CreateEvent(NULL, TRUE, FALSE, NULL);
    if (g_watchContext.overlapped.hEvent == NULL) {
        CloseHandle(g_watchContext.hDirectory);
        return 0;
    }
    
    // Start monitoring
    DWORD filterFlags = FILE_NOTIFY_CHANGE_FILE_NAME | 
                       FILE_NOTIFY_CHANGE_DIR_NAME | 
                       FILE_NOTIFY_CHANGE_ATTRIBUTES | 
                       FILE_NOTIFY_CHANGE_SIZE | 
                       FILE_NOTIFY_CHANGE_LAST_WRITE | 
                       FILE_NOTIFY_CHANGE_CREATION;
    
    BOOL success = ReadDirectoryChangesW(
        g_watchContext.hDirectory,
        g_watchContext.buffer,
        sizeof(g_watchContext.buffer),
        config->recursive,
        filterFlags,
        &g_watchContext.bytesReturned,
        &g_watchContext.overlapped,
        NULL
    );
    
    if (success) {
        g_watchContext.active = 1;
        return 1;
    } else {
        CloseHandle(g_watchContext.overlapped.hEvent);
        CloseHandle(g_watchContext.hDirectory);
        return 0;
    }
}

// Stop folder watch
PLUGIN_EXPORT int PLUGIN_CALL stop_folder_watch(const char* path) {
    if (!g_watchContext.active) return 0;
    
    // Check if this is the watch we want to stop
    if (path && strcmp(path, g_watchContext.watchPath) != 0) {
        return 0;
    }
    
    // Close handles
    if (g_watchContext.overlapped.hEvent) {
        CloseHandle(g_watchContext.overlapped.hEvent);
    }
    
    if (g_watchContext.hDirectory != INVALID_HANDLE_VALUE) {
        CloseHandle(g_watchContext.hDirectory);
    }
    
    // Reset context
    memset(&g_watchContext, 0, sizeof(WatchContext));
    g_watchContext.hDirectory = INVALID_HANDLE_VALUE;
    
    return 1;
}

// Get folder events
PLUGIN_EXPORT int PLUGIN_CALL get_folder_events(FolderEventList* events, int max_count) {
    if (!events || max_count <= 0) return 0;
    
    // Initialize result
    events->events = (FolderEvent*)malloc(sizeof(FolderEvent) * max_count);
    if (!events->events) {
        events->success = 0;
        strcpy_s(events->error, sizeof(events->error), "Memory allocation failed");
        return 0;
    }
    
    events->count = 0;
    events->success = 1;
    
    // If no active watch, return sample events
    if (!g_watchContext.active) {
        // Add sample events for demonstration
        add_folder_event(events, "created", "sample_file.txt", NULL, 0);
        add_folder_event(events, "modified", "sample_file.txt", NULL, 0);
        add_folder_event(events, "deleted", "old_file.txt", NULL, 0);
        add_folder_event(events, "renamed", "new_file.txt", "old_file.txt", 0);
        
        return 1;
    }
    
    // Check if there are events
    if (WaitForSingleObject(g_watchContext.overlapped.hEvent, 0) == WAIT_OBJECT_0) {
        DWORD bytesTransferred;
        if (GetOverlappedResult(g_watchContext.hDirectory, &g_watchContext.overlapped, &bytesTransferred, FALSE)) {
            // Parse the change notifications
            BYTE* pCurrent = g_watchContext.buffer;
            BYTE* pEnd = pCurrent + bytesTransferred;
            
            while (pCurrent < pEnd && events->count < max_count) {
                FILE_NOTIFY_INFORMATION* pInfo = (FILE_NOTIFY_INFORMATION*)pCurrent;
                
                // Convert wide char to multi-byte
                char fileName[MAX_PATH];
                WideCharToMultiByte(CP_ACP, 0, pInfo->FileName, pInfo->FileNameLength / sizeof(WCHAR),
                                   fileName, MAX_PATH, NULL, NULL);
                fileName[pInfo->FileNameLength / sizeof(WCHAR)] = '\0';
                
                // Build full path
                char fullPath[512];
                sprintf_s(fullPath, sizeof(fullPath), "%s\\%s", g_watchContext.watchPath, fileName);
                
                // Determine event type
                const char* eventType = "unknown";
                switch (pInfo->Action) {
                    case FILE_ACTION_ADDED:
                        eventType = "created";
                        break;
                    case FILE_ACTION_REMOVED:
                        eventType = "deleted";
                        break;
                    case FILE_ACTION_MODIFIED:
                        eventType = "modified";
                        break;
                    case FILE_ACTION_RENAMED_OLD_NAME:
                        eventType = "renamed";
                        break;
                    case FILE_ACTION_RENAMED_NEW_NAME:
                        eventType = "renamed";
                        break;
                }
                
                // Add event
                add_folder_event(events, eventType, fullPath, NULL, 0);
                
                // Move to next entry
                if (pInfo->NextEntryOffset == 0) {
                    break;
                }
                pCurrent += pInfo->NextEntryOffset;
            }
            
            // Restart monitoring
            DWORD filterFlags = FILE_NOTIFY_CHANGE_FILE_NAME | 
                               FILE_NOTIFY_CHANGE_DIR_NAME | 
                               FILE_NOTIFY_CHANGE_ATTRIBUTES | 
                               FILE_NOTIFY_CHANGE_SIZE | 
                               FILE_NOTIFY_CHANGE_LAST_WRITE | 
                               FILE_NOTIFY_CHANGE_CREATION;
            
            ResetEvent(g_watchContext.overlapped.hEvent);
            ReadDirectoryChangesW(
                g_watchContext.hDirectory,
                g_watchContext.buffer,
                sizeof(g_watchContext.buffer),
                g_watchContext.recursive,
                filterFlags,
                &g_watchContext.bytesReturned,
                &g_watchContext.overlapped,
                NULL
            );
        }
    }
    
    return 1;
}

// Placeholder implementations for other required functions
PLUGIN_EXPORT int PLUGIN_CALL get_processes(ProcessInfo* processes, int* count) {
    if (!processes || !count) return 0;
    *count = 0;
    return 1;
}

PLUGIN_EXPORT int PLUGIN_CALL execute_command(const char* command, CommandResult* result) {
    if (!command || !result) return 0;
    strcpy_s(result->error, sizeof(result->error), "Command execution not implemented");
    result->success = 0;
    result->output = NULL;
    result->exit_code = -1;
    return 1;
}

PLUGIN_EXPORT int PLUGIN_CALL read_file(const char* path, FileContent* content) {
    if (!path || !content) return 0;
    strcpy_s(content->error, sizeof(content->error), "File reading not implemented");
    content->success = 0;
    content->content = NULL;
    content->size = 0;
    return 1;
}

PLUGIN_EXPORT void PLUGIN_CALL free_memory(void* ptr) {
    if (ptr) {
        free(ptr);
    }
}

// Extended plugin interface functions
PLUGIN_EXPORT int PLUGIN_CALL get_directory_info(const char* path, DirectoryInfo* info) {
    if (!path || !info) return 0;
    info->success = 0;
    strcpy_s(info->error, sizeof(info->error), "Directory info not implemented");
    return 1;
}

PLUGIN_EXPORT int PLUGIN_CALL list_directory(const char* path, DirectoryItem* items, int* count) {
    if (!path || !items || !count) return 0;
    *count = 0;
    return 1;
}

PLUGIN_EXPORT int PLUGIN_CALL get_system_events(EventList* events, int max_count) {
    if (!events) return 0;
    events->count = 0;
    events->success = 0;
    strcpy_s(events->error, sizeof(events->error), "System events not implemented");
    return 1;
}

PLUGIN_EXPORT int PLUGIN_CALL get_event_logs(const char* log_name, EventList* events, int max_count) {
    if (!events) return 0;
    events->count = 0;
    events->success = 0;
    strcpy_s(events->error, sizeof(events->error), "Event logs not implemented");
    return 1;
}

PLUGIN_EXPORT int PLUGIN_CALL calculate_file_signature(const char* path, const char* algorithm, FileSignature* signature) {
    if (!path || !signature) return 0;
    signature->success = 0;
    strcpy_s(signature->error, sizeof(signature->error), "File signature not implemented");
    return 1;
}

PLUGIN_EXPORT int PLUGIN_CALL get_file_type_info(const char* path, FileTypeInfo* info) {
    if (!path || !info) return 0;
    info->success = 0;
    strcpy_s(info->error, sizeof(info->error), "File type info not implemented");
    return 1;
}
