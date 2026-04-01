/**
 * @file root_directory_info_plugin.c
 * @brief Root Directory Information Plugin for Mini MSP Agent
 * 
 * Provides comprehensive root directory information including drive
 * information, mount points, and volume statistics.
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

#ifdef _WIN32
#include <windows.h>
#include <tchar.h>
#include <fileapi.h>
#else
#include <sys/statvfs.h>
#include <mntent.h>
#include <unistd.h>
#include <sys/stat.h>
#endif

// Plugin information
static plugin_info_t root_directory_info_plugin_info = {
    .name = "root_directory_info",
    .version = "1.0.0",
    .description = "Provides root directory and volume information"
};

/**
 * @brief Volume information structure
 */
typedef struct {
    char root_path[512];
    char volume_label[256];
    char filesystem_type[64];
    uint64_t total_space;
    uint64_t free_space;
    uint64_t used_space;
    uint32_t serial_number;
    bool is_removable;
    bool is_system;
    char mount_options[256];
} volume_info_t;

/**
 * @brief Drive information structure (Windows specific)
 */
typedef struct {
    char drive_letter[4];
    char drive_type[32];
    uint64_t total_space;
    uint64_t free_space;
    bool is_ready;
    char volume_label[256];
    char filesystem[32];
} drive_info_t;

// Plugin implementation
static bool root_directory_info_init(void) {
    return true;
}

static void root_directory_info_cleanup(void) {
    // No cleanup needed
}

static plugin_info_t* root_directory_info_get_plugin_info(void) {
    return &root_directory_info_plugin_info;
}

static bool root_directory_info_get_system_metrics(system_metrics_t* metrics) {
    // Not applicable for root directory info plugin
    return false;
}

static bool root_directory_info_get_processes(process_info_t** processes, size_t* count) {
    // Not applicable for root directory info plugin
    return false;
}

static bool root_directory_info_execute_command(const char* command, command_result_t* result) {
    // Not applicable for root directory info plugin
    return false;
}

static bool root_directory_info_read_file(const char* path, file_content_t* content) {
    // Not applicable for root directory info plugin
    return false;
}

static bool root_directory_info_get_system_info(system_info_t* info) {
    // Not applicable for root directory info plugin
    return false;
}

#ifdef _WIN32
/**
 * @brief Get drive type string
 */
static const char* get_drive_type_string(UINT drive_type) {
    switch (drive_type) {
        case DRIVE_UNKNOWN:
            return "Unknown";
        case DRIVE_NO_ROOT_DIR:
            return "No Root Directory";
        case DRIVE_REMOVABLE:
            return "Removable";
        case DRIVE_FIXED:
            return "Fixed";
        case DRIVE_REMOTE:
            return "Remote";
        case DRIVE_CDROM:
            return "CD-ROM";
        case DRIVE_RAMDISK:
            return "RAM Disk";
        default:
            return "Unknown";
    }
}

/**
 * @brief Get Windows drive information
 */
static bool get_windows_drive_info(const char* drive, drive_info_t* info) {
    memset(info, 0, sizeof(drive_info_t));
    strncpy(info->drive_letter, drive, sizeof(info->drive_letter) - 1);
    
    // Get drive type
    UINT drive_type = GetDriveTypeA(drive);
    strncpy(info->drive_type, get_drive_type_string(drive_type), 
           sizeof(info->drive_type) - 1);
    
    // Check if drive is ready
    if (drive_type != DRIVE_NO_ROOT_DIR && drive_type != DRIVE_UNKNOWN) {
        info->is_ready = true;
        
        // Get volume information
        char volume_label[256] = {0};
        char filesystem[32] = {0};
        DWORD serial_number, max_component_length, filesystem_flags;
        
        if (GetVolumeInformationA(drive, volume_label, sizeof(volume_label),
                                &serial_number, &max_component_length,
                                &filesystem_flags, filesystem, sizeof(filesystem))) {
            strncpy(info->volume_label, volume_label, sizeof(info->volume_label) - 1);
            strncpy(info->filesystem, filesystem, sizeof(info->filesystem) - 1);
        }
        
        // Get disk space
        ULARGE_INTEGER free_bytes_available, total_bytes, free_bytes;
        if (GetDiskFreeSpaceExA(drive, &free_bytes_available, &total_bytes, &free_bytes)) {
            info->total_space = total_bytes.QuadPart;
            info->free_space = free_bytes.QuadPart;
        }
    } else {
        info->is_ready = false;
    }
    
    return true;
}

