#include "../include/plugin_interface_common.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Global plugin manager instance
static plugin_manager_t* g_plugin_manager = NULL;

// Common utility functions
plugin_result_t validate_plugin_name(const char* name) {
    if (!name || strlen(name) == 0 || strlen(name) >= 64) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    return PLUGIN_RESULT_SUCCESS;
}

plugin_result_t allocate_string(char** target, const char* source) {
    if (!target || !source) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    size_t len = strlen(source) + 1;
    *target = (char*)malloc(len);
    if (!*target) {
        return PLUGIN_RESULT_ERROR;
    }
    
    strcpy(*target, source);
    return PLUGIN_RESULT_SUCCESS;
}

void free_string(char** str) {
    if (str && *str) {
        free(*str);
        *str = NULL;
    }
}

plugin_result_t copy_plugin_info(plugin_info_t* dest, const plugin_info_t* src) {
    if (!dest || !src) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    memcpy(dest, src, sizeof(plugin_info_t));
    return PLUGIN_RESULT_SUCCESS;
}

// Plugin manager common functions
plugin_result_t init_plugin_manager(plugin_manager_t* manager, size_t initial_capacity) {
    if (!manager) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    manager->entries = (plugin_registry_entry_t*)calloc(initial_capacity, sizeof(plugin_registry_entry_t));
    if (!manager->entries) {
        return PLUGIN_RESULT_ERROR;
    }
    
    manager->count = 0;
    manager->capacity = initial_capacity;
    manager->event_callback = NULL;
    
    return PLUGIN_RESULT_SUCCESS;
}

void cleanup_plugin_manager(plugin_manager_t* manager) {
    if (!manager) return;
    
    if (manager->entries) {
        free(manager->entries);
        manager->entries = NULL;
    }
    
    manager->count = 0;
    manager->capacity = 0;
    manager->event_callback = NULL;
}

plugin_result_t find_plugin_entry(const plugin_manager_t* manager, const char* name, plugin_registry_entry_t** entry) {
    if (!manager || !name || !entry) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    for (size_t i = 0; i < manager->count; i++) {
        if (strcmp(manager->entries[i].name, name) == 0) {
            *entry = &manager->entries[i];
            return PLUGIN_RESULT_SUCCESS;
        }
    }
    
    return PLUGIN_RESULT_NOT_FOUND;
}

void trigger_plugin_event(plugin_event_type_t event_type, const char* plugin_name, const void* event_data) {
    if (g_plugin_manager && g_plugin_manager->event_callback) {
        g_plugin_manager->event_callback(event_type, plugin_name, event_data);
    }
}
