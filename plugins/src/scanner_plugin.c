/**
 * @file scanner_plugin.c
 * @brief Scanner Plugin for Mini MSP Agent
 * 
 * Provides comprehensive file and directory scanning capabilities
 * including pattern matching, content search, and metadata extraction.
 * 
 * @author Mini MSP Agent Team
 * @version 1.0.0
 * @date 2026
 */

#include "../include/plugin_interface.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <time.h>
#include <regex.h>

#ifdef _WIN32
#include <windows.h>
#include <tchar.h>
#include <strsafe.h>
#else
#include <dirent.h>
#include <unistd.h>
#include <fnmatch.h>
#include <sys/stat.h>
#include <pwd.h>
#include <grp.h>
#endif

// Plugin information
static plugin_info_t scanner_plugin_info = {
    .name = "scanner",
    .version = "1.0.0",
    .description = "Provides comprehensive file and directory scanning capabilities"
};

/**
 * @brief Scan result structure
 */
typedef struct {
    char path[512];
    char name[256];
    uint64_t size;
    bool is_directory;
    bool is_hidden;
    uint64_t modified_time;
    uint64_t created_time;
    uint32_t permissions;
    char owner[64];
    char group[64];
    char file_extension[32];
    bool matches_pattern;
    bool matches_content;
} scan_result_t;

/**
 * @brief Scan configuration
 */
typedef struct {
    char root_path[512];
    bool recursive;
    bool include_hidden;
    bool follow_symlinks;
    char file_pattern[256];
    char content_pattern[256];
    bool case_sensitive;
    uint32_t max_depth;
    uint32_t max_results;
    uint64_t min_file_size;
    uint64_t max_file_size;
    time_t modified_after;
    time_t modified_before;
} scan_config_t;

/**
 * @brief Scan statistics
 */
typedef struct {
    uint32_t files_scanned;
    uint32_t directories_scanned;
    uint32_t files_matched;
    uint32_t directories_matched;
    uint64_t total_size;
    uint32_t errors_encountered;
    time_t scan_start_time;
    time_t scan_end_time;
} scan_stats_t;

// Plugin state
static scan_config_t current_scan_config;
static scan_stats_t current_scan_stats;
static scan_result_t* scan_results = NULL;
static size_t scan_results_count = 0;
static bool scan_active = false;

// Plugin implementation
static bool scanner_init(void) {
    memset(&current_scan_config, 0, sizeof(scan_config_t));
    memset(&current_scan_stats, 0, sizeof(scan_stats_t));
    scan_active = false;
    return true;
}

static void scanner_cleanup(void) {
    if (scan_results) {
        free(scan_results);
        scan_results = NULL;
    }
    scan_results_count = 0;
    scan_active = false;
}

static plugin_info_t* scanner_get_plugin_info(void) {
    return &scanner_plugin_info;
}

static bool scanner_get_system_metrics(system_metrics_t* metrics) {
    // Not applicable for scanner plugin
    return false;
}

static bool scanner_get_processes(process_info_t** processes, size_t* count) {
    // Not applicable for scanner plugin
    return false;
}

static bool scanner_execute_command(const char* command, command_result_t* result) {
    // Not applicable for scanner plugin
    return false;
}

static bool scanner_read_file(const char* path, file_content_t* content) {
    // Not applicable for scanner plugin
    return false;
}

static bool scanner_get_system_info(system_info_t* info) {
    // Not applicable for scanner plugin
    return false;
}

/**
 * @brief Check if file matches pattern
 */
static bool matches_file_pattern(const char* filename, const char* pattern, bool case_sensitive) {
    if (!pattern || strlen(pattern) == 0) {
        return true; // No pattern means match all
    }
    
#ifdef _WIN32
    // Windows pattern matching (simplified)
    if (case_sensitive) {
        return PathMatchSpecA(filename, pattern);
    } else {
        char lower_filename[256];
        char lower_pattern[256];
        
        strncpy(lower_filename, filename, sizeof(lower_filename) - 1);
        strncpy(lower_pattern, pattern, sizeof(lower_pattern) - 1);
        
        for (int i = 0; lower_filename[i]; i++) {
            lower_filename[i] = tolower(lower_filename[i]);
        }
        for (int i = 0; lower_pattern[i]; i++) {
            lower_pattern[i] = tolower(lower_pattern[i]);
        }
        
        return PathMatchSpecA(lower_filename, lower_pattern);
    }
#else
    // Unix pattern matching
    int flags = case_sensitive ? 0 : FNM_CASEFOLD;
    return fnmatch(pattern, filename, flags) == 0;
#endif
}

/**
 * @brief Check if file content matches pattern
 */
