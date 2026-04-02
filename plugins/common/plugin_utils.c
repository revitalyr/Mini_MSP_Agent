#include "../include/plugin_interface_common.h"
#include "../include/semantic_types.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>
#include <time.h>

// =============================================================================
// 🛡️ SAFE STRING UTILITIES
// =============================================================================

// Safe string copy with bounds checking
static plugin_result_t safe_strcpy(char* dest, size_t dest_size, const char* src) {
    if (!dest || !src || dest_size == 0) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    size_t src_len = strlen(src);
    if (src_len >= dest_size) {
        return PLUGIN_RESULT_INVALID_PARAM; // Buffer too small
    }
    
    strncpy(dest, src, dest_size - 1);
    dest[dest_size - 1] = '\0'; // Ensure null termination
    
    return PLUGIN_RESULT_SUCCESS;
}

// Safe string concatenation with bounds checking
static plugin_result_t safe_strcat(char* dest, size_t dest_size, const char* src) {
    if (!dest || !src || dest_size == 0) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    size_t dest_len = strlen(dest);
    size_t src_len = strlen(src);
    
    if (dest_len + src_len >= dest_size) {
        return PLUGIN_RESULT_INVALID_PARAM; // Buffer too small
    }
    
    // Use safe_strcat instead of strncat
    plugin_result_t result = safe_strcat(dest, dest_size, src);
    return result;
}

// Safe formatted string with bounds checking
static plugin_result_t safe_sprintf(char* dest, size_t dest_size, const char* format, ...) {
    if (!dest || !format || dest_size == 0) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    va_list args;
    va_start(args, format);
    
    int result = vsnprintf(dest, dest_size, format, args);
    va_end(args);
    
    if (result < 0) {
        return PLUGIN_RESULT_ERROR; // Formatting error
    }
    
    if ((size_t)result >= dest_size) {
        return PLUGIN_RESULT_INVALID_PARAM; // Truncated
    }
    
    return PLUGIN_RESULT_SUCCESS;
}

// =============================================================================
// 🌐 GLOBAL PLUGIN MANAGER
// =============================================================================

// Global plugin manager instance
static plugin_manager_t* g_plugin_manager = NULL;

// =============================================================================
// 🔍 VALIDATION FUNCTIONS
// =============================================================================

