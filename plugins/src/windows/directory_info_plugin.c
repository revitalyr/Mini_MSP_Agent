#include "../../include/simple_plugin.h"
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Type aliases for better readability
typedef uint64_t UnixTimestamp;
typedef uint32_t FileCount;
typedef uint64_t FileSize;

// Global plugin info
static PluginInfo g_plugin_info = {
    "windows_directory_info_plugin",
    "1.0.0",
    "Windows directory information and listing plugin"
};

// Helper function to convert FILETIME to Unix timestamp
static UnixTimestamp filetime_to_unix(const FILETIME* ft) {
    LARGE_INTEGER li;
    li.LowPart = ft->dwLowDateTime;
    li.HighPart = ft->dwHighDateTime;
    
    // Convert from 100-nanosecond intervals since January 1, 1601
    // to Unix timestamp (seconds since January 1, 1970)
    UnixTimestamp unix_time = (li.QuadPart - 116444736000000000LL) / 10000000;
    return unix_time;
}

// RAII-style handle wrapper
typedef struct {
    HANDLE handle;
    int is_valid;
} SafeHandle;

static SafeHandle create_safe_handle(HANDLE h) {
    SafeHandle sh = {h, h != INVALID_HANDLE_VALUE};
    return sh;
}

static void close_safe_handle(SafeHandle* sh) {
    if (sh->is_valid && sh->handle != INVALID_HANDLE_VALUE) {
        CloseHandle(sh->handle);
        sh->is_valid = 0;
        sh->handle = INVALID_HANDLE_VALUE;
    }
}

// Error handling helper
static void set_error(char* buffer, size_t buffer_size, const char* message) {
    if (buffer && buffer_size > 0) {
        strncpy_s(buffer, buffer_size, message, _TRUNCATE);
        buffer[buffer_size - 1] = '\0';
    }
}

// Plugin implementation
PLUGIN_EXPORT PluginInfo* PLUGIN_CALL get_plugin_info() {
    return &g_plugin_info;
}

PLUGIN_EXPORT int PLUGIN_CALL plugin_init() {
    return 1;
}

