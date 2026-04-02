/**
 * @file file_reader_plugin_linux.c
 * @brief Linux-specific implementation for File Reader Plugin
 * 
 * Linux platform specific file operations
 * 
 * @author Mini MSP Agent Team
 * @version 1.0.0
 * @date 2026
 */

#include "../../include/plugin_interface_common.h"
#include "../../include/semantic_types.h"
#include "../../include/file_reader_platform.h"
#include <unistd.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// =============================================================================
// 🐧 LINUX FILE OPERATIONS
// =============================================================================

/**
 * @brief Read file content on Linux
 */
plugin_result_t linux_read_file_content(const char* path, char** content, file_size_t* size) {
    if (!path || !content || !size) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    *content = NULL;
    *size = 0;
    
    int fd = open(path, O_RDONLY);
    if (fd == -1) {
        return PLUGIN_RESULT_PERMISSION_DENIED;
    }
    
    struct stat st;
    if (fstat(fd, &st) == -1) {
        close(fd);
        return PLUGIN_RESULT_ERROR;
    }
    
    if (!S_ISREG(st.st_mode)) {
        close(fd);
        return PLUGIN_RESULT_PERMISSION_DENIED;
    }
    
    if (st.st_size > 1024 * 1024 * 100) { // 100MB limit
        close(fd);
        return PLUGIN_RESULT_PERMISSION_DENIED;
    }
    
    *content = (char*)malloc(st.st_size + 1);
    if (!*content) {
        close(fd);
        return PLUGIN_RESULT_ERROR;
    }
    
    ssize_t bytesRead = read(fd, *content, st.st_size);
    if (bytesRead == -1) {
        free(*content);
        *content = NULL;
        close(fd);
        return PLUGIN_RESULT_ERROR;
    }
    
    (*content)[bytesRead] = '\0';
    *size = bytesRead;
    
    close(fd);
    return PLUGIN_RESULT_SUCCESS;
}

/**
 * @brief Get file metadata on Linux
 */
plugin_result_t linux_get_file_metadata(const char* path, file_metadata_t* metadata) {
    if (!path || !metadata) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    memset(metadata, 0, sizeof(file_metadata_t));
    
    struct stat st;
    if (stat(path, &st) == -1) {
        return PLUGIN_RESULT_NOT_FOUND;
    }
    
    // Extract filename from path
    const char* filename = strrchr(path, '/');
    if (!filename) filename = path;
    else filename++;
    
    // Safe copy of filename
    strncpy(metadata->m_name, filename, sizeof(metadata->m_name) - 1);
    metadata->m_name[sizeof(metadata->m_name) - 1] = '\0';
    
    // Copy full path
    strncpy(metadata->m_path, path, sizeof(metadata->m_path) - 1);
    metadata->m_path[sizeof(metadata->m_path) - 1] = '\0';
    
    metadata->m_size_bytes = st.st_size;
    metadata->m_modification_time = st.st_mtime * 1000; // Convert to milliseconds
    metadata->m_creation_time = st.st_ctime * 1000;
    
    // File permissions
    metadata->m_is_readable = (st.st_mode & S_IRUSR) != 0;
    metadata->m_is_writable = (st.st_mode & S_IWUSR) != 0;
    metadata->m_is_executable = (st.st_mode & S_IXUSR) != 0;
    metadata->m_is_directory = S_ISDIR(st.st_mode);
    metadata->m_is_hidden = (filename[0] == '.');
    
    // Format permissions in rwx format
    snprintf(metadata->m_permissions, sizeof(metadata->m_permissions),
             "%s%s%s%s%s%s%s%s%s%s",
             (st.st_mode & S_IRUSR) ? "r" : "-",
             (st.st_mode & S_IWUSR) ? "w" : "-",
             (st.st_mode & S_IXUSR) ? "x" : "-",
             (st.st_mode & S_IRGRP) ? "r" : "-",
             (st.st_mode & S_IWGRP) ? "w" : "-",
             (st.st_mode & S_IXGRP) ? "x" : "-",
             (st.st_mode & S_IROTH) ? "r" : "-",
             (st.st_mode & S_IWOTH) ? "w" : "-",
             (st.st_mode & S_IXOTH) ? "x" : "-");
    
    return PLUGIN_RESULT_SUCCESS;
}

/**
 * @brief Check if file exists on Linux
 */
bool linux_file_exists(const char* path) {
    if (!path) return false;
    
    struct stat st;
    return (stat(path, &st) == 0 && S_ISREG(st.st_mode));
}

/**
 * @brief Check if path is directory on Linux
 */
bool linux_is_directory(const char* path) {
    if (!path) return false;
    
    struct stat st;
    return (stat(path, &st) == 0 && S_ISDIR(st.st_mode));
}
