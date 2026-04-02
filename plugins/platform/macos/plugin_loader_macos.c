/**
 * @file plugin_loader_macos.c
 * @brief macOS-specific implementation for Plugin Loader
 */

#include "../../include/plugin_interface_common.h"
#include "../../include/semantic_types.h"
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

plugin_result_t macos_load_plugin(const char* library_path, plugin_handle_t* handle) {
    if (!library_path || !handle) return PLUGIN_RESULT_INVALID_PARAM;
    
    *handle = dlopen(library_path, RTLD_LAZY);
    if (!*handle) {
        printf("macOS Error loading plugin %s: %s\n", library_path, dlerror());
        return PLUGIN_RESULT_ERROR;
    }
    
    return PLUGIN_RESULT_SUCCESS;
}

plugin_result_t macos_unload_plugin(plugin_handle_t handle) {
    if (!handle) return PLUGIN_RESULT_INVALID_PARAM;
    
    if (dlclose(handle) != 0) {
        printf("macOS Error unloading plugin: %s\n", dlerror());
        return PLUGIN_RESULT_ERROR;
    }
    
    return PLUGIN_RESULT_SUCCESS;
}

plugin_result_t macos_get_plugin_function(plugin_handle_t handle, const char* function_name, void** function_ptr) {
    if (!handle || !function_name || !function_ptr) return PLUGIN_RESULT_INVALID_PARAM;
    
    *function_ptr = dlsym(handle, function_name);
    if (!*function_ptr) {
        printf("macOS Error finding function %s: %s\n", function_name, dlerror());
        return PLUGIN_RESULT_ERROR;
    }
    
    return PLUGIN_RESULT_SUCCESS;
}

plugin_result_t macos_get_plugin_info(plugin_handle_t handle, plugin_info_t* info) {
    if (!handle || !info) return PLUGIN_RESULT_INVALID_PARAM;
    
    // Try to get plugin info function
    plugin_result_t (*get_info_func)(plugin_info_t*) = NULL;
    
    if (macos_get_plugin_function(handle, "get_plugin_info", (void**)&get_info_func) == PLUGIN_RESULT_SUCCESS) {
        return get_info_func(info);
    }
    
    // Default info if function not available
    strncpy(info->name, "Unknown macOS Plugin", sizeof(info->name) - 1);
    strncpy(info->version, "1.0.0", sizeof(info->version) - 1);
    strncpy(info->description, "macOS Plugin", sizeof(info->description) - 1);
    info->type = PLUGIN_TYPE_UTILITY;
    
    return PLUGIN_RESULT_SUCCESS;
}
