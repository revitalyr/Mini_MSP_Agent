#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <time.h>
#include <sys/utsname.h>
#include <sys/sysinfo.h>
#include "../../include/plugin_interface.h"

// Simple system plugin implementation
static int g_initialized = 0;
static plugin_info_t g_plugin_info = {
    .name = "Simple System Plugin",
    .version = "1.0.0",
    .description = "Basic system information plugin for Unix/Linux"
};

// Plugin initialization
bool plugin_init(void) {
    g_initialized = 1;
    return true;
}

// Plugin cleanup
void plugin_cleanup(void) {
    g_initialized = 0;
}

// Get plugin info
plugin_info_t* get_plugin_info(void) {
    return &g_plugin_info;
}

// Get system metrics
bool get_system_metrics(system_metrics_t* metrics) {
    if (!g_initialized || !metrics) return false;
    
    struct sysinfo si;
    if (sysinfo(&si) != 0) return false;
    
    // Get hostname
    char hostname[256];
    if (gethostname(hostname, sizeof(hostname)) != 0) {
        strcpy(hostname, "unknown");
    }
    
    // Calculate CPU usage (simplified)
    float cpu_usage = 0.0f;
    
    // Calculate memory usage
    uint64_t total_memory = si.totalram;
    uint64_t available_memory = si.freeram;
    float ram_usage = (float)(total_memory - available_memory) / total_memory * 100.0f;
    
    // Calculate disk usage (simplified)
    float disk_usage = 50.0f; // Placeholder
    
    metrics->cpu_usage = cpu_usage;
    metrics->ram_usage = ram_usage;
    metrics->disk_usage = disk_usage;
    metrics->uptime = si.uptime;
    strncpy(metrics->hostname, hostname, sizeof(metrics->hostname) - 1);
    
    return true;
}

// Get process list
bool get_processes(process_info_t** processes, size_t* count) {
    if (!g_initialized || !processes || !count) return false;
    
    // Simple implementation - allocate memory for a few fake processes
    *count = 3;
    *processes = malloc(sizeof(process_info_t) * (*count));
    if (!*processes) return false;
    
    strcpy((*processes)[0].name, "init");
    (*processes)[0].pid = 1;
    (*processes)[0].cpu_usage = 0.1f;
    (*processes)[0].memory_usage = 1024 * 1024; // 1MB
    (*processes)[0].start_time = 0;
    
    strcpy((*processes)[1].name, "kthreadd");
    (*processes)[1].pid = 2;
    (*processes)[1].cpu_usage = 0.0f;
    (*processes)[1].memory_usage = 512 * 1024; // 512KB
    (*processes)[1].start_time = 0;
    
    strcpy((*processes)[2].name, "systemd");
    (*processes)[2].pid = 100;
    (*processes)[2].cpu_usage = 0.5f;
    (*processes)[2].memory_usage = 2048 * 1024; // 2MB
    (*processes)[2].start_time = time(NULL) - 3600;
    
    return true;
}

// Execute command
bool execute_command(const char* command, command_result_t* result) {
    if (!g_initialized || !command || !result) return false;
    
    // Initialize result
    memset(result, 0, sizeof(command_result_t));
    
    // Execute command
    FILE* pipe = popen(command, "r");
    if (!pipe) {
        strcpy(result->error, "Failed to execute command");
        return false;
    }
    
    // Read stdout
    char buffer[1024];
    size_t total_size = 0;
    char* stdout_buffer = malloc(1024);
    stdout_buffer[0] = '\0';
    
    while (fgets(buffer, sizeof(buffer), pipe) != NULL) {
        size_t len = strlen(buffer);
        total_size += len;
        
        char* new_buffer = realloc(stdout_buffer, total_size + 1);
        if (!new_buffer) {
            free(stdout_buffer);
            fclose(pipe);
            strcpy(result->error, "Memory allocation failed");
            return false;
        }
        
        stdout_buffer = new_buffer;
        strcat(stdout_buffer, buffer);
    }
    
    int exit_code = pclose(pipe);
    
    result->stdout = stdout_buffer;
    result->stderr = strdup(""); // No stderr for simplicity
    result->exit_code = exit_code;
    result->success = (exit_code == 0);
    
    return true;
}

// Read file
bool read_file(const char* path, file_content_t* content) {
    if (!g_initialized || !path || !content) return false;
    
    FILE* file = fopen(path, "r");
    if (!file) {
        snprintf(content->error, sizeof(content->error), "Cannot open file: %s", path);
        return false;
    }
    
    // Get file size
    fseek(file, 0, SEEK_END);
    long file_size = ftell(file);
    fseek(file, 0, SEEK_SET);
    
    if (file_size < 0) {
        fclose(file);
        strcpy(content->error, "Cannot determine file size");
        return false;
    }
    
    // Allocate memory
    content->content = malloc(file_size + 1);
    if (!content->content) {
        fclose(file);
        strcpy(content->error, "Memory allocation failed");
        return false;
    }
    
    // Read file
    size_t bytes_read = fread(content->content, 1, file_size, file);
    content->content[bytes_read] = '\0';
    content->size = bytes_read;
    content->success = true;
    
    fclose(file);
    return true;
}

// Get system info
bool get_system_info(system_info_t* info) {
    if (!g_initialized || !info) return false;
    
    struct utsname uts;
    if (uname(&uts) != 0) return false;
    
    struct sysinfo si;
    if (sysinfo(&si) != 0) return false;
    
    strncpy(info->os_type, "Linux", sizeof(info->os_type) - 1);
    strncpy(info->os_version, uts.release, sizeof(info->os_version) - 1);
    strncpy(info->hostname, uts.nodename, sizeof(info->hostname) - 1);
    info->uptime = si.uptime;
    info->cpu_cores = si.procs; // Number of processors
    info->total_memory = si.totalram;
    info->available_memory = si.freeram;
    
    return true;
}

// Free memory
void free_memory(void* ptr) {
    if (ptr) {
        free(ptr);
    }
}

// Plugin interface
static plugin_interface_t g_plugin_interface = {
    .get_plugin_info = get_plugin_info,
    .init = plugin_init,
    .cleanup = plugin_cleanup,
    .get_system_metrics = get_system_metrics,
    .get_processes = get_processes,
    .execute_command = execute_command,
    .read_file = read_file,
    .get_system_info = get_system_info,
    .free_memory = free_memory
};

// Plugin entry point
PLUGIN_EXPORT plugin_interface_t* PLUGIN_CALL get_plugin_interface(void) {
    return &g_plugin_interface;
}