/**
 * @brief Get all Windows drives
 */
static bool get_windows_drives(drive_info_t** drives, size_t* count) {
    DWORD drives = GetLogicalDrives();
    size_t drive_count = 0;
    
    // Count available drives
    for (int i = 0; i < 26; i++) {
        if (drives & (1 << i)) {
            drive_count++;
        }
    }
    
    if (drive_count == 0) {
        *drives = NULL;
        *count = 0;
        return true;
    }
    
    // Allocate memory for drives
    *drives = (drive_info_t*)malloc(drive_count * sizeof(drive_info_t));
    if (!*drives) {
        return false;
    }
    
    // Fill drive information
    size_t index = 0;
    for (int i = 0; i < 26; i++) {
        if (drives & (1 << i)) {
            char drive_path[4];
            snprintf(drive_path, sizeof(drive_path), "%c:\\", 'A' + i);
            
            get_windows_drive_info(drive_path, &(*drives)[index]);
            index++;
        }
    }
    
    *count = drive_count;
    return true;
}

#else
/**
 * @brief Get Linux mount information
 */
static bool get_linux_mounts(volume_info_t** volumes, size_t* count) {
    FILE* mounts = setmntent("/proc/mounts", "r");
    if (!mounts) {
        return false;
    }
    
    // Count mount points
    size_t mount_count = 0;
    struct mntent* entry;
    while ((entry = getmntent(mounts)) != NULL) {
        mount_count++;
    }
    
    if (mount_count == 0) {
        *volumes = NULL;
        *count = 0;
        endmntent(mounts);
        return true;
    }
    
    // Allocate memory for volumes
    *volumes = (volume_info_t*)malloc(mount_count * sizeof(volume_info_t));
    if (!*volumes) {
        endmntent(mounts);
        return false;
    }
    
    // Fill volume information
    rewind(mounts);
    size_t index = 0;
    
    while ((entry = getmntent(mounts)) != NULL && index < mount_count) {
        volume_info_t* volume = &(*volumes)[index];
        memset(volume, 0, sizeof(volume_info_t));
        
        strncpy(volume->root_path, entry->mnt_dir, sizeof(volume->root_path) - 1);
        strncpy(volume->filesystem_type, entry->mnt_type, sizeof(volume->filesystem_type) - 1);
        
        if (entry->mnt_opts) {
            strncpy(volume->mount_options, entry->mnt_opts, sizeof(volume->mount_options) - 1);
        }
        
        // Get filesystem statistics
        struct statvfs fs_stats;
        if (statvfs(entry->mnt_dir, &fs_stats) == 0) {
            volume->total_space = fs_stats.f_blocks * fs_stats.f_frsize;
            volume->free_space = fs_stats.f_bfree * fs_stats.f_frsize;
            volume->used_space = volume->total_space - volume->free_space;
        }
        
        // Determine if removable
        volume->is_removable = (strstr(entry->mnt_dir, "/media/") != NULL) ||
                              (strstr(entry->mnt_dir, "/mnt/") != NULL);
        
        // Determine if system
        volume->is_system = (strcmp(entry->mnt_dir, "/") == 0) ||
                           (strstr(entry->mnt_dir, "/boot") != NULL) ||
                           (strstr(entry->mnt_dir, "/sys") != NULL) ||
                           (strstr(entry->mnt_dir, "/proc") != NULL);
        
        index++;
    }
    
    *count = index;
    endmntent(mounts);
    return true;
}
#endif

/**
 * @brief Get root directory information
 */
