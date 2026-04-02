/**
 * @file file_reader_plugin_windows.c
 * @brief Windows-specific implementation for File Reader Plugin
 * 
 * Windows platform specific file operations
 * 
 * @author Mini MSP Agent Team
 * @version 1.0.0
 * @date 2026
 */

#include "../../include/plugin_interface_common.h"
#include "../../include/semantic_types.h"
#include "../../include/file_reader_platform.h"
#include <windows.h>
#include <io.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// =============================================================================
// 🪟 WINDOWS FILE OPERATIONS
// =============================================================================

/**
 * @brief Read file content on Windows
 */
plugin_result_t windows_read_file_content(const char* path, char** content, file_size_t* size) {
    if (!path || !content || !size) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    *content = NULL;
    *size = 0;
    
    HANDLE hFile = CreateFileA(path, GENERIC_READ, FILE_SHARE_READ, NULL, 
                             OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    if (hFile == INVALID_HANDLE_VALUE) {
        return PLUGIN_RESULT_PERMISSION_DENIED;
    }
    
    LARGE_INTEGER fileSize;
    if (!GetFileSizeEx(hFile, &fileSize)) {
        CloseHandle(hFile);
        return PLUGIN_RESULT_ERROR;
    }
    
    if (fileSize.QuadPart > 1024 * 1024 * 100) { // 100MB limit
        CloseHandle(hFile);
        return PLUGIN_RESULT_PERMISSION_DENIED;
    }
    
    *content = (char*)malloc(fileSize.QuadPart + 1);
    if (!*content) {
        CloseHandle(hFile);
        return PLUGIN_RESULT_ERROR;
    }
    
    DWORD bytesRead;
    if (!ReadFile(hFile, *content, (DWORD)fileSize.QuadPart, &bytesRead, NULL)) {
        free(*content);
        *content = NULL;
        CloseHandle(hFile);
        return PLUGIN_RESULT_ERROR;
    }
    
    (*content)[bytesRead] = '\0';
    *size = bytesRead;
    
    CloseHandle(hFile);
    return PLUGIN_RESULT_SUCCESS;
}

/**
 * @brief Get file metadata on Windows
 */
plugin_result_t windows_get_file_metadata(const char* path, file_metadata_t* metadata) {
    if (!path || !metadata) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    memset(metadata, 0, sizeof(file_metadata_t));
    
    WIN32_FILE_ATTRIBUTE_DATA fileData;
    if (!GetFileAttributesExA(path, GetFileExInfoStandard, &fileData)) {
        return PLUGIN_RESULT_NOT_FOUND;
    }
    
    // Extract filename from path
    const char* filename = strrchr(path, '\\');
    if (!filename) filename = strrchr(path, '/');
    if (!filename) filename = path;
    else filename++;
    
    // Safe copy of filename
    strncpy(metadata->m_name, filename, sizeof(metadata->m_name) - 1);
    metadata->m_name[sizeof(metadata->m_name) - 1] = '\0';
    
    // Copy full path
    strncpy(metadata->m_path, path, sizeof(metadata->m_path) - 1);
    metadata->m_path[sizeof(metadata->m_path) - 1] = '\0';
    
    metadata->m_size_bytes = ((uint64_t)fileData.nFileSizeHigh << 32) | fileData.nFileSizeLow;
    
    // Convert FILETIME to timestamp
    LARGE_INTEGER modified, created;
    modified.LowPart = fileData.ftLastWriteTime.dwLowDateTime;
    modified.HighPart = fileData.ftLastWriteTime.dwHighDateTime;
    created.LowPart = fileData.ftCreationTime.dwLowDateTime;
    created.HighPart = fileData.ftCreationTime.dwHighDateTime;
    
    // Convert Windows FILETIME (100ns intervals since Jan 1, 1601) to Unix timestamp
    metadata->m_modification_time = (modified.QuadPart - 116444736000000000LL) / 10000LL;
    metadata->m_creation_time = (created.QuadPart - 116444736000000000LL) / 10000LL;
    
    // File permissions
    DWORD attributes = fileData.dwFileAttributes;
    metadata->m_is_readable = !(attributes & FILE_ATTRIBUTE_READONLY);
    metadata->m_is_writable = !(attributes & FILE_ATTRIBUTE_READONLY);
    metadata->m_is_hidden = (attributes & FILE_ATTRIBUTE_HIDDEN) != 0;
    metadata->m_is_directory = (attributes & FILE_ATTRIBUTE_DIRECTORY) != 0;
    
    // Default permissions
    strncpy(metadata->m_permissions, "rw-r--r--", sizeof(metadata->m_permissions) - 1);
    metadata->m_permissions[sizeof(metadata->m_permissions) - 1] = '\0';
    
    return PLUGIN_RESULT_SUCCESS;
}

/**
 * @brief Check if file exists on Windows
 */
bool windows_file_exists(const char* path) {
    if (!path) return false;
    
    DWORD attributes = GetFileAttributesA(path);
    return (attributes != INVALID_FILE_ATTRIBUTES && 
            !(attributes & FILE_ATTRIBUTE_DIRECTORY));
}

/**
 * @brief Check if path is directory on Windows
 */
bool windows_is_directory(const char* path) {
    if (!path) return false;
    
    DWORD attributes = GetFileAttributesA(path);
    return (attributes != INVALID_FILE_ATTRIBUTES && 
            (attributes & FILE_ATTRIBUTE_DIRECTORY));
}
