/**
 * @file system_plugin.c
 * @brief Simple C system plugin for Mini MSP Agent
 * 
 * This plugin provides basic system metrics and information using C only.
 * No C++ dependencies, pure C implementation.
 */

#include "../include/plugin_interface.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/sysinfo.h>
#include <sys/utsname.h>
#include <sys/statvfs.h>

// Plugin information
static const char* PLUGIN_NAME = "system_plugin";
static const char* PLUGIN_VERSION = "1.0.0";
static const char* PLUGIN_DESCRIPTION = "Simple C system metrics plugin";

// Plugin implementation functions
static plugin_info_t* get_plugin_info_impl(void);
static bool plugin_init_impl(void);
static void plugin_cleanup_impl(void);
static bool get_system_metrics_impl(system_metrics_t* metrics);
static bool get_processes_impl(process_info_t** processes, size_t* count);
static bool execute_command_impl(const char* command, command_result_t* result);
static bool read_file_impl(const char* path, file_content_t* content);
static bool get_system_info_impl(system_info_t* info);
static void free_memory_impl(void* ptr);

// Get system uptime in seconds
static uint64_t get_system_uptime(void) {
    struct sysinfo si;
    if (sysinfo(&si) == 0) {
        return (uint64_t)si.uptime;
    }
    return 0;
}

// Get CPU usage (simplified version)
static float get_cpu_usage(void) {
    // This is a simplified implementation
    // In a real scenario, you would parse /proc/stat for accurate CPU usage
    FILE* fp = fopen("/proc/loadavg", "r");
    if (fp) {
        float load1;
        if (fscanf(fp, "%f", &load1) == 1) {
            fclose(fp);
            // Convert load average to percentage (rough approximation)
            return load1 * 100.0f;
        }
        fclose(fp);
    }
    return 0.0f;
}

// Get memory usage percentage
static float get_memory_usage(void) {
    struct sysinfo si;
    if (sysinfo(&si) == 0) {
        if (si.totalram > 0) {
            return (float)(si.totalram - si.freeram) * 100.0f / (float)si.totalram;
        }
    }
    return 0.0f;
}

// Get disk usage percentage for root filesystem
static float get_disk_usage(void) {
    struct statvfs vfs;
    if (statvfs("/", &vfs) == 0) {
        if (vfs.f_blocks > 0) {
            return (float)(vfs.f_blocks - vfs.f_bfree) * 100.0f / (float)vfs.f_blocks;
        }
    }
    return 0.0f;
}

// Plugin function implementations
static plugin_info_t* get_plugin_info_impl(void) {
    static plugin_info_t info;
    info.name = PLUGIN_NAME;
    info.version = PLUGIN_VERSION;
    info.description = PLUGIN_DESCRIPTION;
    return &info;
}

static bool plugin_init_impl(void) {
    // Initialize plugin - nothing special needed for this simple plugin
    return true;
}

static void plugin_cleanup_impl(void) {
    // Cleanup resources - nothing needed for this simple plugin
}

static bool get_system_metrics_impl(system_metrics_t* metrics) {
    if (!metrics) return false;
    
    // Initialize metrics
    memset(metrics, 0, sizeof(system_metrics_t));
    
    // Get hostname
    if (gethostname(metrics->hostname, sizeof(metrics->hostname) - 1) != 0) {
        strncpy(metrics->hostname, "unknown", sizeof(metrics->hostname) - 1);
    }
    metrics->hostname[sizeof(metrics->hostname) - 1] = '\0';
    
    // Get system metrics
    metrics->cpu_usage = get_cpu_usage();
    metrics->ram_usage = get_memory_usage();
    metrics->disk_usage = get_disk_usage();
    metrics->uptime = get_system_uptime();
    
    return true;
}

static bool get_processes_impl(process_info_t** processes, size_t* count) {
    // Simplified implementation - return empty list
    // In a real implementation, you would parse /proc to get process information
    if (!processes || !count) return false;
    
    *processes = NULL;
    *count = 0;
    return true;
}

