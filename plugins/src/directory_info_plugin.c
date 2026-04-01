/**
 * @file directory_info_plugin.c
 * @brief Directory Information Plugin for Mini MSP Agent
 * 
 * Provides comprehensive directory information including file counts,
 * sizes, permissions, and metadata for cross-platform support.
 * 
 * @author Mini MSP Agent Team
 * @version 1.0.0
 * @date 2026
 */

#include "../include/plugin_interface.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#ifdef _WIN32
#include <windows.h>
#include <tchar.h>
#include <strsafe.h>
#else
#include <dirent.h>
#include <unistd.h>
#include <pwd.h>
#include <grp.h>
#endif

// Plugin information
static plugin_info_t directory_info_plugin_info = {
    .name = "directory_info",
    .version = "1.0.0",
    .description = "Provides detailed directory information and statistics"
};

/**
 * @brief Directory entry information structure
 */
typedef struct {
    char name[256];
    char path[512];
    uint64_t size;
    bool is_directory;
    bool is_hidden;
    uint64_t modified_time;
    uint32_t permissions;
} directory_entry_t;

/**
 * @brief Directory statistics structure
 */
typedef struct {
    uint32_t total_files;
    uint32_t total_directories;
    uint64_t total_size;
    uint32_t hidden_files;
    uint32_t hidden_directories;
    char path[512];
} directory_stats_t;

// Plugin implementation
static bool directory_info_init(void) {
    return true;
}

static void directory_info_cleanup(void) {
    // No cleanup needed
}

static plugin_info_t* directory_info_get_plugin_info(void) {
    return &directory_info_plugin_info;
}

static bool directory_info_get_system_metrics(system_metrics_t* metrics) {
    // Not applicable for directory info plugin
    return false;
}

static bool directory_info_get_processes(process_info_t** processes, size_t* count) {
    // Not applicable for directory info plugin
    return false;
}

static bool directory_info_execute_command(const char* command, command_result_t* result) {
    // Not applicable for directory info plugin
    return false;
}

static bool directory_info_read_file(const char* path, file_content_t* content) {
    // Not applicable for directory info plugin
    return false;
}

static bool directory_info_get_system_info(system_info_t* info) {
    // Not applicable for directory info plugin
    return false;
}

/**
 * @brief Get directory statistics
 */
static bool get_directory_stats(const char* path, directory_stats_t* stats) {
    if (!path || !stats) return false;
    
    memset(stats, 0, sizeof(directory_stats_t));
    strncpy(stats->path, path, sizeof(stats->path) - 1);
    
#ifdef _WIN32
    WIN32_FIND_DATAA findFileData;
    HANDLE hFind = INVALID_HANDLE_VALUE;
    char searchPath[512];
    
    snprintf(searchPath, sizeof(searchPath), "%s\\*", path);
    
    hFind = FindFirstFileA(searchPath, &findFileData);
    if (hFind == INVALID_HANDLE_VALUE) {
        return false;
    }
    
    do {
        if (strcmp(findFileData.cFileName, ".") != 0 && 
            strcmp(findFileData.cFileName, "..") != 0) {
            
            if (findFileData.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) {
                stats->total_directories++;
                if (findFileData.dwFileAttributes & FILE_ATTRIBUTE_HIDDEN) {
                    stats->hidden_directories++;
                }
            } else {
                stats->total_files++;
                if (findFileData.dwFileAttributes & FILE_ATTRIBUTE_HIDDEN) {
                    stats->hidden_files++;
                }
                stats->total_size += ((uint64_t)findFileData.nFileSizeHigh << 32) | 
                                  findFileData.nFileSizeLow;
            }
        }
    } while (FindNextFileA(hFind, &findFileData) != 0);
    
    FindClose(hFind);
#else
    DIR* dir = opendir(path);
    if (!dir) {
        return false;
    }
    
    struct dirent* entry;
    struct stat st;
    char fullPath[1024];
    
    while ((entry = readdir(dir)) != NULL) {
        if (strcmp(entry->d_name, ".") != 0 && 
            strcmp(entry->d_name, "..") != 0) {
            
            snprintf(fullPath, sizeof(fullPath), "%s/%s", path, entry->d_name);
            
            if (stat(fullPath, &st) == 0) {
                if (S_ISDIR(st.st_mode)) {
                    stats->total_directories++;
                    if (entry->d_name[0] == '.') {
                        stats->hidden_directories++;
                    }
                } else {
                    stats->total_files++;
                    if (entry->d_name[0] == '.') {
                        stats->hidden_files++;
                    }
                    stats->total_size += st.st_size;
                }
            }
        }
    }
    
    closedir(dir);
#endif
    
    return true;
}

/**
 * @brief List directory entries
 */