PLUGIN_EXPORT void PLUGIN_CALL plugin_cleanup() {
    // Cleanup if needed
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

// Get directory information with improved error handling
PLUGIN_EXPORT int PLUGIN_CALL get_directory_info(const char* path, DirectoryInfo* info) {
    if (!path || !info) return 0;
    
    // Initialize result
    memset(info, 0, sizeof(DirectoryInfo));
    strncpy_s(info->path, sizeof(info->path), path, _TRUNCATE);
    info->success = 0;
    
    // Check if directory exists
    DWORD attrs = GetFileAttributesA(path);
    if (attrs == INVALID_FILE_ATTRIBUTES || !(attrs & FILE_ATTRIBUTE_DIRECTORY)) {
        set_error(info->error, sizeof(info->error), "Directory not found or not accessible");
        return 0;
    }
    
    // Get directory handle with RAII pattern
    char search_path[512];
    snprintf(search_path, sizeof(search_path), "%s\\*", path);
    
    WIN32_FIND_DATAA find_data;
    HANDLE hFind = FindFirstFileA(search_path, &find_data);
    SafeHandle safe_find = create_safe_handle(hFind);
    
    if (!safe_find.is_valid) {
        set_error(info->error, sizeof(info->error), "Failed to enumerate directory");
        return 0;
    }
    
    // Count files and directories with proper types
    FileSize total_size = 0;
    FileCount file_count = 0;
    FileCount dir_count = 0;
    
    do {
        if (strcmp(find_data.cFileName, ".") == 0 || strcmp(find_data.cFileName, "..") == 0) {
            continue;
        }
        
        if (find_data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) {
            dir_count++;
        } else {
            file_count++;
            LARGE_INTEGER size;
            size.LowPart = find_data.nFileSizeLow;
            size.HighPart = find_data.nFileSizeHigh;
            total_size += size.QuadPart;
        }
    } while (FindNextFileA(safe_find.handle, &find_data));
    
    close_safe_handle(&safe_find);
    
    // Get directory timestamps with RAII
    SafeHandle safe_dir = create_safe_handle(CreateFileA(
        path,
        GENERIC_READ,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        NULL,
        OPEN_EXISTING,
        FILE_FLAG_BACKUP_SEMANTICS,
        NULL
    ));
    
    if (safe_dir.is_valid) {
        FILETIME creation_time, access_time, write_time;
        if (GetFileTime(safe_dir.handle, &creation_time, &access_time, &write_time)) {
            info->created_time = filetime_to_unix(&creation_time);
            info->accessed_time = filetime_to_unix(&access_time);
            info->modified_time = filetime_to_unix(&write_time);
        }
        close_safe_handle(&safe_dir);
    }
    
    // Fill result with proper types
    info->size = total_size;
    info->file_count = file_count;
    info->dir_count = dir_count;
    info->success = 1;
    
    return 1;
}

// List directory contents with improved safety
PLUGIN_EXPORT int PLUGIN_CALL list_directory(const char* path, DirectoryItem* items, int* count) {
    if (!path || !items || !count) return 0;
    
    // Initialize result
    memset(items, 0, sizeof(DirectoryItem) * (*count));
    
    // Check if directory exists
    DWORD attrs = GetFileAttributesA(path);
    if (attrs == INVALID_FILE_ATTRIBUTES || !(attrs & FILE_ATTRIBUTE_DIRECTORY)) {
        return 0;
    }
    
    // Get directory handle with RAII
    char search_path[512];
    snprintf(search_path, sizeof(search_path), "%s\\*", path);
    
    WIN32_FIND_DATAA find_data;
    HANDLE hFind = FindFirstFileA(search_path, &find_data);
    SafeHandle safe_find = create_safe_handle(hFind);
    
    if (!safe_find.is_valid) {
        return 0;
    }
    
    const int max_count = *count;
    int actual_count = 0;
    
    do {
        if (actual_count >= max_count) break;
        
        if (strcmp(find_data.cFileName, ".") == 0 || strcmp(find_data.cFileName, "..") == 0) {
            continue;
        }
        
        DirectoryItem* item = &items[actual_count];
        
        // Fill basic info with safe string operations
        strncpy_s(item->path, sizeof(item->path), path, _TRUNCATE);
        strncpy_s(item->name, sizeof(item->name), find_data.cFileName, _TRUNCATE);
        
        item->is_directory = (find_data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) ? 1 : 0;
        item->is_hidden = (find_data.dwFileAttributes & FILE_ATTRIBUTE_HIDDEN) ? 1 : 0;
        
        // Get file size
        LARGE_INTEGER size;
        size.LowPart = find_data.nFileSizeLow;
        size.HighPart = find_data.nFileSizeHigh;
        item->size = size.QuadPart;
        
        // Get timestamps
        item->created_time = filetime_to_unix(&find_data.ftCreationTime);
        item->accessed_time = filetime_to_unix(&find_data.ftLastAccessTime);
        item->modified_time = filetime_to_unix(&find_data.ftLastWriteTime);
        
        // Get file permissions (simplified)
        if (find_data.dwFileAttributes & FILE_ATTRIBUTE_READONLY) {
            strncpy_s(item->permissions, sizeof(item->permissions), "read-only", _TRUNCATE);
        } else {
            strncpy_s(item->permissions, sizeof(item->permissions), "read-write", _TRUNCATE);
        }
        
        actual_count++;
    } while (FindNextFileA(safe_find.handle, &find_data));
    
    close_safe_handle(&safe_find);
    
    *count = actual_count;
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

PLUGIN_EXPORT int PLUGIN_CALL start_folder_watch(const WatchConfig* config) {
    if (!config) return 0;
    return 0; // Not implemented
}

PLUGIN_EXPORT int PLUGIN_CALL stop_folder_watch(const char* path) {
    if (!path) return 0;
    return 0; // Not implemented
}

PLUGIN_EXPORT int PLUGIN_CALL get_folder_events(FolderEventList* events, int max_count) {
    if (!events) return 0;
    events->count = 0;
    events->success = 0;
    strcpy_s(events->error, sizeof(events->error), "Folder events not implemented");
    return 1;
}
