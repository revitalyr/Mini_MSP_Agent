#pragma once

#include <stdint.h>
#include <stdbool.h>

#ifdef _WIN32
    #define PLUGIN_EXPORT __declspec(dllexport)
    #define PLUGIN_CALL __cdecl
#else
    #define PLUGIN_EXPORT __attribute__((visibility("default")))
    #define PLUGIN_CALL
#endif

// Plugin information structure
typedef struct {
    const char* name;
    const char* version;
    const char* description;
} plugin_info_t;

// System metrics structure
typedef struct {
    float cpu_usage;
    float ram_usage;
    float disk_usage;
    uint64_t uptime;
    char hostname[256];
} system_metrics_t;

// Process information structure
typedef struct {
    uint32_t pid;
    char name[256];
    float cpu_usage;
    uint64_t memory_usage;
    uint64_t start_time;
} process_info_t;

// File content structure
typedef struct {
    char* content;
    size_t size;
    bool success;
    char error[512];
} file_content_t;

// Command result structure
typedef struct {
    char* stdout;
    char* stderr;
    int exit_code;
    bool success;
    char error[512];
} command_result_t;

// System info structure
typedef struct {
    char os_type[64];
    char os_version[128];
    char hostname[256];
    uint64_t uptime;
    uint32_t cpu_cores;
    uint64_t total_memory;
    uint64_t available_memory;
} system_info_t;

// Plugin function types
typedef plugin_info_t* (PLUGIN_CALL *get_plugin_info_fn_t)(void);
typedef bool (PLUGIN_CALL *plugin_init_fn_t)(void);
typedef void (PLUGIN_CALL *plugin_cleanup_fn_t)(void);
typedef bool (PLUGIN_CALL *get_system_metrics_fn_t)(system_metrics_t* metrics);
typedef bool (PLUGIN_CALL *get_processes_fn_t)(process_info_t** processes, size_t* count);
typedef bool (PLUGIN_CALL *execute_command_fn_t)(const char* command, command_result_t* result);
typedef bool (PLUGIN_CALL *read_file_fn_t)(const char* path, file_content_t* content);
typedef bool (PLUGIN_CALL *get_system_info_fn_t)(system_info_t* info);
typedef void (PLUGIN_CALL *free_memory_fn_t)(void* ptr);

// Plugin interface structure
typedef struct {
    get_plugin_info_fn_t get_plugin_info;
    plugin_init_fn_t init;
    plugin_cleanup_fn_t cleanup;
    get_system_metrics_fn_t get_system_metrics;
    get_processes_fn_t get_processes;
    execute_command_fn_t execute_command;
    read_file_fn_t read_file;
    get_system_info_fn_t get_system_info;
    free_memory_fn_t free_memory;
} plugin_interface_t;

// Plugin entry point
#ifdef __cplusplus
extern "C" {
#endif

PLUGIN_EXPORT plugin_interface_t* PLUGIN_CALL get_plugin_interface(void);

#ifdef __cplusplus
}
#endif