static bool list_directory_entries(const char* path, directory_entry_t** entries, size_t* count) {
    if (!path || !entries || !count) return false;
    
    // First pass: count entries
    size_t entryCount = 0;
    
#ifdef _WIN32
    WIN32_FIND_DATAA findFileData;
    HANDLE hFind = INVALID_HANDLE_VALUE;
    char searchPath[512];
    
    snprintf(searchPath, sizeof(searchPath), "%s\\*", path);
    
    hFind = FindFirstFileA(searchPath, &findFileData);
    if (hFind == INVALID_HANDLE_VALUE) {
        return false;
    }
    
    do {
        if (strcmp(findFileData.cFileName, ".") != 0 && 
            strcmp(findFileData.cFileName, "..") != 0) {
            entryCount++;
        }
    } while (FindNextFileA(hFind, &findFileData) != 0);
    
    FindClose(hFind);
    
    // Allocate memory for entries
    *entries = (directory_entry_t*)malloc(entryCount * sizeof(directory_entry_t));
    if (!*entries) {
        return false;
    }
    
    // Second pass: fill entries
    hFind = FindFirstFileA(searchPath, &findFileData);
    size_t index = 0;
    
    do {
        if (strcmp(findFileData.cFileName, ".") != 0 && 
            strcmp(findFileData.cFileName, "..") != 0) {
            
            directory_entry_t* entry = &(*entries)[index];
            memset(entry, 0, sizeof(directory_entry_t));
            
            strncpy(entry->name, findFileData.cFileName, sizeof(entry->name) - 1);
            snprintf(entry->path, sizeof(entry->path), "%s\\%s", path, findFileData.cFileName);
            
            entry->is_directory = (findFileData.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0;
            entry->is_hidden = (findFileData.dwFileAttributes & FILE_ATTRIBUTE_HIDDEN) != 0;
            entry->size = ((uint64_t)findFileData.nFileSizeHigh << 32) | findFileData.nFileSizeLow;
            
            // Convert FILETIME to Unix timestamp
            ULARGE_INTEGER ull;
            ull.LowPart = findFileData.ftLastWriteTime.dwLowDateTime;
            ull.HighPart = findFileData.ftLastWriteTime.dwHighDateTime;
            entry->modified_time = (ull.QuadPart - 116444736000000000ULL) / 10000000ULL;
            
            entry->permissions = findFileData.dwFileAttributes;
            
            index++;
        }
    } while (FindNextFileA(hFind, &findFileData) != 0);
    
    FindClose(hFind);
#else
    DIR* dir = opendir(path);
    if (!dir) {
        return false;
    }
    
    struct dirent* entry;
    struct stat st;
    char fullPath[1024];
    
    // Count entries
    while ((entry = readdir(dir)) != NULL) {
        if (strcmp(entry->d_name, ".") != 0 && 
            strcmp(entry->d_name, "..") != 0) {
            entryCount++;
        }
    }
    
    // Allocate memory
    *entries = (directory_entry_t*)malloc(entryCount * sizeof(directory_entry_t));
    if (!*entries) {
        closedir(dir);
        return false;
    }
    
    // Fill entries
    rewinddir(dir);
    size_t index = 0;
    
    while ((entry = readdir(dir)) != NULL) {
        if (strcmp(entry->d_name, ".") != 0 && 
            strcmp(entry->d_name, "..") != 0) {
            
            snprintf(fullPath, sizeof(fullPath), "%s/%s", path, entry->d_name);
            
            if (stat(fullPath, &st) == 0) {
                directory_entry_t* dirEntry = &(*entries)[index];
                memset(dirEntry, 0, sizeof(directory_entry_t));
                
                strncpy(dirEntry->name, entry->d_name, sizeof(dirEntry->name) - 1);
                strncpy(dirEntry->path, fullPath, sizeof(dirEntry->path) - 1);
                
                dirEntry->is_directory = S_ISDIR(st.st_mode);
                dirEntry->is_hidden = (entry->d_name[0] == '.');
                dirEntry->size = st.st_size;
                dirEntry->modified_time = st.st_mtime;
                dirEntry->permissions = st.st_mode;
                
                index++;
            }
        }
    }
    
    closedir(dir);
#endif
    
    *count = entryCount;
    return true;
}

static void directory_info_free_memory(void* ptr) {
    if (ptr) {
        free(ptr);
    }
}

// Plugin interface
static plugin_interface_t directory_info_interface = {
    .get_plugin_info = directory_info_get_plugin_info,
    .init = directory_info_init,
    .cleanup = directory_info_cleanup,
    .get_system_metrics = directory_info_get_system_metrics,
    .get_processes = directory_info_get_processes,
    .execute_command = directory_info_execute_command,
    .read_file = directory_info_read_file,
    .get_system_info = directory_info_get_system_info,
    .free_memory = directory_info_free_memory
};

// Plugin entry point
PLUGIN_EXPORT plugin_interface_t* PLUGIN_CALL get_plugin_interface(void) {
    return &directory_info_interface;
}
