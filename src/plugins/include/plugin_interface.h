/**
 * @file plugin_interface.h
 * @brief Plugin Interface Definition for Mini MSP Agent
 * 
 * This header defines the standardized interface that all C plugins must implement
 * to work with the Mini MSP Agent system. It provides data structures and
 * function pointers for plugin communication.
 * 
 * @author Mini MSP Agent Team
 * @version 1.0.0
 * @date 2026
 * 
 * ## Plugin Development Guide
 * 
 * 1. Include this header in your plugin source
 * 2. Implement all required functions from the interface
 * 3. Export the get_plugin_interface() function
 * 4. Compile as a shared library (.so on Linux, .dll on Windows)
 * 5. Place in the plugins directory
 * 
 * ## Thread Safety
 * 
 * All plugin functions must be thread-safe as they may be called
 * from multiple threads simultaneously.
 */

#pragma once

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "semantic_types.h"

#ifdef _WIN32
    #define PLUGIN_EXPORT __declspec(dllexport)
    #define PLUGIN_CALL __cdecl
#else
    #define PLUGIN_EXPORT __attribute__((visibility("default")))
    #define PLUGIN_CALL
#endif

/**
 * @brief Plugin information structure
 * 
 * Contains metadata about the plugin including name, version,
 * and description. This is returned by the get_plugin_info() function.
 * 
 * @var name - Human-readable plugin name
 * @var version - Plugin version string (semantic versioning)
 * @var description - Brief description of plugin functionality
 */
typedef struct {
    const char* name;
    const char* version;
    const char* description;
} plugin_info_t;

/**
 * @brief System metrics structure
 * 
 * Contains current system performance metrics collected by the plugin.
 * All values should be as accurate as possible.
 * 
 * @var cpu_usage - CPU usage percentage (0.0 - 100.0)
 * @var ram_usage - RAM usage percentage (0.0 - 100.0)
 * @var disk_usage - Disk usage percentage (0.0 - 100.0)
 * @var uptime - System uptime in seconds
 * @var hostname - System hostname (null-terminated, max 255 chars)
 */
typedef struct {
    Percentage cpu_usage;
    Percentage ram_usage;
    Percentage disk_usage;
    Uptime uptime;
    char m_hostname[kMaxHostnameLen];
} system_metrics_t;

/**
 * @brief Process information structure
 * 
 * Contains information about a running process including
 * resource usage and identification details.
 * 
 * @var pid - Process ID
 * @var name - Process name (null-terminated, max 255 chars)
 * @var cpu_usage - CPU usage percentage (0.0 - 100.0)
 * @var memory_usage - Memory usage in bytes
 * @var start_time - Process start time (Unix timestamp)
 */
typedef struct {
    ProcessId pid;
    char m_name[kMaxHostnameLen];
    Percentage cpu_usage;
    MemorySize memory_usage;
    Timestamp start_time;
} process_info_t;

/**
 * @brief File content structure
 * 
 * Contains the result of a file read operation including
 * the content, size, and any error information.
 * 
 * @var content - Pointer to file content (must be freed by caller)
 * @var size - Size of content in bytes
 * @var success - true if file was read successfully
 * @var error - Error message if success is false (null-terminated)
 */
typedef struct {
    char* content;
    BufferSize size;
    bool success;
    char m_error[kMaxErrorMsgLen];
} file_content_t;

/**
 * @brief Command execution result structure
 * 
 * Contains the result of a command execution including
 * stdout, stderr, exit code, and error information.
 * 
 * @var stdout - Pointer to stdout output (must be freed by caller)
 * @var stderr - Pointer to stderr output (must be freed by caller)
 * @var exit_code - Process exit code
 * @var success - true if command executed successfully
 * @var error - Error message if success is false (null-terminated)
 */

// Save and disable stdout/stderr macros to avoid conflicts with Windows CRT
#ifdef _WIN32
    #pragma push_macro("stdout")
    #pragma push_macro("stderr")
    #undef stdout
    #undef stderr
#endif

typedef struct {
    char* stdout;
    char* stderr;
    ExitCode exit_code;
    bool success;
    char m_error[kMaxErrorMsgLen];
} command_result_t;

// Restore stdout/stderr macros
#ifdef _WIN32
    #pragma pop_macro("stderr")
    #pragma pop_macro("stdout")
#endif

/**
 * @brief System information structure
 * 
 * Contains comprehensive system information including OS details,
 * hardware specifications, and resource information.
 * 
 * @var os_type - Operating system type (null-terminated, max 63 chars)
 * @var os_version - OS version (null-terminated, max 127 chars)
 * @var hostname - System hostname (null-terminated, max 255 chars)
 * @var uptime - System uptime in seconds
 * @var cpu_cores - Number of CPU cores
 * @var total_memory - Total system memory in bytes
 * @var available_memory - Available memory in bytes
 */
typedef struct {
    char m_os_type[kMaxOsTypeLen];
    char m_os_version[kMaxOsVersionLen];
    char m_hostname[kMaxHostnameLen];
    Uptime uptime;
    uint32_t cpu_cores;
    MemorySize total_memory;
    MemorySize available_memory;
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
typedef directory_info_data_t* (PLUGIN_CALL *get_directory_info_data_fn_t)(const char* path, bool recursive, bool show_hidden, DepthLevel max_depth);
typedef event_data_t* (PLUGIN_CALL *get_event_data_fn_t)(const char* path);
typedef watchers_data_t* (PLUGIN_CALL *get_watchers_data_fn_t)(void);
typedef file_reader_data_t* (PLUGIN_CALL *get_file_reader_data_fn_t)(const char* path);
typedef sensor_data_t* (PLUGIN_CALL *get_sensor_data_fn_t)(void);
typedef camera_data_t* (PLUGIN_CALL *get_camera_data_fn_t)(void);
typedef processing_results_t* (PLUGIN_CALL *get_processing_results_fn_t)(void);
typedef video_frame_t* (PLUGIN_CALL *get_video_frame_fn_t)(void);
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
    get_directory_info_data_fn_t get_directory_info_data;
    get_event_data_fn_t get_event_data;
    get_watchers_data_fn_t get_watchers_data;
    get_file_reader_data_fn_t get_file_reader_data;
    get_sensor_data_fn_t get_sensor_data;
    get_camera_data_fn_t get_camera_data;
    get_processing_results_fn_t get_processing_results;
    get_video_frame_fn_t get_video_frame;
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
