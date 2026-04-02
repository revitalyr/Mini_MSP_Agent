/**
 * @file file_reader_plugin_macos.c
 * @brief macOS-specific implementation for File Reader Plugin
 */

#include "../../include/plugin_interface_common.h"
#include "../../include/semantic_types.h"
#include "../../include/file_reader_platform.h"
#include <sys/stat.h>
#include <unistd.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

plugin_result_t macos_file_exists(const char* filepath) {
    if (!filepath) return PLUGIN_RESULT_INVALID_PARAM;
    
    struct stat st;
    if (stat(filepath, &st) == 0) {
        return PLUGIN_RESULT_SUCCESS;
    }
    return PLUGIN_RESULT_ERROR;
}

plugin_result_t macos_get_file_size(const char* filepath, file_size_t* size) {
    if (!filepath || !size) return PLUGIN_RESULT_INVALID_PARAM;
    
    struct stat st;
    if (stat(filepath, &st) == 0) {
        *size = st.st_size;
        return PLUGIN_RESULT_SUCCESS;
    }
    return PLUGIN_RESULT_ERROR;
}

plugin_result_t macos_get_file_modified_time(const char* filepath, timestamp_t* timestamp) {
    if (!filepath || !timestamp) return PLUGIN_RESULT_INVALID_PARAM;
    
    struct stat st;
    if (stat(filepath, &st) == 0) {
        *timestamp = st.st_mtime * 1000; // Convert to milliseconds
        return PLUGIN_RESULT_SUCCESS;
    }
    return PLUGIN_RESULT_ERROR;
}

plugin_result_t macos_read_file_content(const char* filepath, char* buffer, size_t buffer_size, size_t* bytes_read) {
    if (!filepath || !buffer || !bytes_read) return PLUGIN_RESULT_INVALID_PARAM;
    
    int fd = open(filepath, O_RDONLY);
    if (fd == -1) return PLUGIN_RESULT_ERROR;
    
    ssize_t read_bytes = read(fd, buffer, buffer_size - 1);
    close(fd);
    
    if (read_bytes == -1) return PLUGIN_RESULT_ERROR;
    
    buffer[read_bytes] = '\0';
    *bytes_read = (size_t)read_bytes;
    
    return PLUGIN_RESULT_SUCCESS;
}

plugin_result_t macos_check_file_permissions(const char* filepath, uint32_t* permissions) {
    if (!filepath || !permissions) return PLUGIN_RESULT_INVALID_PARAM;
    
    struct stat st;
    if (stat(filepath, &st) == 0) {
        *permissions = st.st_mode & 0777; // Get rwx permissions
        return PLUGIN_RESULT_SUCCESS;
    }
    return PLUGIN_RESULT_ERROR;
}
