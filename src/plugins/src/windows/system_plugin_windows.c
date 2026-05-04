/**
 * @file system_plugin_windows.c
 * @brief Windows-specific system plugin implementation in C
 * 
 * This plugin provides Windows-specific system metrics and operations
 * using Windows API calls.
 */

#include "../include/plugin_interface.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <windows.h>
#include <psapi.h>
#include <tchar.h>

// Plugin information
static const char* PLUGIN_NAME = "modern_system_plugin";
static const char* PLUGIN_VERSION = "1.0.0";
static const char* PLUGIN_DESCRIPTION = "Windows-specific system metrics plugin";

// Get CPU usage using Windows Performance Counters
static float get_cpu_usage(void) {
    static ULARGE_INTEGER last_cpu, last_sys_idle, last_sys_kernel, last_sys_user;
    ULARGE_INTEGER cpu, sys_idle, sys_kernel, sys_user;
    
    // Get system times
    FILETIME idle_time, kernel_time, user_time;
    if (!GetSystemTimes(&idle_time, &kernel_time, &user_time)) {
        return 0.0f;
    }
    
    // Convert to ULARGE_INTEGER
    memcpy(&sys_idle, &idle_time, sizeof(FILETIME));
    memcpy(&sys_kernel, &kernel_time, sizeof(FILETIME));
    memcpy(&sys_user, &user_time, sizeof(FILETIME));
    
    // Get process time (as proxy for CPU time)
    FILETIME create_time, exit_time, proc_kernel_time, proc_user_time;
    if (!GetProcessTimes(GetCurrentProcess(), &create_time, &exit_time, 
                        &proc_kernel_time, &proc_user_time)) {
        return 0.0f;
    }
    
    memcpy(&cpu, &proc_kernel_time, sizeof(FILETIME));
    
    // Calculate CPU usage
    ULARGE_INTEGER kernel_diff, user_diff;
    kernel_diff.QuadPart = sys_kernel.QuadPart - last_sys_kernel.QuadPart;
    user_diff.QuadPart = sys_user.QuadPart - last_sys_user.QuadPart;
    
    ULARGE_INTEGER total_diff = kernel_diff;
    total_diff.QuadPart += user_diff.QuadPart;
    
    if (total_diff.QuadPart > 0) {
        float cpu_usage = 100.0f - (float)(sys_idle.QuadPart - last_sys_idle.QuadPart) * 100.0f / (float)total_diff.QuadPart;
        
        // Update last values
        last_cpu = cpu;
        last_sys_idle = sys_idle;
        last_sys_kernel = sys_kernel;
        last_sys_user = sys_user;
        
        return cpu_usage > 0.0f && cpu_usage <= 100.0f ? cpu_usage : 0.0f;
    }
    
    return 0.0f;
}

// Get memory usage using Windows API
static float get_memory_usage(void) {
    MEMORYSTATUSEX mem_info;
    mem_info.dwLength = sizeof(MEMORYSTATUSEX);
    
    if (GlobalMemoryStatusEx(&mem_info)) {
        return (float)mem_info.dwMemoryLoad;
    }
    
    return 0.0f;
}

// Get disk usage for system drive
static float get_disk_usage(void) {
    // Get system drive (usually C:)
    char system_path[MAX_PATH];
    if (GetSystemDirectoryA(system_path, MAX_PATH) == 0) {
        return 0.0f;
    }
    
    // Extract drive letter
    system_path[3] = '\0'; // Keep only "C:\"
    
    ULARGE_INTEGER free_bytes, total_bytes;
    if (GetDiskFreeSpaceExA(system_path, &free_bytes, &total_bytes, NULL)) {
        if (total_bytes.QuadPart > 0) {
            return 100.0f - (float)free_bytes.QuadPart * 100.0f / (float)total_bytes.QuadPart;
        }
    }
    
    return 0.0f;
}

// Get system uptime
static uint64_t get_system_uptime(void) {
    return GetTickCount64() / 1000; // Convert milliseconds to seconds
}

// Get CPU core count
static uint32_t get_cpu_cores(void) {
    SYSTEM_INFO sys_info;
    GetSystemInfo(&sys_info);
    return sys_info.dwNumberOfProcessors;
}

// Get memory information
static void get_memory_info(uint64_t* total, uint64_t* available) {
    MEMORYSTATUSEX mem_info;
    mem_info.dwLength = sizeof(MEMORYSTATUSEX);
    
    if (GlobalMemoryStatusEx(&mem_info)) {
        *total = mem_info.ullTotalPhys;
        *available = mem_info.ullAvailPhys;
    } else {
        *total = 0;
        *available = 0;
    }
}