static bool get_root_directory_info(volume_info_t** volumes, size_t* count) {
    if (!volumes || !count) return false;
    
#ifdef _WIN32
    // Get Windows drive information
    drive_info_t* drives = NULL;
    size_t drive_count = 0;
    
    if (!get_windows_drives(&drives, &drive_count)) {
        return false;
    }
    
    // Convert drive info to volume info
    *volumes = (volume_info_t*)malloc(drive_count * sizeof(volume_info_t));
    if (!*volumes) {
        free(drives);
        return false;
    }
    
    for (size_t i = 0; i < drive_count; i++) {
        volume_info_t* volume = &(*volumes)[i];
        memset(volume, 0, sizeof(volume_info_t));
        
        strncpy(volume->root_path, drives[i].drive_letter, sizeof(volume->root_path) - 1);
        strncpy(volume->volume_label, drives[i].volume_label, sizeof(volume->volume_label) - 1);
        strncpy(volume->filesystem_type, drives[i].filesystem, sizeof(volume->filesystem_type) - 1);
        
        volume->total_space = drives[i].total_space;
        volume->free_space = drives[i].free_space;
        volume->used_space = drives[i].total_space - drives[i].free_space;
        
        volume->is_removable = (strcmp(drives[i].drive_type, "Removable") == 0) ||
                              (strcmp(drives[i].drive_type, "CD-ROM") == 0);
        
        volume->is_system = (strcmp(drives[i].drive_letter, "C:\\") == 0);
    }
    
    *count = drive_count;
    free(drives);
    
#else
    // Get Linux mount information
    return get_linux_mounts(volumes, count);
#endif
}

/**
 * @brief Get volume by path
 */
static bool get_volume_by_path(const char* path, volume_info_t* volume) {
    if (!path || !volume) return false;
    
    volume_info_t* volumes = NULL;
    size_t count = 0;
    
    if (!get_root_directory_info(&volumes, &count)) {
        return false;
    }
    
    // Find the volume that contains the path
    size_t best_match = 0;
    size_t best_match_length = 0;
    
    for (size_t i = 0; i < count; i++) {
        size_t path_length = strlen(volumes[i].root_path);
        if (strncmp(path, volumes[i].root_path, path_length) == 0) {
            if (path_length > best_match_length) {
                best_match = i;
                best_match_length = path_length;
            }
        }
    }
    
    if (best_match_length > 0) {
        memcpy(volume, &volumes[best_match], sizeof(volume_info_t));
        free(volumes);
        return true;
    }
    
    free(volumes);
    return false;
}

/**
 * @brief Get system root directory
 */
static bool get_system_root_directory(char* root_path, size_t buffer_size) {
    if (!root_path || buffer_size == 0) return false;
    
#ifdef _WIN32
    // Get Windows system directory
    GetSystemDirectoryA(root_path, (DWORD)buffer_size);
    
    // Extract just the drive letter and root
    if (strlen(root_path) >= 2) {
        root_path[2] = '\0';
        root_path[3] = '\\';
        root_path[4] = '\0';
    } else {
        strcpy(root_path, "C:\\");
    }
#else
    // Linux root is always /
    strncpy(root_path, "/", buffer_size - 1);
    root_path[buffer_size - 1] = '\0';
#endif
    
    return true;
}

static void root_directory_info_free_memory(void* ptr) {
    if (ptr) {
        free(ptr);
    }
}

// Plugin interface
static plugin_interface_t root_directory_info_interface = {
    .get_plugin_info = root_directory_info_get_plugin_info,
    .init = root_directory_info_init,
    .cleanup = root_directory_info_cleanup,
    .get_system_metrics = root_directory_info_get_system_metrics,
    .get_processes = root_directory_info_get_processes,
    .execute_command = root_directory_info_execute_command,
    .read_file = root_directory_info_read_file,
    .get_system_info = root_directory_info_get_system_info,
    .free_memory = root_directory_info_free_memory
};

// Plugin entry point
PLUGIN_EXPORT plugin_interface_t* PLUGIN_CALL get_plugin_interface(void) {
    return &root_directory_info_interface;
}
