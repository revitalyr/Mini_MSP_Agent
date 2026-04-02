#include "../../include/plugin_interface_common.h"
#include <dlfcn.h>
#include <unistd.h>
#include <sys/stat.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <stdbool.h>
#include <stdint.h>

// Platform-specific result codes
typedef enum {
    PLATFORM_RESULT_SUCCESS = 0,
    PLATFORM_RESULT_ERROR = 1,
    PLATFORM_RESULT_DLL_LOAD_FAILED = 2,
    PLATFORM_RESULT_PROC_NOT_FOUND = 3,
    PLATFORM_RESULT_ACCESS_DENIED = 4
} platform_result_t;

// Platform-specific plugin loader
typedef struct {
    void* m_handle;
    plugin_interface_t* m_interface;
    char m_so_path[512];
    plugin_status_t m_status;
    uint64_t m_load_time;
    uint32_t m_calls_made;
} linux_plugin_loader_t;

// Global state
static linux_plugin_loader_t* g_plugin_loaders = NULL;
static size_t g_plugin_count = 0;
static size_t g_plugin_capacity = 0;

// Platform-specific functions
platform_result_t load_linux_plugin(const char* so_path, linux_plugin_loader_t* loader) {
    if (!so_path || !loader) {
        return PLATFORM_RESULT_ERROR;
    }
    
    // Load the shared library
    loader->m_handle = dlopen(so_path, RTLD_LAZY);
    if (!loader->m_handle) {
        return PLATFORM_RESULT_DLL_LOAD_FAILED;
    }
    
    // Get the plugin interface function
    typedef plugin_interface_t* (*get_plugin_interface_t)(void);
    get_plugin_interface_t get_interface = (get_plugin_interface_t)(uintptr_t)dlsym(loader->m_handle, "get_plugin_interface");
    
    if (!get_interface) {
        dlclose(loader->m_handle);
        loader->m_handle = NULL;
        return PLATFORM_RESULT_PROC_NOT_FOUND;
    }
    
    // Get the plugin interface
    loader->m_interface = get_interface();
    if (!loader->m_interface) {
        dlclose(loader->m_handle);
        loader->m_handle = NULL;
        return PLATFORM_RESULT_ERROR;
    }
    
    // Initialize the plugin
    if (loader->m_interface && loader->m_interface->init) {
        if (loader->m_interface->init() != PLUGIN_RESULT_SUCCESS) {
            dlclose(loader->m_handle);
            loader->m_handle = NULL;
            loader->m_interface = NULL;
            return PLATFORM_RESULT_ERROR;
        }
    }
    
    // Store path and metadata
    strncpy(loader->m_so_path, so_path, sizeof(loader->m_so_path) - 1);
    loader->m_status = PLUGIN_STATUS_LOADED;
    loader->m_load_time = time(NULL) * 1000; // Convert to milliseconds
    loader->m_calls_made = 0;
    
    return PLATFORM_RESULT_SUCCESS;
}

platform_result_t unload_linux_plugin(linux_plugin_loader_t* loader) {
    if (!loader || !loader->m_handle) {
        return PLATFORM_RESULT_ERROR;
    }
    
    // Cleanup plugin
    if (loader->m_interface && loader->m_interface->cleanup) {
        loader->m_interface->cleanup();
    }
    
    // Close the shared library
    dlclose(loader->m_handle);
    loader->m_handle = NULL;
    loader->m_interface = NULL;
    loader->m_status = PLUGIN_STATUS_UNLOADED;
    
    return PLATFORM_RESULT_SUCCESS;
}

platform_result_t execute_linux_plugin_command(linux_plugin_loader_t* loader, const char* command, const char* params, command_result_t* result) {
    if (!loader || !loader->m_interface || !command || !result) {
        return PLATFORM_RESULT_ERROR;
    }
    
    if (!loader->m_interface->execute_command) {
        result->result = PLUGIN_RESULT_ERROR;
        strncpy(result->error, "Command execution not supported", sizeof(result->error) - 1);
        return PLATFORM_RESULT_ERROR;
    }
    
    loader->m_calls_made++;
    return loader->m_interface->execute_command(command, params, result);
}

// Platform-specific utility functions
uint64_t get_platform_timestamp(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000 + ts.tv_nsec / 1000000;
}

bool check_file_exists_linux(const char* path) {
    struct stat st;
    return (stat(path, &st) == 0) && S_ISREG(st.st_mode);
}

bool check_directory_exists_linux(const char* path) {
    struct stat st;
    return (stat(path, &st) == 0) && S_ISDIR(st.st_mode);
}
