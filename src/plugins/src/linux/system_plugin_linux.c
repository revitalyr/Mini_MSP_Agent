/**
 * @file system_plugin_linux.c
 * @brief Linux-specific system plugin implementation in C
 * 
 * This plugin provides Linux-specific system metrics and operations
 * using direct system calls and /proc filesystem access.
 */

#include "../include/plugin_interface.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/sysinfo.h>
#include <sys/utsname.h>
#include <sys/statvfs.h>
#include <dirent.h>
#include <fcntl.h>

// Plugin information
static const char* PLUGIN_NAME = "linux_system_plugin";
static const char* PLUGIN_VERSION = "1.0.0";
static const char* PLUGIN_DESCRIPTION = "Linux-specific system metrics plugin";

// Get accurate CPU usage from /proc/stat
static float get_cpu_usage(void) {
    FILE* fp = fopen("/proc/stat", "r");
    if (!fp) return 0.0f;
    
    char line[256];
    if (fgets(line, sizeof(line), fp)) {
        unsigned long user, nice, system, idle, iowait, irq, softirq;
        if (sscanf(line, "cpu %lu %lu %lu %lu %lu %lu %lu", 
                   &user, &nice, &system, &idle, &iowait, &irq, &softirq) == 7) {
            fclose(fp);
            
            unsigned long total = user + nice + system + idle + iowait + irq + softirq;
            unsigned long work = user + nice + system + irq + softirq;
            
            if (total > 0) {
                return (float)work * 100.0f / (float)total;
            }
        }
    }
    fclose(fp);
    return 0.0f;
}

// Get memory information from /proc/meminfo
static float get_memory_usage(void) {
    FILE* fp = fopen("/proc/meminfo", "r");
    if (!fp) return 0.0f;
    
    char line[256];
    unsigned long total_mem = 0, free_mem = 0, buffers = 0, cached = 0;
    
    while (fgets(line, sizeof(line), fp)) {
        if (sscanf(line, "MemTotal: %lu kB", &total_mem) == 1) continue;
        if (sscanf(line, "MemFree: %lu kB", &free_mem) == 1) continue;
        if (sscanf(line, "Buffers: %lu kB", &buffers) == 1) continue;
        if (sscanf(line, "Cached: %lu kB", &cached) == 1) continue;
        
        if (total_mem && free_mem) break;
    }
    fclose(fp);
    
    if (total_mem > 0) {
        unsigned long used = total_mem - free_mem - buffers - cached;
        return (float)used * 100.0f / (float)total_mem;
    }
    
    return 0.0f;
}

// Get disk usage for root filesystem
static float get_disk_usage(void) {
    struct statvfs vfs;
    if (statvfs("/", &vfs) == 0) {
        if (vfs.f_blocks > 0) {
            return (float)(vfs.f_blocks - vfs.f_bfree) * 100.0f / (float)vfs.f_blocks;
        }
    }
    return 0.0f;
}

// Get system uptime with high precision
static uint64_t get_system_uptime(void) {
    struct sysinfo si;
    if (sysinfo(&si) == 0) {
        return (uint64_t)si.uptime;
    }
    
    // Fallback to /proc/uptime
    FILE* fp = fopen("/proc/uptime", "r");
    if (fp) {
        double uptime;
        if (fscanf(fp, "%lf", &uptime) == 1) {
            fclose(fp);
            return (uint64_t)uptime;
        }
        fclose(fp);
    }
    
    return 0;
}

// Get CPU core count
static uint32_t get_cpu_cores(void) {
    FILE* fp = fopen("/proc/cpuinfo", "r");
    if (!fp) return 1;
    
    char line[256];
    uint32_t cores = 0;
    
    while (fgets(line, sizeof(line), fp)) {
        if (strncmp(line, "processor", 9) == 0) {
            cores++;
        }
    }
    fclose(fp);
    
    return cores > 0 ? cores : 1;
}

// Get total and available memory
static void get_memory_info(uint64_t* total, uint64_t* available) {
    struct sysinfo si;
    if (sysinfo(&si) == 0) {
        *total = si.totalram * si.mem_unit;
        *available = si.freeram * si.mem_unit;
        return;
    }
    
    // Fallback to /proc/meminfo
    FILE* fp = fopen("/proc/meminfo", "r");
    if (fp) {
        char line[256];
        unsigned long total_kb = 0, free_kb = 0;
        
        while (fgets(line, sizeof(line), fp)) {
            if (sscanf(line, "MemTotal: %lu kB", &total_kb) == 1) continue;
            if (sscanf(line, "MemAvailable: %lu kB", &free_kb) == 1) continue;
            if (sscanf(line, "MemFree: %lu kB", &free_kb) == 1 && total_kb) break;
        }
        fclose(fp);
        
        *total = total_kb * 1024;
        *available = free_kb * 1024;
    }
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
    return true;
}

static void plugin_cleanup_impl(void) {
    // Nothing to cleanup
}

