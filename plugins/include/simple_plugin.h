#pragma once

#include <stdint.h>

#ifdef _WIN32
    #define PLUGIN_EXPORT __declspec(dllexport)
    #define PLUGIN_CALL __cdecl
#else
    #define PLUGIN_EXPORT __attribute__((visibility("default")))
    #define PLUGIN_CALL
#endif

#ifdef __cplusplus
extern "C" {
#endif

// Simple plugin interface
typedef struct {
    char name[256];
    char version[64];
    char description[512];
} PluginInfo;

typedef struct {
    float cpu_usage;
    float ram_usage;
    float disk_usage;
    uint64_t uptime;
    char hostname[256];
} SystemMetrics;

typedef struct {
    uint32_t pid;
    char name[256];
    uint64_t memory_usage;
} ProcessInfo;

typedef struct {
    char* output;
    int exit_code;
    int success;
    char error[256];
} CommandResult;

typedef struct {
    char* content;
    size_t size;
    int success;
    char error[256];
} FileContent;

// Directory Information Plugin
typedef struct {
    char path[512];
    uint64_t size;
    uint32_t file_count;
    uint32_t dir_count;
    uint64_t created_time;
    uint64_t modified_time;
    uint64_t accessed_time;
    int success;
    char error[256];
} DirectoryInfo;

typedef struct {
    char path[512];
    char name[256];
    uint64_t size;
    int is_directory;
    int is_hidden;
    uint64_t created_time;
    uint64_t modified_time;
    uint64_t accessed_time;
    char permissions[64];
} DirectoryItem;

// Event Data Plugin
typedef struct {
    char event_type[64];
    char source[256];
    uint64_t timestamp;
    char message[1024];
    char severity[32];
    int event_id;
    char category[128];
} EventData;

typedef struct {
    EventData* events;
    int count;
    int success;
    char error[256];
} EventList;

// File Signature Plugin
typedef struct {
    char file_path[512];
    char algorithm[64];
    char signature[512];
    uint64_t file_size;
    uint64_t computed_time;
    int success;
    char error[256];
} FileSignature;

typedef struct {
    char file_path[512];
    char mime_type[128];
    char file_type[128];
    char encoding[64];
    int is_executable;
    int is_archive;
    int is_text;
    int success;
    char error[256];
} FileTypeInfo;

// Folder Watcher Plugin
typedef struct {
    char watch_path[512];
    int recursive;
    int active;
    char filter[256];
} WatchConfig;

typedef struct {
    char event_type[64];  // "created", "modified", "deleted", "renamed"
    char file_path[512];
    char old_path[512];   // for rename events
    uint64_t timestamp;
    int is_directory;
} FolderEvent;

typedef struct {
    FolderEvent* events;
    int count;
    int success;
    char error[256];
} FolderEventList;

// Plugin interface functions
typedef PluginInfo* (PLUGIN_CALL *get_plugin_info_fn_t)();
typedef int (PLUGIN_CALL *plugin_init_fn_t)();
typedef void (PLUGIN_CALL *plugin_cleanup_fn_t)();
typedef int (PLUGIN_CALL *get_system_metrics_fn_t)(SystemMetrics* metrics);
typedef int (PLUGIN_CALL *get_processes_fn_t)(ProcessInfo* processes, int* count);
typedef int (PLUGIN_CALL *execute_command_fn_t)(const char* command, CommandResult* result);
typedef int (PLUGIN_CALL *read_file_fn_t)(const char* path, FileContent* content);
typedef void (PLUGIN_CALL *free_memory_fn_t)(void* ptr);

// Directory Information functions
typedef int (PLUGIN_CALL *get_directory_info_fn_t)(const char* path, DirectoryInfo* info);
typedef int (PLUGIN_CALL *list_directory_fn_t)(const char* path, DirectoryItem* items, int* count);

// Event Data functions
typedef int (PLUGIN_CALL *get_system_events_fn_t)(EventList* events, int max_count);
typedef int (PLUGIN_CALL *get_event_logs_fn_t)(const char* log_name, EventList* events, int max_count);

// File Signature functions
typedef int (PLUGIN_CALL *calculate_file_signature_fn_t)(const char* path, const char* algorithm, FileSignature* signature);
typedef int (PLUGIN_CALL *get_file_type_info_fn_t)(const char* path, FileTypeInfo* info);

// Folder Watcher functions
typedef int (PLUGIN_CALL *start_folder_watch_fn_t)(const WatchConfig* config);
typedef int (PLUGIN_CALL *stop_folder_watch_fn_t)(const char* path);
typedef int (PLUGIN_CALL *get_folder_events_fn_t)(FolderEventList* events, int max_count);

typedef struct {
    get_plugin_info_fn_t get_plugin_info;
    plugin_init_fn_t init;
    plugin_cleanup_fn_t cleanup;
    get_system_metrics_fn_t get_system_metrics;
    get_processes_fn_t get_processes;
    execute_command_fn_t execute_command;
    read_file_fn_t read_file;
    free_memory_fn_t free_memory;
    
    // Directory Information
    get_directory_info_fn_t get_directory_info;
    list_directory_fn_t list_directory;
    
    // Event Data
    get_system_events_fn_t get_system_events;
    get_event_logs_fn_t get_event_logs;
    
    // File Signature
    calculate_file_signature_fn_t calculate_file_signature;
    get_file_type_info_fn_t get_file_type_info;
    
    // Folder Watcher
    start_folder_watch_fn_t start_folder_watch;
    stop_folder_watch_fn_t stop_folder_watch;
    get_folder_events_fn_t get_folder_events;
} plugin_interface_t;

#ifdef __cplusplus
}
#endif
