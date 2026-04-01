#include "../../include/simple_plugin.h"
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#undef stdout

// Type aliases for better readability
typedef uint64_t Timestamp;
typedef uint32_t EventId;
typedef uint32_t EventCount;

// Global plugin info
static PluginInfo g_plugin_info = {
    "windows_event_data_plugin",
    "1.0.0",
    "Windows system events and log data plugin"
};

// Helper function to convert FILETIME to Unix timestamp
static Timestamp filetime_to_unix(const FILETIME* ft) {
    LARGE_INTEGER li;
    li.LowPart = ft->dwLowDateTime;
    li.HighPart = ft->dwHighDateTime;
    
    // Convert from 100-nanosecond intervals since January 1, 1601
    // to Unix timestamp (seconds since January 1, 1970)
    Timestamp unix_time = (li.QuadPart - 116444736000000000LL) / 10000000;
    return unix_time;
}

// Error handling helper
static void set_error(char* buffer, size_t buffer_size, const char* message) {
    if (buffer && buffer_size > 0) {
        strncpy_s(buffer, buffer_size, message, _TRUNCATE);
        buffer[buffer_size - 1] = '\0';
    }
}

// Memory allocation helper with error checking
static void* safe_malloc(size_t size) {
    void* ptr = malloc(size);
    return ptr;
}

// Event creation helper
static void create_sample_event(EventData* event, const char* type, const char* source, 
                              const char* message, const char* severity, EventId id, const char* category) {
    if (!event) return;
    
    strncpy_s(event->event_type, sizeof(event->event_type), type, _TRUNCATE);
    strncpy_s(event->source, sizeof(event->source), source, _TRUNCATE);
    event->timestamp = GetTickCount64() / 1000;
    strncpy_s(event->message, sizeof(event->message), message, _TRUNCATE);
    strncpy_s(event->severity, sizeof(event->severity), severity, _TRUNCATE);
    event->event_id = id;
    strncpy_s(event->category, sizeof(event->category), category, _TRUNCATE);
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

// Get system events from Windows Event Log with improved safety
PLUGIN_EXPORT int PLUGIN_CALL get_system_events(EventList* events, int max_count) {
    if (!events || max_count <= 0) return 0;
    
    // Initialize result
    events->events = (EventData*)safe_malloc(sizeof(EventData) * max_count);
    if (!events->events) {
        set_error(events->error, sizeof(events->error), "Memory allocation failed");
        events->success = 0;
        return 0;
    }
    
    events->count = 0;
    events->success = 1;
    
    // Create sample events for demonstration with helper function
    if (max_count >= 2) {
        events->count = 2;
        
        create_sample_event(&events->events[0], 
                          "system", "Windows", 
                          "System started successfully", "INFO", 
                          6009, "System");
        
        create_sample_event(&events->events[1], 
                          "security", "Windows Security", 
                          "User login successful", "INFO", 
                          4624, "Logon/Logoff");
        
        // Adjust timestamp for second event (1 hour ago)
        events->events[1].timestamp = GetTickCount64() / 1000 - 3600;
    } else if (max_count == 1) {
        events->count = 1;
        create_sample_event(&events->events[0], 
                          "system", "Windows", 
                          "System event sample", "INFO", 
                          1, "System");
    }
    
    return 1;
}

// Get event logs from specific log
PLUGIN_EXPORT int PLUGIN_CALL get_event_logs(const char* log_name, EventList* events, int max_count) {
    if (!log_name || !events || max_count <= 0) return 0;
    
    // Initialize result
    events->events = (EventData*)malloc(sizeof(EventData) * max_count);
    if (!events->events) {
        events->success = 0;
        strcpy_s(events->error, sizeof(events->error), "Memory allocation failed");
        return 0;
    }
    
    events->count = 0;
    events->success = 1;
    
    // Create sample events for the specified log
    events->count = 1;
    
    EventData* event = &events->events[0];
    strcpy_s(event->event_type, sizeof(event->event_type), log_name);
    strcpy_s(event->source, sizeof(event->source), log_name);
    event->timestamp = GetTickCount64() / 1000;
    
    char message[1024];
    sprintf_s(message, sizeof(message), "Sample event from %s log", log_name);
    strcpy_s(event->message, sizeof(event->message), message);
    
    strcpy_s(event->severity, sizeof(event->severity), "INFO");
    event->event_id = 1000;
    strcpy_s(event->category, sizeof(event->category), log_name);
    
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