static bool get_system_metrics_impl(system_metrics_t* metrics) {
    if (!metrics) return false;
    
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
    if (!processes || !count) return false;
    
    *processes = NULL;
    *count = 0;
    
    // Count processes in /proc
    DIR* proc_dir = opendir("/proc");
    if (!proc_dir) return false;
    
    struct dirent* entry;
    size_t process_count = 0;
    
    while ((entry = readdir(proc_dir)) != NULL) {
        if (strspn(entry->d_name, "0123456789") == strlen(entry->d_name)) {
            process_count++;
        }
    }
    
    if (process_count == 0) {
        closedir(proc_dir);
        return true;
    }
    
    // Allocate memory for processes
    *processes = malloc(process_count * sizeof(process_info_t));
    if (!*processes) {
        closedir(proc_dir);
        return false;
    }
    
    // Fill process information (simplified)
    rewinddir(proc_dir);
    size_t index = 0;
    
    while ((entry = readdir(proc_dir)) != NULL && index < process_count) {
        if (strspn(entry->d_name, "0123456789") == strlen(entry->d_name)) {
            process_info_t* proc = &(*processes)[index];
            memset(proc, 0, sizeof(process_info_t));
            
            proc->pid = (uint32_t)atoi(entry->d_name);
            snprintf(proc->name, sizeof(proc->name), "process_%u", proc->pid);
            
            // Read start time from /proc/[pid]/stat
            char stat_path[64];
            snprintf(stat_path, sizeof(stat_path), "/proc/%s/stat", entry->d_name);
            
            FILE* stat_file = fopen(stat_path, "r");
            if (stat_file) {
                char line[1024];
                if (fgets(line, sizeof(line), stat_file)) {
                    // Parse fields from /proc/[pid]/stat
                    char* token = strtok(line, " ");
                    for (int i = 1; token && i < 22; i++) {
                        token = strtok(NULL, " ");
                        if (i == 21 && token) { // Start time field
                            proc->start_time = strtoull(token, NULL, 10);
                            break;
                        }
                    }
                }
                fclose(stat_file);
            }
            
            index++;
        }
    }
    
    closedir(proc_dir);
    *count = index;
    return true;
}

static bool execute_command_impl(const char* command, command_result_t* result) {
    if (!command || !result) return false;
    
    memset(result, 0, sizeof(command_result_t));
    
    FILE* pipe = popen(command, "r");
    if (!pipe) {
        snprintf(result->error, sizeof(result->error), "Failed to execute command: %s", command);
        return false;
    }
    
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
    result->stderr = strdup("");
    result->exit_code = exit_code;
    result->success = (exit_code == 0);
    
    return true;
}

static bool read_file_impl(const char* path, file_content_t* content) {
    if (!path || !content) return false;
    
    memset(content, 0, sizeof(file_content_t));
    
    FILE* file = fopen(path, "r");
    if (!file) {
        snprintf(content->error, sizeof(content->error), "Failed to open file: %s", path);
        return false;
    }
    
    fseek(file, 0, SEEK_END);
    long file_size = ftell(file);
    fseek(file, 0, SEEK_SET);
    
    if (file_size < 0) {
        fclose(file);
        snprintf(content->error, sizeof(content->error), "Failed to get file size");
        return false;
    }
    
    char* buffer = malloc(file_size + 1);
    if (!buffer) {
        fclose(file);
        snprintf(content->error, sizeof(content->error), "Memory allocation failed");
        return false;
    }
    
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
    
    memset(info, 0, sizeof(system_info_t));
    
    // Get system information
    struct utsname uts;
    if (uname(&uts) == 0) {
        snprintf(info->os_type, sizeof(info->os_type), "%s", uts.sysname);
        snprintf(info->os_version, sizeof(info->os_version), "%s %s", uts.release, uts.version);
        snprintf(info->hostname, sizeof(info->hostname), "%s", uts.nodename);
    } else {
        strncpy(info->os_type, "Linux", sizeof(info->os_type) - 1);
        strncpy(info->os_version, "Unknown", sizeof(info->os_version) - 1);
        strncpy(info->hostname, "unknown", sizeof(info->hostname) - 1);
    }
    
    // Get system metrics
    info->uptime = get_system_uptime();
    info->cpu_cores = get_cpu_cores();
    get_memory_info(&info->total_memory, &info->available_memory);
    
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
    .get_directory_info_data = NULL,
    .get_event_data = NULL,
    .get_watchers_data = NULL,
    .get_file_reader_data = NULL,
    .get_sensor_data = NULL,
    .get_camera_data = NULL,
    .get_processing_results = NULL,
    .get_video_frame = NULL,
    .get_forensic_data = NULL,
    .free_memory = free_memory_impl,
    .execute_json = NULL
};

// Plugin entry point
PLUGIN_EXPORT plugin_interface_t* PLUGIN_CALL get_plugin_interface(void) {
    return &interface;
}