static bool execute_command_impl(const char* command, command_result_t* result) {
    if (!command || !result) return false;
    
    // Initialize result
    memset(result, 0, sizeof(command_result_t));
    
    // Execute command using popen
    FILE* pipe = popen(command, "r");
    if (!pipe) {
        snprintf(result->error, sizeof(result->error), "Failed to execute command: %s", command);
        return false;
    }
    
    // Read output
    char buffer[1024];
    size_t total_size = 0;
    size_t buffer_size = 1024;
    char* output = malloc(buffer_size);
    
    if (!output) {
        pclose(pipe);
        snprintf(result->error, sizeof(result->error), "Memory allocation failed");
        return false;
    }
    
    output[0] = '\0';
    
    while (fgets(buffer, sizeof(buffer), pipe) != NULL) {
        size_t len = strlen(buffer);
        if (total_size + len + 1 > buffer_size) {
            buffer_size *= 2;
            char* new_output = realloc(output, buffer_size);
            if (!new_output) {
                free(output);
                pclose(pipe);
                snprintf(result->error, sizeof(result->error), "Memory reallocation failed");
                return false;
            }
            output = new_output;
        }
        strcat(output + total_size, buffer);
        total_size += len;
    }
    
    int exit_code = pclose(pipe);
    
    result->stdout = output;
    result->stderr = strdup(""); // No stderr capture in this simple implementation
    result->exit_code = exit_code;
    result->success = (exit_code == 0);
    
    return true;
}

static bool read_file_impl(const char* path, file_content_t* content) {
    if (!path || !content) return false;
    
    // Initialize content
    memset(content, 0, sizeof(file_content_t));
    
    // Open file
    FILE* file = fopen(path, "r");
    if (!file) {
        snprintf(content->error, sizeof(content->error), "Failed to open file: %s", path);
        return false;
    }
    
    // Get file size
    fseek(file, 0, SEEK_END);
    long file_size = ftell(file);
    fseek(file, 0, SEEK_SET);
    
    if (file_size < 0) {
        fclose(file);
        snprintf(content->error, sizeof(content->error), "Failed to get file size");
        return false;
    }
    
    // Allocate buffer
    char* buffer = malloc(file_size + 1);
    if (!buffer) {
        fclose(file);
        snprintf(content->error, sizeof(content->error), "Memory allocation failed");
        return false;
    }
    
    // Read file
    size_t bytes_read = fread(buffer, 1, file_size, file);
    fclose(file);
    
    buffer[bytes_read] = '\0';
    
    content->content = buffer;
    content->size = bytes_read;
    content->success = true;
    
    return true;
}

static bool get_system_info_impl(system_info_t* info) {
    if (!info) return false;
    
    // Initialize info
    memset(info, 0, sizeof(system_info_t));
    
    // Get system information
    struct utsname uts;
    if (uname(&uts) == 0) {
        snprintf(info->os_type, sizeof(info->os_type), "%s", uts.sysname);
        snprintf(info->os_version, sizeof(info->os_version), "%s %s", uts.release, uts.version);
        snprintf(info->hostname, sizeof(info->hostname), "%s", uts.nodename);
    } else {
        strncpy(info->os_type, "Unknown", sizeof(info->os_type) - 1);
        strncpy(info->os_version, "Unknown", sizeof(info->os_version) - 1);
        strncpy(info->hostname, "unknown", sizeof(info->hostname) - 1);
    }
    
    // Get system metrics
    struct sysinfo si;
    if (sysinfo(&si) == 0) {
        info->uptime = (uint64_t)si.uptime;
        info->cpu_cores = (uint32_t)si.procs; // This is not exactly CPU cores, but a fallback
        info->total_memory = si.totalram * si.mem_unit;
        info->available_memory = si.freeram * si.mem_unit;
    }
    
    return true;
}

static void free_memory_impl(void* ptr) {
    if (ptr) {
        free(ptr);
    }
}

// Plugin interface structure
static plugin_interface_t interface = {
    .get_plugin_info = get_plugin_info_impl,
    .init = plugin_init_impl,
    .cleanup = plugin_cleanup_impl,
    .get_system_metrics = get_system_metrics_impl,
    .get_processes = get_processes_impl,
    .execute_command = execute_command_impl,
    .read_file = read_file_impl,
    .get_system_info = get_system_info_impl,
    .free_memory = free_memory_impl
};

// Plugin entry point
PLUGIN_EXPORT plugin_interface_t* PLUGIN_CALL get_plugin_interface(void) {
    return &interface;
}
