/**
 * @file plugin_registry.c
 * @brief Plugin Registry for Mini MSP Agent
 * 
 * Provides centralized plugin management, registration, and discovery
 * for all Mini MSP Agent plugins.
 * 
 * @author Mini MSP Agent Team
 * @version 1.0.0
 * @date 2026
 */

#include "plugin_interface.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <dlfcn.h> // For dynamic library loading

#ifdef _WIN32
#include <windows.h>
#define PLUGIN_EXTENSION ".dll"
#define PATH_SEPARATOR "\\"
#else
#include <dirent.h>
#include <sys/stat.h>
#define PLUGIN_EXTENSION ".so"
#define PATH_SEPARATOR "/"
#endif

/**
 * @brief Registered plugin structure
 */
typedef struct {
    char name[256];
    char version[64];
    char description[512];
    char file_path[512];
    plugin_interface_t* interface;
    void* library_handle;
    bool is_loaded;
    bool is_enabled;
    time_t load_time;
    uint64_t calls_made;
    uint64_t errors;
} registered_plugin_t;

/**
 * @brief Plugin registry state
 */
typedef struct {
    registered_plugin_t* plugins;
    size_t plugin_count;
    size_t plugin_capacity;
    char plugin_directory[512];
    bool auto_discover;
    time_t last_scan_time;
} plugin_registry_t;

// Global registry instance
static plugin_registry_t g_registry = {0};

// Registry functions
bool plugin_registry_init(const char* plugin_dir);
void plugin_registry_cleanup(void);
bool plugin_registry_scan_directory(void);
bool plugin_registry_load_plugin(const char* plugin_path);
bool plugin_registry_unload_plugin(const char* plugin_name);
bool plugin_registry_enable_plugin(const char* plugin_name, bool enable);
registered_plugin_t* plugin_registry_find_plugin(const char* plugin_name);
bool plugin_registry_get_all_plugins(registered_plugin_t** plugins, size_t* count);
bool plugin_registry_get_enabled_plugins(registered_plugin_t** plugins, size_t* count);
size_t plugin_registry_get_plugin_count(void);
size_t plugin_registry_get_enabled_count(void);

/**
 * @brief Initialize plugin registry
 */
bool plugin_registry_init(const char* plugin_dir) {
    if (!plugin_dir) {
        return false;
    }
    
    memset(&g_registry, 0, sizeof(plugin_registry_t));
    strncpy(g_registry.plugin_directory, plugin_dir, sizeof(g_registry.plugin_directory) - 1);
    
    // Initial capacity for 16 plugins
    g_registry.plugin_capacity = 16;
    g_registry.plugins = (registered_plugin_t*)malloc(
        g_registry.plugin_capacity * sizeof(registered_plugin_t));
    
    if (!g_registry.plugins) {
        return false;
    }
    
    g_registry.auto_discover = true;
    g_registry.last_scan_time = 0;
    
    // Scan for plugins
    return plugin_registry_scan_directory();
}

/**
 * @brief Cleanup plugin registry
 */
void plugin_registry_cleanup(void) {
    if (!g_registry.plugins) {
        return;
    }
    
    // Unload all plugins
    for (size_t i = 0; i < g_registry.plugin_count; i++) {
        if (g_registry.plugins[i].is_loaded) {
            plugin_registry_unload_plugin(g_registry.plugins[i].name);
        }
    }
    
    free(g_registry.plugins);
    g_registry.plugins = NULL;
    g_registry.plugin_count = 0;
    g_registry.plugin_capacity = 0;
}

/**
 * @brief Scan plugin directory for plugins
 */