static bool matches_content_pattern(const char* filepath, const char* pattern, bool case_sensitive) {
    if (!pattern || strlen(pattern) == 0) {
        return false; // No content pattern means no content match
    }
    
    FILE* fp = fopen(filepath, "rb");
    if (!fp) {
        return false;
    }
    
    // Read file in chunks to avoid loading large files entirely
    char buffer[8192];
    size_t bytes_read;
    bool found = false;
    
    regex_t regex;
    int regex_flags = case_sensitive ? 0 : REG_ICASE;
    
    if (regcomp(&regex, pattern, regex_flags) != 0) {
        fclose(fp);
        return false;
    }
    
    while ((bytes_read = fread(buffer, 1, sizeof(buffer), fp)) > 0) {
        // Search for pattern in buffer
        if (regexec(&regex, buffer, 0, NULL, 0) == 0) {
            found = true;
            break;
        }
        
        // Handle pattern spanning across buffer boundaries (simplified)
        if (bytes_read < sizeof(buffer)) {
            break; // End of file
        }
    }
    
    regfree(&regex);
    fclose(fp);
    return found;
}

/**
 * @brief Extract file extension
 */
static void extract_file_extension(const char* filename, char* extension, size_t extension_size) {
    const char* dot = strrchr(filename, '.');
    if (dot && dot != filename) {
        strncpy(extension, dot + 1, extension_size - 1);
        extension[extension_size - 1] = '\0';
    } else {
        extension[0] = '\0';
    }
}

/**
 * @brief Get file owner and group (Unix only)
 */
#ifdef _WIN32
static void get_file_owner_group(const char* path, char* owner, char* group) {
    // Windows doesn't have simple owner/group concept like Unix
    strcpy(owner, "Unknown");
    strcpy(group, "Unknown");
}
#else
static void get_file_owner_group(const char* path, char* owner, char* group) {
    struct stat st;
    if (stat(path, &st) == 0) {
        struct passwd* pwd = getpwuid(st.st_uid);
        struct group* grp = getgrgid(st.st_gid);
        
        if (pwd) {
            strncpy(owner, pwd->pw_name, 63);
            owner[63] = '\0';
        } else {
            snprintf(owner, 64, "%d", st.st_uid);
        }
        
        if (grp) {
            strncpy(group, grp->gr_name, 63);
            group[63] = '\0';
        } else {
            snprintf(group, 64, "%d", st.st_gid);
        }
    } else {
        strcpy(owner, "Unknown");
        strcpy(group, "Unknown");
    }
}
#endif

/**
 * @brief Scan directory recursively
 */
