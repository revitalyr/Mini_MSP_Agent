/**
 * @file system_plugin_macos.c
 * @brief macOS-specific system plugin implementation in C
 * 
 * This plugin provides macOS-specific system metrics and operations
 * using Darwin system calls and APIs.
 */

#include "../include/plugin_interface.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/sysctl.h>
#include <sys/utsname.h>
#include <sys/statvfs.h>
#include <sys/mount.h>
#include <mach/mach.h>
#include <mach/host_statistics.h>
#include <mach/vm_statistics.h>
#include <CoreFoundation/CoreFoundation.h>
#include <IOKit/ps/IOPowerSources.h>
#include <IOKit/ps/IOPSKeys.h>

// Plugin information
static const char* PLUGIN_NAME = "macos_system_plugin";
static const char* PLUGIN_VERSION = "1.0.0";
static const char* PLUGIN_DESCRIPTION = "macOS-specific system metrics plugin";

// Get CPU usage using host_statistics
static float get_cpu_usage(void) {
    host_cpu_load_info_data_t cpuinfo;
    mach_msg_type_number_t count = HOST_CPU_LOAD_INFO_COUNT;
    
    if (host_statistics(mach_host_self(), HOST_CPU_LOAD_INFO, 
                        (host_info_t)&cpuinfo, &count) != KERN_SUCCESS) {
        return 0.0f;
    }
    
    // Calculate CPU usage
    unsigned long total_ticks = 0;
    for (int i = 0; i < CPU_STATE_MAX; i++) {
        total_ticks += cpuinfo.cpu_ticks[i];
    }
    
    if (total_ticks > 0) {
        unsigned long idle_ticks = cpuinfo.cpu_ticks[CPU_STATE_IDLE];
        return 100.0f - (float)idle_ticks * 100.0f / (float)total_ticks;
    }
    
    return 0.0f;
}

// Get memory usage using vm_statistics
static float get_memory_usage(void) {
    vm_statistics64_data_t vm_info;
    mach_msg_type_number_t count = HOST_VM_INFO64_COUNT;
    
    if (host_statistics64(mach_host_self(), HOST_VM_INFO64, 
                          (host_info64_t)&vm_info, &count) != KERN_SUCCESS) {
        return 0.0f;
    }
    
    uint64_t total_memory = 0;
    size_t size = sizeof(total_memory);
    
    if (sysctlbyname("hw.memsize", &total_memory, &size, NULL, 0) != 0) {
        return 0.0f;
    }
    
    if (total_memory > 0) {
        uint64_t free_memory = vm_info.free_count * vm_page_size;
        uint64_t used_memory = total_memory - free_memory;
        return (float)used_memory * 100.0f / (float)total_memory;
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

// Get system uptime
static uint64_t get_system_uptime(void) {
    struct timeval boottime;
    size_t size = sizeof(boottime);
    
    if (sysctlbyname("kern.boottime", &boottime, &size, NULL, 0) == 0) {
        time_t now = time(NULL);
        return (uint64_t)(now - boottime.tv_sec);
    }
    
    // Fallback to mach_absolute_time
    mach_timebase_info_data_t timebase;
    mach_timebase_info(&timebase);
    
    uint64_t uptime = mach_absolute_time() / 1000000000ULL; // Convert to seconds
    return uptime;
}

// Get CPU core count
static uint32_t get_cpu_cores(void) {
    int cores = 0;
    size_t size = sizeof(cores);
    
    if (sysctlbyname("hw.ncpu", &cores, &size, NULL, 0) != 0) {
        return 1;
    }
    
    return cores > 0 ? (uint32_t)cores : 1;
}

// Get total and available memory
static void get_memory_info(uint64_t* total, uint64_t* available) {
    size_t size = sizeof(*total);
    
    if (sysctlbyname("hw.memsize", total, &size, NULL, 0) != 0) {
        *total = 0;
        *available = 0;
        return;
    }
    
    // Get free memory using vm_statistics
    vm_statistics64_data_t vm_info;
    mach_msg_type_number_t count = HOST_VM_INFO64_COUNT;
    
    if (host_statistics64(mach_host_self(), HOST_VM_INFO64, 
                          (host_info64_t)&vm_info, &count) == KERN_SUCCESS) {
        *available = vm_info.free_count * vm_page_size;
    } else {
        *available = 0;
    }
}

// Get macOS version information
static void get_macos_info(char* os_version) {
    // Use Gestalt to get macOS version
    SInt32 major_version, minor_version, bug_fix_version;
    
    if (Gestalt(gestaltSystemVersionMajor, &major_version) == noErr &&
        Gestalt(gestaltSystemVersionMinor, &minor_version) == noErr &&
        Gestalt(gestaltSystemVersionBugFix, &bug_fix_version) == noErr) {
        
        snprintf(os_version, 128, "%d.%d.%d", major_version, minor_version, bug_fix_version);
    } else {
        // Fallback to sysctl
        char version_str[256];
        size_t size = sizeof(version_str);
        
        if (sysctlbyname("kern.version", version_str, &size, NULL, 0) == 0) {
            strncpy(os_version, version_str, 127);
            os_version[127] = '\0';
        } else {
            strcpy(os_version, "Unknown");
        }
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
    
    // Get process count
    int mib[4] = {CTL_KERN, KERN_PROC, KERN_PROC_ALL, 0};
    size_t size = 0;
    
    if (sysctl(mib, 4, NULL, &size, NULL, 0) != 0) {
        return false;
    }
    
    size_t process_count = size / sizeof(struct kinfo_proc);
    if (process_count == 0) return true;
    
    // Allocate memory for processes
    *processes = malloc(process_count * sizeof(process_info_t));
    if (!*processes) return false;
    
    // Get process list
    struct kinfo_proc* proc_list = malloc(size);
    if (!proc_list) {
        free(*processes);
        *processes = NULL;
        return false;
    }
    
    if (sysctl(mib, 4, proc_list, &size, NULL, 0) != 0) {
        free(proc_list);
        free(*processes);
        *processes = NULL;
        return false;
    }
    
    // Fill process information
    size_t index = 0;
    process_count = size / sizeof(struct kinfo_proc);
    
    for (size_t i = 0; i < process_count && index < process_count; i++) {
        process_info_t* proc = &(*processes)[index];
        memset(proc, 0, sizeof(process_info_t));
        
        proc->pid = proc_list[i].kp_proc.p_pid;
        
        // Get process name
        if (proc_list[i].kp_proc.p_comm[0] != '\0') {
            strncpy(proc->name, proc_list[i].kp_proc.p_comm, sizeof(proc->name) - 1);
            proc->name[sizeof(proc->name) - 1] = '\0';
        } else {
            snprintf(proc->name, sizeof(proc->name), "process_%d", proc->pid);
        }
        
        // Get start time
        proc->start_time = proc_list[i].kp_proc.p_starttime.tv_sec;
        
        index++;
    }
    
    free(proc_list);
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
        strncpy(info->os_type, uts.sysname, sizeof(info->os_type) - 1);
        get_macos_info(info->os_version);
        strncpy(info->hostname, uts.nodename, sizeof(info->hostname) - 1);
    } else {
        strncpy(info->os_type, "Darwin", sizeof(info->os_type) - 1);
        strcpy(info->os_version, "Unknown");
        strncpy(info->hostname, "unknown", sizeof(info->hostname) - 1);
    }
    
    info->os_type[sizeof(info->os_type) - 1] = '\0';
    info->hostname[sizeof(info->hostname) - 1] = '\0';
    
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
