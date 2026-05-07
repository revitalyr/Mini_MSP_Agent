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
#include <type_traits>
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
    uint64_t uptime;
    float cpu_usage;
    float ram_usage;
    float disk_usage;
    uint32_t _reserved; // Padding to 8-byte boundary
    char hostname[256];
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
    uint32_t pid;
    char name[256];
    float cpu_usage;
    uint64_t memory_usage;
    uint64_t start_time;
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
    uint32_t success; // Standardized for FFI stability
    char* content;
    size_t size;
    char error[256];
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
    uint32_t success; // Standardized for FFI stability
    int exit_code;
    char* stdout;
    char* stderr;
    char error[256];
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
    uint64_t uptime;
    uint64_t total_memory;
    uint64_t available_memory;
    char os_type[64];
    char os_version[128];
    char hostname[256];
    uint32_t cpu_cores;
    uint32_t _reserved; // Padding for 8-byte alignment
} system_info_t;

/**
 * @brief Forensic finding entry
 * 
 * Contains a single forensic artifact finding with category,
 * artifact type, path, value, and suspicious flag.
 */
typedef struct {
    char category[64];
    char artifact_type[64];
    char path[512];
    char value[512];
    uint32_t suspicious; // Using uint32_t for stable FFI size
    char details[1024];
} forensic_finding_t;

/**
 * @brief Forensic findings data
 * 
 * Contains all forensic findings collected from the system.
 * Includes registry autorun entries and other persistence mechanisms.
 */
typedef struct {
    forensic_finding_t* findings;
    size_t count;
    Timestamp collection_time;
} forensic_data_t;

// =============================================================================
// STATIC ALIGNMENT CHECKS (FFI SAFETY)
// =============================================================================
#ifdef __cplusplus
// Проверка размеров (Size checks)
static_assert(sizeof(system_metrics_t) == 280, "system_metrics_t size must be 280");
static_assert(sizeof(system_info_t) == 480, "system_info_t size must be 480");
static_assert(sizeof(forensic_finding_t) == 2180, "forensic_finding_t size must be 2180");
static_assert(sizeof(process_info_t) == 280, "process_info_t size must be 280");

// Проверка критических смещений (Offset checks)
static_assert(offsetof(system_metrics_t, uptime) == 0);
static_assert(offsetof(system_metrics_t, hostname) == 24);

static_assert(offsetof(system_info_t, total_memory) == 8);
static_assert(offsetof(system_info_t, hostname) == 216);

static_assert(offsetof(forensic_finding_t, suspicious) == 1152);
static_assert(offsetof(forensic_finding_t, details) == 1156);

static_assert(offsetof(process_info_t, cpu_usage) == 260);
static_assert(offsetof(process_info_t, memory_usage) == 264); // Учитывая 4-байтовый padding после cpu_usage
#endif

// Plugin function types
typedef const char* (PLUGIN_CALL *get_plugin_info_fn_t)(void);
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
typedef forensic_data_t* (PLUGIN_CALL *get_forensic_data_fn_t)(void);

/**
 * @brief Execute command with JSON request/response
 *
 * Allows direct JSON communication between server and plugin.
 * Plugin receives JSON request and returns JSON response as string.
 * Server will forward response directly to web without processing.
 *
 * @param json_request - JSON string with command and parameters
 * @return JSON response string (must be freed by caller via free_memory)
 *
 * Example request: {"cmd":"get_forensic","format":"json","params":{}}
 * Example response: {"status":"ok","data":{"findings":[]}}
 */
typedef char* (PLUGIN_CALL *execute_json_fn_t)(const char* json_request);

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
    get_forensic_data_fn_t get_forensic_data;
    free_memory_fn_t free_memory;
    execute_json_fn_t execute_json;  // Direct JSON exchange - server forwards response as-is
} plugin_interface_t;

// Plugin entry point
#ifdef __cplusplus
extern "C" {
#endif

PLUGIN_EXPORT plugin_interface_t* PLUGIN_CALL get_plugin_interface(void);

#ifdef __cplusplus
}
#endif