static bool scan_directory_recursive(const char* path, uint32_t current_depth) {
    if (scan_active == false) {
        return false; // Scan was cancelled
    }
    
    if (current_depth > current_scan_config.max_depth) {
        return true; // Max depth reached
    }
    
#ifdef _WIN32
    WIN32_FIND_DATAA findFileData;
    HANDLE hFind = INVALID_HANDLE_VALUE;
    char searchPath[512];
    
    snprintf(searchPath, sizeof(searchPath), "%s\\*", path);
    
    hFind = FindFirstFileA(searchPath, &findFileData);
    if (hFind == INVALID_HANDLE_VALUE) {
        current_scan_stats.errors_encountered++;
        return false;
    }
    
    do {
        if (strcmp(findFileData.cFileName, ".") != 0 && 
            strcmp(findFileData.cFileName, "..") != 0) {
            
            current_scan_stats.files_scanned++;
            
            // Check if hidden
            bool is_hidden = (findFileData.dwFileAttributes & FILE_ATTRIBUTE_HIDDEN) != 0;
            
            if (!current_scan_config.include_hidden && is_hidden) {
                continue;
            }
            
            char fullPath[512];
            snprintf(fullPath, sizeof(fullPath), "%s\\%s", path, findFileData.cFileName);
            
            bool is_directory = (findFileData.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0;
            
            if (is_directory) {
                current_scan_stats.directories_scanned++;
                
                // Add directory result if it matches
                if (scan_results_count < current_scan_config.max_results) {
                    scan_result_t* result = &scan_results[scan_results_count];
                    memset(result, 0, sizeof(scan_result_t));
                    
                    strncpy(result->path, fullPath, sizeof(result->path) - 1);
                    strncpy(result->name, findFileData.cFileName, sizeof(result->name) - 1);
                    result->is_directory = true;
                    result->is_hidden = is_hidden;
                    
                    // Convert FILETIME to Unix timestamp
                    ULARGE_INTEGER ull;
                    ull.LowPart = findFileData.ftLastWriteTime.dwLowDateTime;
                    ull.HighPart = findFileData.ftLastWriteTime.dwHighDateTime;
                    result->modified_time = (ull.QuadPart - 116444736000000000ULL) / 10000000ULL;
                    
                    result->permissions = findFileData.dwFileAttributes;
                    get_file_owner_group(fullPath, result->owner, result->group);
                    
                    result->matches_pattern = matches_file_pattern(findFileData.cFileName, 
                                                              current_scan_config.file_pattern, 
                                                              current_scan_config.case_sensitive);
                    
                    scan_results_count++;
                }
                
                // Recursively scan subdirectory
                if (current_scan_config.recursive) {
                    scan_directory_recursive(fullPath, current_depth + 1);
                }
            } else {
                // Process file
                uint64_t file_size = ((uint64_t)findFileData.nFileSizeHigh << 32) | 
                                  findFileData.nFileSizeLow;
                
                // Check size constraints
                if (current_scan_config.min_file_size > 0 && 
                    file_size < current_scan_config.min_file_size) {
                    continue;
                }
                
                if (current_scan_config.max_file_size > 0 && 
                    file_size > current_scan_config.max_file_size) {
                    continue;
                }
                
                // Check time constraints
                ULARGE_INTEGER ull;
                ull.LowPart = findFileData.ftLastWriteTime.dwLowDateTime;
                ull.HighPart = findFileData.ftLastWriteTime.dwHighDateTime;
                time_t modified_time = (ull.QuadPart - 116444736000000000ULL) / 10000000ULL;
                
                if (current_scan_config.modified_after > 0 && 
                    modified_time < current_scan_config.modified_after) {
                    continue;
                }
                
                if (current_scan_config.modified_before > 0 && 
                    modified_time > current_scan_config.modified_before) {
                    continue;
                }
                
                current_scan_stats.total_size += file_size;
                
                // Add file result if it matches
                if (scan_results_count < current_scan_config.max_results) {
                    scan_result_t* result = &scan_results[scan_results_count];
                    memset(result, 0, sizeof(scan_result_t));
                    
                    strncpy(result->path, fullPath, sizeof(result->path) - 1);
                    strncpy(result->name, findFileData.cFileName, sizeof(result->name) - 1);
                    result->size = file_size;
                    result->is_directory = false;
                    result->is_hidden = is_hidden;
                    result->modified_time = modified_time;
                    result->permissions = findFileData.dwFileAttributes;
                    
                    get_file_owner_group(fullPath, result->owner, result->group);
                    extract_file_extension(findFileData.cFileName, result->file_extension, 
                                      sizeof(result->file_extension));
                    
                    result->matches_pattern = matches_file_pattern(findFileData.cFileName, 
                                                              current_scan_config.file_pattern, 
                                                              current_scan_config.case_sensitive);
                    
                    result->matches_content = matches_content_pattern(fullPath, 
                                                                current_scan_config.content_pattern, 
                                                                current_scan_config.case_sensitive);
                    
                    if (result->matches_pattern || result->matches_content) {
                        current_scan_stats.files_matched++;
                    }
                    
                    scan_results_count++;
                }
            }
        }
    } while (FindNextFileA(hFind, &findFileData) != 0);
    
    FindClose(hFind);
    
#else
    DIR* dir = opendir(path);
    if (!dir) {
        current_scan_stats.errors_encountered++;
        return false;
    }
    
    struct dirent* entry;
    struct stat st;
    char fullPath[1024];
    
    while ((entry = readdir(dir)) != NULL && scan_active) {
        if (strcmp(entry->d_name, ".") != 0 && 
            strcmp(entry->d_name, "..") != 0) {
            
            current_scan_stats.files_scanned++;
            
            // Check if hidden
            bool is_hidden = (entry->d_name[0] == '.');
            
            if (!current_scan_config.include_hidden && is_hidden) {
                continue;
            }
            
            snprintf(fullPath, sizeof(fullPath), "%s/%s", path, entry->d_name);
            
            if (stat(fullPath, &st) == 0) {
                bool is_directory = S_ISDIR(st.st_mode);
                
                if (is_directory) {
                    current_scan_stats.directories_scanned++;
                    
                    // Add directory result
                    if (scan_results_count < current_scan_config.max_results) {
                        scan_result_t* result = &scan_results[scan_results_count];
                        memset(result, 0, sizeof(scan_result_t));
                        
                        strncpy(result->path, fullPath, sizeof(result->path) - 1);
                        strncpy(result->name, entry->d_name, sizeof(result->name) - 1);
                        result->is_directory = true;
                        result->is_hidden = is_hidden;
                        result->modified_time = st.st_mtime;
                        result->permissions = st.st_mode;
                        
                        get_file_owner_group(fullPath, result->owner, result->group);
                        
                        result->matches_pattern = matches_file_pattern(entry->d_name, 
                                                                  current_scan_config.file_pattern, 
                                                                  current_scan_config.case_sensitive);
                        
                        scan_results_count++;
                    }
                    
                    // Recursively scan subdirectory
                    if (current_scan_config.recursive) {
                        scan_directory_recursive(fullPath, current_depth + 1);
                    }
                } else {
                    // Process file
                    // Check size constraints
                    if (current_scan_config.min_file_size > 0 && 
                        st.st_size < current_scan_config.min_file_size) {
                        continue;
                    }
                    
                    if (current_scan_config.max_file_size > 0 && 
                        st.st_size > current_scan_config.max_file_size) {
                        continue;
                    }
                    
                    // Check time constraints
                    if (current_scan_config.modified_after > 0 && 
                        st.st_mtime < current_scan_config.modified_after) {
                        continue;
                    }
                    
                    if (current_scan_config.modified_before > 0 && 
                        st.st_mtime > current_scan_config.modified_before) {
                        continue;
                    }
                    
                    current_scan_stats.total_size += st.st_size;
                    
                    // Add file result
                    if (scan_results_count < current_scan_config.max_results) {
                        scan_result_t* result = &scan_results[scan_results_count];
                        memset(result, 0, sizeof(scan_result_t));
                        
                        strncpy(result->path, fullPath, sizeof(result->path) - 1);
                        strncpy(result->name, entry->d_name, sizeof(result->name) - 1);
                        result->size = st.st_size;
                        result->is_directory = false;
                        result->is_hidden = is_hidden;
                        result->modified_time = st.st_mtime;
                        result->permissions = st.st_mode;
                        
                        get_file_owner_group(fullPath, result->owner, result->group);
                        extract_file_extension(entry->d_name, result->file_extension, 
                                          sizeof(result->file_extension));
                        
                        result->matches_pattern = matches_file_pattern(entry->d_name, 
                                                                  current_scan_config.file_pattern, 
                                                                  current_scan_config.case_sensitive);
                        
                        result->matches_content = matches_content_pattern(fullPath, 
                                                                    current_scan_config.content_pattern, 
                                                                    current_scan_config.case_sensitive);
                        
                        if (result->matches_pattern || result->matches_content) {
                            current_scan_stats.files_matched++;
                        }
                        
                        scan_results_count++;
                    }
                }
            }
        }
    }
    
    closedir(dir);
#endif
    
    return true;
}

/**
 * @brief Start scan with configuration
 */
static bool start_scan(const scan_config_t* config) {
    if (!config || scan_active) {
        return false;
    }
    
    // Copy configuration
    memcpy(&current_scan_config, config, sizeof(scan_config_t));
    
    // Initialize scan statistics
    memset(&current_scan_stats, 0, sizeof(scan_stats_t));
    current_scan_stats.scan_start_time = time(NULL);
    
    // Allocate results buffer
    if (scan_results) {
        free(scan_results);
    }
    
    scan_results = (scan_result_t*)malloc(config->max_results * sizeof(scan_result_t));
    if (!scan_results) {
        return false;
    }
    
    scan_results_count = 0;
    scan_active = true;
    
    // Start scanning
    bool success = scan_directory_recursive(config->root_path, 0);
    
    current_scan_stats.scan_end_time = time(NULL);
    scan_active = false;
    
    return success;
}

/**
 * @brief Cancel active scan
 */
static bool cancel_scan(void) {
    scan_active = false;
    return true;
}

/**
 * @brief Get scan results
 */
static bool get_scan_results(scan_result_t** results, size_t* count) {
    if (!results || !count) return false;
    
    *results = scan_results;
    *count = scan_results_count;
    return true;
}

/**
 * @brief Get scan statistics
 */
static bool get_scan_statistics(scan_stats_t* stats) {
    if (!stats) return false;
    
    memcpy(stats, &current_scan_stats, sizeof(scan_stats_t));
    return true;
}

static void scanner_free_memory(void* ptr) {
    if (ptr) {
        free(ptr);
    }
}

// Plugin interface
static plugin_interface_t scanner_interface = {
    .get_plugin_info = scanner_get_plugin_info,
    .init = scanner_init,
    .cleanup = scanner_cleanup,
    .get_system_metrics = scanner_get_system_metrics,
    .get_processes = scanner_get_processes,
    .execute_command = scanner_execute_command,
    .read_file = scanner_read_file,
    .get_system_info = scanner_get_system_info,
    .free_memory = scanner_free_memory
};

// Plugin entry point
PLUGIN_EXPORT plugin_interface_t* PLUGIN_CALL get_plugin_interface(void) {
    return &scanner_interface;
}