bool plugin_registry_scan_directory(void) {
    if (!g_registry.auto_discover) {
        return true;
    }
    
#ifdef _WIN32
    WIN32_FIND_DATAA findFileData;
    HANDLE hFind = INVALID_HANDLE_VALUE;
    char searchPath[512];
    
    snprintf(searchPath, sizeof(searchPath), "%s\\*%s", 
             g_registry.plugin_directory, PLUGIN_EXTENSION);
    
    hFind = FindFirstFileA(searchPath, &findFileData);
    if (hFind == INVALID_HANDLE_VALUE) {
        return false;
    }
    
    do {
        if (findFileData.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) {
            continue; // Skip directories
        }
        
        char fullPath[512];
        snprintf(fullPath, sizeof(fullPath), "%s\\%s", 
                 g_registry.plugin_directory, findFileData.cFileName);
        
        plugin_registry_load_plugin(fullPath);
    } while (FindNextFileA(hFind, &findFileData) != 0);
    
    FindClose(hFind);
#else
    DIR* dir = opendir(g_registry.plugin_directory);
    if (!dir) {
        return false;
    }
    
    struct dirent* entry;
    while ((entry = readdir(dir)) != NULL) {
        // Check if file has plugin extension
        size_t name_len = strlen(entry->d_name);
        size_t ext_len = strlen(PLUGIN_EXTENSION);
        
        if (name_len > ext_len && 
            strcmp(entry->d_name + name_len - ext_len, PLUGIN_EXTENSION) == 0) {
            
            char fullPath[512];
            snprintf(fullPath, sizeof(fullPath), "%s%s%s", 
                     g_registry.plugin_directory, PATH_SEPARATOR, entry->d_name);
            
            plugin_registry_load_plugin(fullPath);
        }
    }
    
    closedir(dir);
#endif
    
    g_registry.last_scan_time = time(NULL);
    return true;
}

/**
 * @brief Load a plugin from file
 */
bool plugin_registry_load_plugin(const char* plugin_path) {
    if (!plugin_path) {
        return false;
    }
    
    // Check if plugin is already loaded
    const char* filename = strrchr(plugin_path, PATH_SEPARATOR[0]);
    if (!filename) {
        filename = plugin_path;
    } else {
        filename++;
    }
    
    // Remove extension for plugin name
    char plugin_name[256];
    strncpy(plugin_name, filename, sizeof(plugin_name) - 1);
    char* dot = strrchr(plugin_name, '.');
    if (dot) {
        *dot = '\0';
    }
    
    // Check if already loaded
    if (plugin_registry_find_plugin(plugin_name)) {
        return true; // Already loaded
    }
    
    // Expand registry if needed
    if (g_registry.plugin_count >= g_registry.plugin_capacity) {
        size_t new_capacity = g_registry.plugin_capacity * 2;
        registered_plugin_t* new_plugins = (registered_plugin_t*)realloc(
            g_registry.plugins, new_capacity * sizeof(registered_plugin_t));
        
        if (!new_plugins) {
            return false;
        }
        
        g_registry.plugins = new_plugins;
        g_registry.plugin_capacity = new_capacity;
    }
    
    // Load plugin library
#ifdef _WIN32
    HMODULE library = LoadLibraryA(plugin_path);
    if (!library) {
        return false;
    }
    
    // Get plugin interface function
    plugin_interface_t* (*get_interface)(void) = 
        (plugin_interface_t* (*)(void))GetProcAddress(library, "get_plugin_interface");
    
    if (!get_interface) {
        FreeLibrary(library);
        return false;
    }
#else
    void* library = dlopen(plugin_path, RTLD_LAZY);
    if (!library) {
        return false;
    }
    
    // Get plugin interface function
    plugin_interface_t* (*get_interface)(void) = 
        (plugin_interface_t* (*)(void))dlsym(library, "get_plugin_interface");
    
    if (!get_interface) {
        dlclose(library);
        return false;
    }
#endif
    
    // Get plugin interface
    plugin_interface_t* interface = get_interface();
    if (!interface) {
#ifdef _WIN32
        FreeLibrary(library);
#else
        dlclose(library);
#endif
        return false;
    }
    
    // Get plugin info
    plugin_info_t* info = interface->get_plugin_info();
    if (!info) {
#ifdef _WIN32
        FreeLibrary(library);
#else
        dlclose(library);
#endif
        return false;
    }
    
    // Initialize plugin
    if (interface->init && !interface->init()) {
#ifdef _WIN32
        FreeLibrary(library);
#else
        dlclose(library);
#endif
        return false;
    }
    
    // Register plugin
    registered_plugin_t* plugin = &g_registry.plugins[g_registry.plugin_count];
    memset(plugin, 0, sizeof(registered_plugin_t));
    
    strncpy(plugin->name, info->name, sizeof(plugin->name) - 1);
    strncpy(plugin->version, info->version, sizeof(plugin->version) - 1);
    strncpy(plugin->description, info->description, sizeof(plugin->description) - 1);
    strncpy(plugin->file_path, plugin_path, sizeof(plugin->file_path) - 1);
    
    plugin->interface = interface;
    plugin->library_handle = library;
    plugin->is_loaded = true;
    plugin->is_enabled = true;
    plugin->load_time = time(NULL);
    plugin->calls_made = 0;
    plugin->errors = 0;
    
    g_registry.plugin_count++;
    return true;
}

