/**
 * @file file_reader_plugin.c
 * @brief File Reader Plugin for Mini MSP Agent - Platform Independent Core
 * 
 * Provides comprehensive file reading capabilities including text files,
 * binary files, and various encoding support.
 * 
 * @author Mini MSP Agent Team
 * @version 1.0.0
 * @date 2026
 */

#include "../include/plugin_interface_common.h"
#include "../include/semantic_types.h"
#include "../include/file_reader_platform.h"
#include "../include/safe_functions.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

// =============================================================================
// 📄 PLUGIN INFORMATION
// =============================================================================

static plugin_info_t g_plugin_info = {
    .name = "file_reader",
    .version = "1.0.0",
    .description = "Reads files with support for various encodings and formats",
    .author = "Mini MSP Team",
    .status = PLUGIN_STATUS_UNLOADED,
    .load_time = 0,
    .calls_made = 0
};

// =============================================================================
// 🔧 CORE PLUGIN FUNCTIONS
// =============================================================================

/**
 * @brief Initialize file reader plugin
 */
static plugin_result_t file_reader_init(void) {
    g_plugin_info.status = PLUGIN_STATUS_LOADED;
    g_plugin_info.load_time = time(NULL) * 1000; // Convert to milliseconds
    return PLUGIN_RESULT_SUCCESS;
}

/**
 * @brief Cleanup file reader plugin
 */
static plugin_result_t file_reader_cleanup(void) {
    g_plugin_info.status = PLUGIN_STATUS_UNLOADED;
    return PLUGIN_RESULT_SUCCESS;
}

/**
 * @brief Get plugin information
 */
static plugin_result_t file_reader_get_info(plugin_info_t* info) {
    if (!info) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    memcpy(info, &g_plugin_info, sizeof(plugin_info_t));
    return PLUGIN_RESULT_SUCCESS;
}

/**
 * @brief Read file content safely
 */
static plugin_result_t file_reader_read_content(const char* path, char** content, file_size_t* size) {
    if (!path || !content || !size) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    // Validate path
    if (strlen(path) == 0 || strlen(path) > 4096) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    // Check if file exists
    if (!platform_file_exists(path)) {
        return PLUGIN_RESULT_NOT_FOUND;
    }
    
    // Use platform-specific implementation
    plugin_result_t result = platform_read_file_content(path, content, size);
    if (result == PLUGIN_RESULT_SUCCESS) {
        g_plugin_info.calls_made++;
    }
    
    return result;
}

/**
 * @brief Get file metadata
 */
static plugin_result_t file_reader_get_metadata(const char* path, command_result_t* result) {
    if (!path || !result) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    // Validate path
    if (strlen(path) == 0 || strlen(path) > 4096) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    file_metadata_t metadata;
    plugin_result_t plugin_result = platform_get_file_metadata(path, &metadata);
    
    if (plugin_result != PLUGIN_RESULT_SUCCESS) {
        result->result = plugin_result;
        safe_sprintf(result->error, sizeof(result->error), 
                   "Failed to get metadata: error code %d", plugin_result);
        return plugin_result;
    }
    
    // Format metadata as JSON
    char json_buffer[1024];
    safe_result_t format_result = safe_sprintf(json_buffer, sizeof(json_buffer),
        "{"
        "\"path\":\"%s\","
        "\"name\":\"%s\","
        "\"size_bytes\":%llu,"
        "\"modification_time\":%llu,"
        "\"creation_time\":%llu,"
        "\"permissions\":\"%s\","
        "\"is_readable\":%s,"
        "\"is_writable\":%s,"
        "\"is_executable\":%s,"
        "\"is_hidden\":%s,"
        "\"is_directory\":%s"
        "}",
        metadata.m_path,
        metadata.m_name,
        (unsigned long long)metadata.m_size_bytes,
        (unsigned long long)metadata.m_modification_time,
        (unsigned long long)metadata.m_creation_time,
        metadata.m_permissions,
        metadata.m_is_readable ? "true" : "false",
        metadata.m_is_writable ? "true" : "false",
        metadata.m_is_executable ? "true" : "false",
        metadata.m_is_hidden ? "true" : "false",
        metadata.m_is_directory ? "true" : "false"
    );
    
    if (format_result != SAFE_SUCCESS) {
        result->result = PLUGIN_RESULT_ERROR;
        safe_strcpy(result->error, sizeof(result->error), "Failed to format metadata");
        return PLUGIN_RESULT_ERROR;
    }
    
    // Allocate and copy result
    result->data = strdup(json_buffer);
    if (!result->data) {
        result->result = PLUGIN_RESULT_ERROR;
        safe_strcpy(result->error, sizeof(result->error), "Memory allocation failed");
        return PLUGIN_RESULT_ERROR;
    }
    
    result->data_size = strlen(json_buffer);
    result->result = PLUGIN_RESULT_SUCCESS;
    result->error[0] = '\0';
    
    g_plugin_info.calls_made++;
    return PLUGIN_RESULT_SUCCESS;
}

/**
 * @brief Check if path exists
 */
