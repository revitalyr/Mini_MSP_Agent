#ifndef SAFE_FUNCTIONS_H
#define SAFE_FUNCTIONS_H

#include <stddef.h>
#include <stdarg.h>
#include <stdbool.h>

// Safe string operations with bounds checking
typedef enum {
    SAFE_SUCCESS = 0,
    SAFE_ERROR_NULL_POINTER,
    SAFE_ERROR_BUFFER_TOO_SMALL,
    SAFE_ERROR_INVALID_PARAM
} safe_result_t;

// Safe string copy
safe_result_t safe_strcpy(char* dest, size_t dest_size, const char* src);

// Safe string concatenation  
safe_result_t safe_strcat(char* dest, size_t dest_size, const char* src);

// Safe formatted string
safe_result_t safe_sprintf(char* dest, size_t dest_size, const char* format, ...);

// Safe string length check
bool safe_strlen_check(const char* str, size_t max_len);

#endif // SAFE_FUNCTIONS_H