/**
 * @brief Unload a plugin
 */
bool plugin_registry_unload_plugin(const char* plugin_name) {
    registered_plugin_t* plugin = plugin_registry_find_plugin(plugin_name);
    if (!plugin || !plugin->is_loaded) {
        return false;
    }
    
    // Cleanup plugin
    if (plugin->interface && plugin->interface->cleanup) {
        plugin->interface->cleanup();
    }
    
    // Unload library
#ifdef _WIN32
    if (plugin->library_handle) {
        FreeLibrary((HMODULE)plugin->library_handle);
    }
#else
    if (plugin->library_handle) {
        dlclose(plugin->library_handle);
    }
#endif
    
    plugin->is_loaded = false;
    plugin->is_enabled = false;
    plugin->interface = NULL;
    plugin->library_handle = NULL;
    
    return true;
}

/**
 * @brief Enable or disable a plugin
 */
bool plugin_registry_enable_plugin(const char* plugin_name, bool enable) {
    registered_plugin_t* plugin = plugin_registry_find_plugin(plugin_name);
    if (!plugin) {
        return false;
    }
    
    plugin->is_enabled = enable;
    return true;
}

/**
 * @brief Find plugin by name
 */
registered_plugin_t* plugin_registry_find_plugin(const char* plugin_name) {
    if (!plugin_name) {
        return NULL;
    }
    
    for (size_t i = 0; i < g_registry.plugin_count; i++) {
        if (strcmp(g_registry.plugins[i].name, plugin_name) == 0) {
            return &g_registry.plugins[i];
        }
    }
    
    return NULL;
}

/**
 * @brief Get all plugins
 */
bool plugin_registry_get_all_plugins(registered_plugin_t** plugins, size_t* count) {
    if (!plugins || !count) {
        return false;
    }
    
    *plugins = g_registry.plugins;
    *count = g_registry.plugin_count;
    return true;
}

/**
 * @brief Get enabled plugins
 */
bool plugin_registry_get_enabled_plugins(registered_plugin_t** plugins, size_t* count) {
    if (!plugins || !count) {
        return false;
    }
    
    // Count enabled plugins
    size_t enabled_count = 0;
    for (size_t i = 0; i < g_registry.plugin_count; i++) {
        if (g_registry.plugins[i].is_enabled) {
            enabled_count++;
        }
    }
    
    if (enabled_count == 0) {
        *plugins = NULL;
        *count = 0;
        return true;
    }
    
    // Allocate array for enabled plugins
    registered_plugin_t* enabled_plugins = (registered_plugin_t*)malloc(
        enabled_count * sizeof(registered_plugin_t));
    if (!enabled_plugins) {
        return false;
    }
    
    // Copy enabled plugins
    size_t index = 0;
    for (size_t i = 0; i < g_registry.plugin_count; i++) {
        if (g_registry.plugins[i].is_enabled) {
            memcpy(&enabled_plugins[index], &g_registry.plugins[i], sizeof(registered_plugin_t));
            index++;
        }
    }
    
    *plugins = enabled_plugins;
    *count = enabled_count;
    return true;
}

/**
 * @brief Get total plugin count
 */
size_t plugin_registry_get_plugin_count(void) {
    return g_registry.plugin_count;
}

/**
 * @brief Get enabled plugin count
 */
size_t plugin_registry_get_enabled_count(void) {
    size_t count = 0;
    for (size_t i = 0; i < g_registry.plugin_count; i++) {
        if (g_registry.plugins[i].is_enabled) {
            count++;
        }
    }
    return count;
}