// Get OS version information (Simplified for compatibility)
static void get_os_info(char* os_type, char* os_version) {
    strcpy(os_type, "Windows");

    // GetVersionEx is deprecated and unreliable without manifest.
    // Using a more stable way to check for modern Windows.
    HMODULE hKernel = GetModuleHandleA("kernel32.dll");
    if (hKernel) {
        // Just a placeholder: in a real scenario, you'd check file version of kernel32.dll
        // or use RtlGetVersion from ntdll.dll
        OSVERSIONINFOEXW osvi = { sizeof(osvi), 0, 0, 0, 0, {0}, 0, 0 };
        typedef NTSTATUS (WINAPI *RtlGetVersionPtr)(PRTL_OSVERSIONINFOEXW);
        RtlGetVersionPtr fn = (RtlGetVersionPtr)GetProcAddress(GetModuleHandleA("ntdll.dll"), "RtlGetVersion");
        if (fn && fn(&osvi) == 0) {
            snprintf(os_version, 128, "%d.%d (Build %d)", 
                osvi.dwMajorVersion, osvi.dwMinorVersion, osvi.dwBuildNumber);
        } else {
            strcpy(os_version, "NT Family");
        }
    } else {
        strcpy(os_version, "Unknown");
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
    
    // Get hostname using Windows API directly
    DWORD size = sizeof(metrics->hostname);
    if (!GetComputerNameA(metrics->hostname, &size)) {
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

static bool get_processes_impl(process_info_t** out_processes, size_t* out_count) {
    if (!out_processes || !out_count) return false;
    
    *out_processes = NULL;
    *out_count = 0;
    
    // Get process list
    DWORD process_ids[1024], needed;
    if (!EnumProcesses(process_ids, sizeof(process_ids), &needed)) {
        return false;
    }
    
    size_t process_count = needed / sizeof(DWORD);
    if (process_count == 0) return true;
    
    // Allocate memory
    process_info_t* list = (process_info_t*)malloc(process_count * sizeof(process_info_t));
    if (!list) return false;
    
    size_t index = 0;
    
    for (size_t i = 0; i < process_count && index < process_count; i++) {
        if (process_ids[i] == 0) continue;
        
        HANDLE hProcess = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, FALSE, process_ids[i]);
        if (!hProcess) continue;
        
        process_info_t* proc = &list[index];
        memset(proc, 0, sizeof(process_info_t));
        
        proc->pid = process_ids[i];
        
        // Get process name
        HMODULE hMod;
        DWORD cbNeeded;
        if (EnumProcessModules(hProcess, &hMod, sizeof(hMod), &cbNeeded)) {
            char process_name[MAX_PATH];
            if (GetModuleBaseNameA(hProcess, hMod, process_name, sizeof(process_name))) {
                strncpy(proc->name, process_name, sizeof(proc->name) - 1);
                proc->name[sizeof(proc->name) - 1] = '\0';
            }
        }
        
        // Get process creation time
        FILETIME create_time, exit_time, kernel_time, user_time;
        if (GetProcessTimes(hProcess, &create_time, &exit_time, &kernel_time, &user_time)) {
            ULARGE_INTEGER uli;
            uli.LowPart = create_time.dwLowDateTime;
            uli.HighPart = create_time.dwHighDateTime;
            proc->start_time = uli.QuadPart / 10000000ULL - 11644473600ULL; // Convert to Unix timestamp
        }
        
        CloseHandle(hProcess);
        index++;
    }
    
    *out_processes = list;
    *out_count = index;
    return true;
}

static bool execute_command_impl(const char* command, command_result_t* result) {
    if (!command || !result) return false;
    
    memset(result, 0, sizeof(command_result_t));
    
    // Create pipes for stdout and stderr
    HANDLE hStdoutRead, hStdoutWrite;
    HANDLE hStderrRead, hStderrWrite;
    
    SECURITY_ATTRIBUTES sa;
    sa.nLength = sizeof(SECURITY_ATTRIBUTES);
    sa.bInheritHandle = TRUE;
    sa.lpSecurityDescriptor = NULL;
    
    if (!CreatePipe(&hStdoutRead, &hStdoutWrite, &sa, 0) ||
        !CreatePipe(&hStderrRead, &hStderrWrite, &sa, 0)) {
        snprintf(result->error, sizeof(result->error), "Failed to create pipes");
        return false;
    }
    
    // Set up STARTUPINFO
    STARTUPINFOA si;
    PROCESS_INFORMATION pi;
    
    memset(&si, 0, sizeof(si));
    si.cb = sizeof(si);
    si.hStdError = hStderrWrite;
    si.hStdOutput = hStdoutWrite;
    si.hStdInput = GetStdHandle(STD_INPUT_HANDLE);
    si.dwFlags |= STARTF_USESTDHANDLES;
    
    // Create the process
    char cmd_line[MAX_PATH * 2];
    snprintf(cmd_line, sizeof(cmd_line), "cmd.exe /c %s", command);
    
    if (!CreateProcessA(NULL, cmd_line, NULL, NULL, TRUE, 0, NULL, NULL, &si, &pi)) {
        CloseHandle(hStdoutRead);
        CloseHandle(hStdoutWrite);
        CloseHandle(hStderrRead);
        CloseHandle(hStderrWrite);
        snprintf(result->error, sizeof(result->error), "Failed to create process");
        return false;
    }
    
    // Close write ends
    CloseHandle(hStdoutWrite);
    CloseHandle(hStderrWrite);
    
    // Read stdout
    DWORD bytes_read;
    char buffer[1024];
    size_t total_size = 0;
    size_t buffer_size = 1024;
    char* output = malloc(buffer_size);
    
    if (!output) {
        CloseHandle(hStdoutRead);
        CloseHandle(hStderrRead);
        CloseHandle(pi.hProcess);
        CloseHandle(pi.hThread);
        snprintf(result->error, sizeof(result->error), "Memory allocation failed");
        return false;
    }
    
    output[0] = '\0';
    
    while (ReadFile(hStdoutRead, buffer, sizeof(buffer) - 1, &bytes_read, NULL) && bytes_read > 0) {
        buffer[bytes_read] = '\0';
        
        if (total_size + bytes_read + 1 > buffer_size) {
            buffer_size *= 2;
            char* new_output = realloc(output, buffer_size);
            if (!new_output) {
                free(output);
                CloseHandle(hStdoutRead);
                CloseHandle(hStderrRead);
                CloseHandle(pi.hProcess);
                CloseHandle(pi.hThread);
                snprintf(result->error, sizeof(result->error), "Memory reallocation failed");
                return false;
            }
            output = new_output;
        }
        
        strcat(output + total_size, buffer);
        total_size += bytes_read;
    }
    
    // Read stderr
    char* stderr_output = malloc(1024);
    if (stderr_output) {
        stderr_output[0] = '\0';
        size_t stderr_size = 0;
        
        while (ReadFile(hStderrRead, buffer, sizeof(buffer) - 1, &bytes_read, NULL) && bytes_read > 0) {
            buffer[bytes_read] = '\0';
            
            if (stderr_size + bytes_read + 1 > 1024) {
                break; // Limit stderr output
            }
            
            strcat(stderr_output + stderr_size, buffer);
            stderr_size += bytes_read;
        }
    } else {
        stderr_output = strdup("");
    }
    
    CloseHandle(hStdoutRead);
    CloseHandle(hStderrRead);
    
    // Wait for process to finish
    WaitForSingleObject(pi.hProcess, INFINITE);
    
    DWORD exit_code;
    GetExitCodeProcess(pi.hProcess, &exit_code);
    
    CloseHandle(pi.hProcess);
    CloseHandle(pi.hThread);
    
    result->stdout = output;
    result->stderr = stderr_output;
    result->exit_code = (int)exit_code;
    result->success = (exit_code == 0);
    
    return true;
}

static bool read_file_impl(const char* path, file_content_t* content) {
    if (!path || !content) return false;
    
    memset(content, 0, sizeof(file_content_t));
    
    HANDLE hFile = CreateFileA(path, GENERIC_READ, FILE_SHARE_READ, NULL, 
                             OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    
    if (hFile == INVALID_HANDLE_VALUE) {
        snprintf(content->error, sizeof(content->error), "Failed to open file: %s", path);
        return false;
    }
    
    DWORD file_size = GetFileSize(hFile, NULL);
    if (file_size == INVALID_FILE_SIZE) {
        CloseHandle(hFile);
        snprintf(content->error, sizeof(content->error), "Failed to get file size");
        return false;
    }
    
    char* buffer = malloc(file_size + 1);
    if (!buffer) {
        CloseHandle(hFile);
        snprintf(content->error, sizeof(content->error), "Memory allocation failed");
        return false;
    }
    
    DWORD bytes_read;
    if (!ReadFile(hFile, buffer, file_size, &bytes_read, NULL) || bytes_read != file_size) {
        free(buffer);
        CloseHandle(hFile);
        snprintf(content->error, sizeof(content->error), "Failed to read file");
        return false;
    }
    
    CloseHandle(hFile);
    
    buffer[bytes_read] = '\0';
    
    content->content = buffer;
    content->size = bytes_read;
    content->success = true;
    
    return true;
}

static bool get_system_info_impl(system_info_t* info) {
    if (!info) return false;
    
    memset(info, 0, sizeof(system_info_t));
    
    // Get OS information
    get_os_info(info->os_type, info->os_version);
    
    // Get hostname
    DWORD size = sizeof(info->hostname);
    if (!GetComputerNameA(info->hostname, &size)) {
        strncpy(info->hostname, "unknown", sizeof(info->hostname) - 1);
    }
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