plugin_result_t validate_plugin_name(const char* name) {
    if (!name) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    size_t len = strlen(name);
    if (len == 0 || len >= sizeof(((plugin_info_t*)0)->name)) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    // Check for valid characters (alphanumeric, underscore, hyphen)
    for (size_t i = 0; i < len; i++) {
        char c = name[i];
        if (!((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || 
              (c >= '0' && c <= '9') || c == '_' || c == '-')) {
            return PLUGIN_RESULT_INVALID_PARAM;
        }
    }
    
    return PLUGIN_RESULT_SUCCESS;
}

plugin_result_t validate_path_string(const path_string_t path) {
    if (!path) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    size_t len = strlen(path);
    if (len == 0 || len > 4096) { // Reasonable path limit
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    // Basic path validation - prevent directory traversal
    if (strstr(path, "..") != NULL) {
        return PLUGIN_RESULT_PERMISSION_DENIED;
    }
    
    return PLUGIN_RESULT_SUCCESS;
}

plugin_result_t validate_buffer_size(file_size_t requested_size, file_size_t max_size) {
    if (requested_size == 0) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    if (requested_size > max_size) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    // Check for reasonable limits (prevent memory exhaustion)
    const file_size_t MAX_REASONABLE_SIZE = 1024 * 1024 * 1024; // 1GB
    if (requested_size > MAX_REASONABLE_SIZE) {
        return PLUGIN_RESULT_PERMISSION_DENIED;
    }
    
    return PLUGIN_RESULT_SUCCESS;
}

// =============================================================================
// 💾 SAFE MEMORY MANAGEMENT
// =============================================================================

plugin_result_t allocate_string(char** target, const char* source) {
    if (!target || !source) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    *target = NULL; // Ensure pointer is NULL on failure
    
    size_t len = strlen(source);
    if (len == 0) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    // Allocate with extra space for null terminator
    char* new_str = (char*)malloc(len + 1);
    if (!new_str) {
        return PLUGIN_RESULT_ERROR;
    }
    
    // Safe copy
    plugin_result_t copy_result = safe_strcpy(new_str, len + 1, source);
    if (copy_result != PLUGIN_RESULT_SUCCESS) {
        free(new_str);
        return copy_result;
    }
    
    *target = new_str;
    return PLUGIN_RESULT_SUCCESS;
}

plugin_result_t allocate_buffer(data_buffer_t* buffer, file_size_t size) {
    if (!buffer) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    // Validate size
    plugin_result_t validation = validate_buffer_size(size, 1024 * 1024); // 1MB default max
    if (validation != PLUGIN_RESULT_SUCCESS) {
        return validation;
    }
    
    // Initialize buffer
    buffer->m_data = NULL;
    buffer->m_size = 0;
    buffer->m_capacity = 0;
    buffer->m_is_dynamic = true;
    
    if (size > 0) {
        void* data = malloc(size);
        if (!data) {
            return PLUGIN_RESULT_ERROR;
        }
        
        buffer->m_data = data;
        buffer->m_capacity = size;
        buffer->m_size = 0;
    }
    
    return PLUGIN_RESULT_SUCCESS;
}

void free_string(char** str) {
    if (str && *str) {
        free(*str);
        *str = NULL;
    }
}

void free_buffer(data_buffer_t* buffer) {
    if (buffer) {
        if (buffer->m_is_dynamic && buffer->m_data) {
            // Clear sensitive data before free
            memset(buffer->m_data, 0, buffer->m_capacity);
            free(buffer->m_data);
        }
        buffer->m_data = NULL;
        buffer->m_size = 0;
        buffer->m_capacity = 0;
        buffer->m_is_dynamic = false;
    }
}

// =============================================================================
// 📋 SAFE COPY OPERATIONS
// =============================================================================

plugin_result_t copy_plugin_info(plugin_info_t* dest, const plugin_info_t* src) {
    if (!dest || !src) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    // Use memcpy for binary copy - it's safe for POD types
    memcpy(dest, src, sizeof(plugin_info_t));
    
    return PLUGIN_RESULT_SUCCESS;
}

plugin_result_t copy_directory_entry(directory_entry_t* dest, const directory_entry_t* src) {
    if (!dest || !src) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    // Copy structure
    memcpy(dest, src, sizeof(directory_entry_t));
    
    // Deep copy strings safely
    plugin_result_t result;
    
    result = allocate_string(&dest->m_name, src->m_name);
    if (result != PLUGIN_RESULT_SUCCESS) {
        return result;
    }
    
    result = allocate_string(&dest->m_full_path, src->m_full_path);
    if (result != PLUGIN_RESULT_SUCCESS) {
        free_string(&dest->m_name);
        return result;
    }
    
    return PLUGIN_RESULT_SUCCESS;
}

void cleanup_directory_entry(directory_entry_t* entry) {
    if (entry) {
        free_string(&entry->m_name);
        free_string(&entry->m_full_path);
        memset(entry, 0, sizeof(directory_entry_t));
    }
}

// =============================================================================
// 🎛️ PLUGIN MANAGER OPERATIONS
// =============================================================================

plugin_result_t init_plugin_manager(plugin_manager_t* manager, size_t initial_capacity) {
    if (!manager) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    if (initial_capacity == 0 || initial_capacity > 1000) { // Reasonable limit
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    // Initialize to zero first
    memset(manager, 0, sizeof(plugin_manager_t));
    
    // Allocate entries array
    plugin_registry_entry_t* entries = (plugin_registry_entry_t*)calloc(
        initial_capacity, sizeof(plugin_registry_entry_t));
    if (!entries) {
        return PLUGIN_RESULT_ERROR;
    }
    
    manager->entries = entries;
    manager->count = 0;
    manager->capacity = initial_capacity;
    manager->event_callback = NULL;
    
    return PLUGIN_RESULT_SUCCESS;
}

void cleanup_plugin_manager(plugin_manager_t* manager) {
    if (!manager) return;
    
    if (manager->entries) {
        // Clean up each entry
        for (size_t i = 0; i < manager->count; i++) {
            plugin_registry_entry_t* entry = &manager->entries[i];
            // name is a fixed array, no need to free
            // library_path is also a fixed array, no need to free
            
            if (entry->interface) {
                // Platform-specific cleanup would go here
                entry->interface = NULL;
            }
        }
        
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
    
    // Validate name
    if (validate_plugin_name(name) != PLUGIN_RESULT_SUCCESS) {
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

// =============================================================================
// 📡 EVENT HANDLING
// =============================================================================

void trigger_plugin_event(plugin_event_type_t event_type, const char* plugin_name, const void* event_data) {
    if (!g_plugin_manager || !g_plugin_manager->event_callback) {
        return;
    }
    
    // Validate plugin name if provided
    if (plugin_name && validate_plugin_name(plugin_name) != PLUGIN_RESULT_SUCCESS) {
        return;
    }
    
    // Call event handler safely
    g_plugin_manager->event_callback(event_type, plugin_name, event_data);
}

plugin_result_t register_event_callback(plugin_manager_t* manager, plugin_event_callback_t callback) {
    if (!manager || !callback) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    manager->event_callback = callback;
    g_plugin_manager = manager; // Update global reference
    
    return PLUGIN_RESULT_SUCCESS;
}

// =============================================================================
// 🔧 UTILITY FUNCTIONS
// =============================================================================

plugin_result_t format_error_message(error_message_t* message, const char* format, ...) {
    if (!message || !format) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    char buffer[512]; // Reasonable error message limit
    
    va_list args;
    va_start(args, format);
    
    int result = vsnprintf(buffer, sizeof(buffer), format, args);
    va_end(args);
    
    if (result < 0 || result >= (int)sizeof(buffer)) {
        return PLUGIN_RESULT_ERROR;
    }
    
    return allocate_string(message, buffer);
}

plugin_result_t get_current_timestamp(timestamp_t* timestamp) {
    if (!timestamp) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    // Platform-independent timestamp
    *timestamp = (timestamp_t)time(NULL) * 1000; // Convert to milliseconds
    
    return PLUGIN_RESULT_SUCCESS;
}

bool is_safe_numeric_value(long value, long min_val, long max_val) {
    return (value >= min_val && value <= max_val);
}

// =============================================================================
// 🔒 SECURITY FUNCTIONS
// =============================================================================

plugin_result_t sanitize_path_string(path_string_t path) {
    if (!path) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    size_t len = strlen(path);
    if (len == 0 || len > 4096) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    // Remove dangerous characters
    for (size_t i = 0; i < len; i++) {
        char c = path[i];
        
        // Reject control characters and dangerous symbols
        if (c < 32 || c > 126) {
            return PLUGIN_RESULT_PERMISSION_DENIED;
        }
        
        // Reject path traversal attempts
        if (i > 0 && path[i-1] == '.' && c == '.') {
            return PLUGIN_RESULT_PERMISSION_DENIED;
        }
    }
    
    return PLUGIN_RESULT_SUCCESS;
}
