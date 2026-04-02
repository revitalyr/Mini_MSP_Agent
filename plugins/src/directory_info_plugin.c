#include "../include/plugin_interface_common.h"
#include "../include/semantic_types.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

// Plugin state
typedef struct {
    directory_stats_t m_current_stats;
    directory_entry_t* m_entries;
    size_t m_entry_count;
    size_t m_entry_capacity;
    path_string_t m_scanned_path;
    timestamp_t m_last_scan_time;
    call_count_t m_scan_calls_made;
} directory_info_state_t;

// Global plugin state
static directory_info_state_t g_plugin_state = {0};

// Plugin information
static plugin_info_t g_plugin_info = {
    .name = "directory_info",
    .version = "1.0.0",
    .description = "Provides comprehensive directory information including file counts, sizes, and metadata",
    .author = "Mini MSP Team",
    .status = PLUGIN_STATUS_UNLOADED,
    .load_time = 0,
    .calls_made = 0
};

// Helper functions
static plugin_result_t allocate_path_string(path_string_t* target, const char* source) {
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

static void free_path_string(path_string_t* str) {
    if (str && *str) {
        free(*str);
        *str = NULL;
    }
}

// Plugin interface functions
static plugin_result_t directory_info_init(void) {
    memset(&g_plugin_state, 0, sizeof(g_plugin_state));
    g_plugin_state.m_entry_capacity = 1000;
    g_plugin_state.m_entries = (directory_entry_t*)malloc(
        g_plugin_state.m_entry_capacity * sizeof(directory_entry_t));
    
    if (!g_plugin_state.m_entries) {
        return PLUGIN_RESULT_ERROR;
    }
    
    g_plugin_info.status = PLUGIN_STATUS_LOADED;
    g_plugin_info.load_time = time(NULL) * 1000;
    
    return PLUGIN_RESULT_SUCCESS;
}

static plugin_result_t directory_info_cleanup(void) {
    if (g_plugin_state.m_entries) {
        for (size_t i = 0; i < g_plugin_state.m_entry_count; i++) {
            free_path_string(&g_plugin_state.m_entries[i].m_name);
            free_path_string(&g_plugin_state.m_entries[i].m_full_path);
        }
        free(g_plugin_state.m_entries);
        g_plugin_state.m_entries = NULL;
    }
    
    free_path_string(&g_plugin_state.m_scanned_path);
    memset(&g_plugin_state, 0, sizeof(g_plugin_state));
    
    g_plugin_info.status = PLUGIN_STATUS_UNLOADED;
    return PLUGIN_RESULT_SUCCESS;
}

static plugin_result_t directory_info_get_info(plugin_info_t* info) {
    if (!info) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    memcpy(info, &g_plugin_info, sizeof(plugin_info_t));
    return PLUGIN_RESULT_SUCCESS;
}

static plugin_result_t directory_info_execute_command(const char* command, const char* params, command_result_t* result) {
    if (!command || !result) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    g_plugin_info.calls_made++;
    
    if (strcmp(command, "scan_directory") == 0) {
        if (!params) {
            result->result = PLUGIN_RESULT_INVALID_PARAM;
            strncpy(result->error, "Directory path required", sizeof(result->error) - 1);
            return PLUGIN_RESULT_INVALID_PARAM;
        }
        
        // TODO: Implement directory scanning logic
        result->result = PLUGIN_RESULT_SUCCESS;
        result->data = strdup("Directory scanned successfully");
        result->data_size = strlen(result->data);
        
    } else {
        result->result = PLUGIN_RESULT_NOT_FOUND;
        strncpy(result->error, "Unknown command", sizeof(result->error) - 1);
        return PLUGIN_RESULT_NOT_FOUND;
    }
    
    return PLUGIN_RESULT_SUCCESS;
}

static plugin_result_t directory_info_get_metrics(char* metrics, size_t buffer_size) {
    if (!metrics || buffer_size == 0) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    snprintf(metrics, buffer_size,
        "{"
        "\"scan_calls\":%u,"
        "\"entries_scanned\":%zu,"
        "\"last_scan_time\":%llu"
        "}",
        g_plugin_state.m_scan_calls_made,
        g_plugin_state.m_entry_count,
        (unsigned long long)g_plugin_state.m_last_scan_time
    );
    
    return PLUGIN_RESULT_SUCCESS;
}

static plugin_result_t directory_info_set_event_callback(plugin_event_callback_t callback) {
    // TODO: Store callback for event notifications
    return PLUGIN_RESULT_SUCCESS;
}

// Plugin interface export
plugin_interface_t* get_plugin_interface(void) {
    static plugin_interface_t interface = {
        .init = directory_info_init,
        .cleanup = directory_info_cleanup,
        .get_info = directory_info_get_info,
        .execute_command = directory_info_execute_command,
        .get_metrics = directory_info_get_metrics,
        .set_event_callback = directory_info_set_event_callback
    };
    
    return &interface;
}