static plugin_result_t file_reader_exists(const char* path, command_result_t* result) {
    if (!path || !result) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    bool exists = platform_file_exists(path);
    
    char json_buffer[256];
    safe_result_t format_result = safe_sprintf(json_buffer, sizeof(json_buffer),
        "{\"exists\":%s}", exists ? "true" : "false");
    
    if (format_result != SAFE_SUCCESS) {
        result->result = PLUGIN_RESULT_ERROR;
        safe_strcpy(result->error, sizeof(result->error), "Failed to format response");
        return PLUGIN_RESULT_ERROR;
    }
    
    result->data = strdup(json_buffer);
    if (!result->data) {
        result->result = PLUGIN_RESULT_ERROR;
        safe_strcpy(result->error, sizeof(result->error), "Memory allocation failed");
        return PLUGIN_RESULT_ERROR;
    }
    
    result->data_size = strlen(json_buffer);
    result->result = PLUGIN_RESULT_SUCCESS;
    result->error[0] = '\0';
    
    g_plugin_info.calls_made++;
    return PLUGIN_RESULT_SUCCESS;
}

/**
 * @brief Get file size
 */
static plugin_result_t file_reader_get_size(const char* path, command_result_t* result) {
    if (!path || !result) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    file_metadata_t metadata;
    plugin_result_t plugin_result = platform_get_file_metadata(path, &metadata);
    
    if (plugin_result != PLUGIN_RESULT_SUCCESS) {
        result->result = plugin_result;
        safe_sprintf(result->error, sizeof(result->error), 
                   "Failed to get file size: error code %d", plugin_result);
        return plugin_result;
    }
    
    char json_buffer[256];
    safe_result_t format_result = safe_sprintf(json_buffer, sizeof(json_buffer),
        "{\"size_bytes\":%llu}", (unsigned long long)metadata.m_size_bytes);
    
    if (format_result != SAFE_SUCCESS) {
        result->result = PLUGIN_RESULT_ERROR;
        safe_strcpy(result->error, sizeof(result->error), "Failed to format response");
        return PLUGIN_RESULT_ERROR;
    }
    
    result->data = strdup(json_buffer);
    if (!result->data) {
        result->result = PLUGIN_RESULT_ERROR;
        safe_strcpy(result->error, sizeof(result->error), "Memory allocation failed");
        return PLUGIN_RESULT_ERROR;
    }
    
    result->data_size = strlen(json_buffer);
    result->result = PLUGIN_RESULT_SUCCESS;
    result->error[0] = '\0';
    
    g_plugin_info.calls_made++;
    return PLUGIN_RESULT_SUCCESS;
}

/**
 * @brief Execute command
 */
static plugin_result_t file_reader_execute_command(const char* command, const char* params, command_result_t* result) {
    if (!command || !result) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    // Initialize result
    memset(result, 0, sizeof(command_result_t));
    
    if (strcmp(command, "read_file") == 0) {
        return file_reader_read_content(params, &result->data, (file_size_t*)&result->data_size);
    }
    else if (strcmp(command, "get_metadata") == 0) {
        return file_reader_get_metadata(params, result);
    }
    else if (strcmp(command, "exists") == 0) {
        return file_reader_exists(params, result);
    }
    else if (strcmp(command, "get_size") == 0) {
        return file_reader_get_size(params, result);
    }
    else {
        result->result = PLUGIN_RESULT_NOT_FOUND;
        safe_strcpy(result->error, sizeof(result->error), "Unknown command");
        return PLUGIN_RESULT_NOT_FOUND;
    }
}

/**
 * @brief Get plugin metrics
 */
static plugin_result_t file_reader_get_metrics(char* metrics, size_t buffer_size) {
    if (!metrics || buffer_size == 0) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    safe_result_t result = safe_sprintf(metrics, buffer_size,
        "{"
        "\"plugin_name\":\"file_reader\","
        "\"version\":\"%s\","
        "\"status\":\"%s\","
        "\"calls_made\":%u,"
        "\"load_time\":%llu"
        "}",
        g_plugin_info.version,
        (g_plugin_info.status == PLUGIN_STATUS_ACTIVE) ? "active" : "inactive",
        g_plugin_info.calls_made,
        (unsigned long long)g_plugin_info.load_time
    );
    
    return (result == SAFE_SUCCESS) ? PLUGIN_RESULT_SUCCESS : PLUGIN_RESULT_ERROR;
}

/**
 * @brief Set event callback (not implemented)
 */
static plugin_result_t file_reader_set_event_callback(plugin_event_callback_t callback) {
    // File reader plugin doesn't generate events
    (void)callback;
    return PLUGIN_RESULT_SUCCESS;
}

// =============================================================================
// 🔌 PLUGIN INTERFACE EXPORT
// =============================================================================

static plugin_interface_t g_file_reader_interface = {
    .init = file_reader_init,
    .cleanup = file_reader_cleanup,
    .get_info = file_reader_get_info,
    .execute_command = file_reader_execute_command,
    .get_metrics = file_reader_get_metrics,
    .set_event_callback = file_reader_set_event_callback
};

/**
 * @brief Plugin entry point
 */
__declspec(dllexport) plugin_interface_t* get_plugin_interface(void) {
    g_plugin_info.status = PLUGIN_STATUS_ACTIVE;
    return &g_file_reader_interface;
}
