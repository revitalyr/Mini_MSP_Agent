#include "../include/safe_functions.h"
#include <string.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdbool.h>

// =============================================================================
// 🛡️ SAFE STRING IMPLEMENTATIONS
// =============================================================================

safe_result_t safe_strcpy(char* dest, size_t dest_size, const char* src) {
    if (!dest || !src || dest_size == 0) {
        return SAFE_ERROR_NULL_POINTER;
    }
    
    size_t src_len = strlen(src);
    if (src_len >= dest_size) {
        return SAFE_ERROR_BUFFER_TOO_SMALL;
    }
    
    strncpy(dest, src, dest_size - 1);
    dest[dest_size - 1] = '\0'; // Ensure null termination
    
    return SAFE_SUCCESS;
}

safe_result_t safe_strcat(char* dest, size_t dest_size, const char* src) {
    if (!dest || !src || dest_size == 0) {
        return SAFE_ERROR_NULL_POINTER;
    }
    
    size_t dest_len = strlen(dest);
    size_t src_len = strlen(src);
    
    if (dest_len + src_len >= dest_size) {
        return SAFE_ERROR_BUFFER_TOO_SMALL;
    }
    
    strncat(dest, src, dest_size - dest_len - 1);
    
    return SAFE_SUCCESS;
}

safe_result_t safe_sprintf(char* dest, size_t dest_size, const char* format, ...) {
    if (!dest || !format || dest_size == 0) {
        return SAFE_ERROR_NULL_POINTER;
    }
    
    va_list args;
    va_start(args, format);
    
    int result = vsnprintf(dest, dest_size, format, args);
    va_end(args);
    
    if (result < 0) {
        return SAFE_ERROR_INVALID_PARAM; // Formatting error
    }
    
    if ((size_t)result >= dest_size) {
        return SAFE_ERROR_BUFFER_TOO_SMALL; // Truncated
    }
    
    return SAFE_SUCCESS;
}

bool safe_strlen_check(const char* str, size_t max_len) {
    if (!str) {
        return false;
    }
    
    size_t len = strlen(str);
    return len <= max_len;
}
