#ifndef PLUGIN_INTERFACE_COMMON_H
#define PLUGIN_INTERFACE_COMMON_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// Common type definitions
typedef enum {
    PLUGIN_RESULT_SUCCESS = 0,
    PLUGIN_RESULT_ERROR = 1,
    PLUGIN_RESULT_INVALID_PARAM = 2,
    PLUGIN_RESULT_NOT_FOUND = 3,
    PLUGIN_RESULT_PERMISSION_DENIED = 4
} plugin_result_t;

typedef enum {
    PLUGIN_STATUS_UNLOADED = 0,
    PLUGIN_STATUS_LOADING = 1,
    PLUGIN_STATUS_LOADED = 2,
    PLUGIN_STATUS_ACTIVE = 3,
    PLUGIN_STATUS_ERROR = 4,
    PLUGIN_STATUS_UNLOADING = 5
} plugin_status_t;

typedef enum {
    PLUGIN_EVENT_TYPE_LOADED = 0,
    PLUGIN_EVENT_TYPE_UNLOADED = 1,
    PLUGIN_EVENT_TYPE_ERROR = 2,
    PLUGIN_EVENT_TYPE_STATUS_CHANGED = 3
} plugin_event_type_t;

// Plugin information structure (internal management)
typedef struct {
    char name[64];
    char version[16];
    char description[256];
    char author[64];
    plugin_status_t status;
    uint64_t load_time;
    uint32_t calls_made;
} plugin_info_internal_t;

// Command result structure (internal management)
typedef struct {
    plugin_result_t result;
    char* data;
    size_t data_size;
    char error[256];
} command_result_internal_t;

// Event callback function type
typedef void (*plugin_event_callback_t)(plugin_event_type_t event_type, const char* plugin_name, const void* event_data);

// Common utility functions (internal management)
typedef struct {
    plugin_result_t (*init)(void);
    plugin_result_t (*cleanup)(void);
    plugin_result_t (*get_info)(plugin_info_internal_t* info);
    plugin_result_t (*execute_command)(const char* command, const char* params, command_result_internal_t* result);
    plugin_result_t (*get_metrics)(char* metrics, size_t buffer_size);
    plugin_result_t (*set_event_callback)(plugin_event_callback_t callback);
} plugin_interface_internal_t;

// Plugin registry entry
typedef struct {
    char name[64];
    plugin_interface_internal_t* interface;
    char library_path[512];
    plugin_status_t status;
    uint64_t load_time;
    uint32_t calls_made;
} plugin_registry_entry_t;

// Global plugin manager state
typedef struct {
    plugin_registry_entry_t* entries;
    size_t count;
    size_t capacity;
    plugin_event_callback_t event_callback;
} plugin_manager_t;

#ifdef __cplusplus
}
#endif

#endif // PLUGIN_INTERFACE_COMMON_H
