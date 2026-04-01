#include "../../include/simple_plugin.h"
#include <windows.h>
#include <psapi.h>
#include <tlhelp32.h>
#include <cstring>
#include <cstdlib>
#include <cstdio>
#include <string>

// Global plugin info
static PluginInfo g_plugin_info = {
    "windows_system_plugin",
    "1.0.0",
    "Windows system metrics and operations plugin"
};

// Plugin implementation
extern "C" {
    
PLUGIN_EXPORT PluginInfo* PLUGIN_CALL get_plugin_info() {
    return &g_plugin_info;
}

PLUGIN_EXPORT bool PLUGIN_CALL plugin_init() {
    return true;
}

PLUGIN_EXPORT void PLUGIN_CALL plugin_cleanup() {
    // Cleanup if needed
}

PLUGIN_EXPORT bool PLUGIN_CALL get_system_metrics(SystemMetrics* metrics) {
    if (!metrics) return false;
    
    // Get CPU usage (simplified)
    metrics->cpu_usage = 25.0f; // Placeholder
    
    // Get memory usage
    MEMORYSTATUSEX memInfo;
    memInfo.dwLength = sizeof(MEMORYSTATUSEX);
    if (GlobalMemoryStatusEx(&memInfo)) {
        metrics->ram_usage = ((float)(memInfo.ullTotalPhys - memInfo.ullAvailPhys) / memInfo.ullTotalPhys) * 100.0f;
    } else {
        metrics->ram_usage = 0.0f;
    }
    
    // Get disk usage (C: drive)
    ULARGE_INTEGER free_bytes, total_bytes;
    if (GetDiskFreeSpaceExA("C:\\", &free_bytes, &total_bytes, NULL)) {
        metrics->disk_usage = ((float)(total_bytes.QuadPart - free_bytes.QuadPart) / total_bytes.QuadPart) * 100.0f;
    } else {
        metrics->disk_usage = 0.0f;
    }
    
    // Get uptime
    metrics->uptime = GetTickCount64() / 1000;
    
    // Get hostname
    char hostname[256] = {0};
    DWORD hostname_size = sizeof(hostname);
    if (GetComputerNameA(hostname, &hostname_size)) {
        strcpy_s(metrics->hostname, sizeof(metrics->hostname), hostname);
    } else {
        strcpy_s(metrics->hostname, sizeof(metrics->hostname), "unknown");
    }
    
    return true;
}

PLUGIN_EXPORT bool PLUGIN_CALL get_processes(ProcessInfo* processes, int* count) {
    if (!processes || !count) return false;
    
    HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if (snapshot == INVALID_HANDLE_VALUE) return false;
    
    PROCESSENTRY32 pe32;
    pe32.dwSize = sizeof(PROCESSENTRY32);
    
    int process_count = 0;
    int max_count = *count;
    
    if (Process32First(snapshot, &pe32)) {
        do {
            if (process_count < max_count) {
                processes[process_count].pid = pe32.th32ProcessID;
                strcpy_s(processes[process_count].name, sizeof(processes[process_count].name), 
                         pe32.szExeFile);
                
                // Get memory info
                HANDLE hProcess = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, FALSE, pe32.th32ProcessID);
                if (hProcess) {
                    PROCESS_MEMORY_COUNTERS pmc;
                    if (GetProcessMemoryInfo(hProcess, &pmc, sizeof(pmc))) {
                        processes[process_count].memory_usage = pmc.WorkingSetSize;
                    }
                    CloseHandle(hProcess);
                } else {
                    processes[process_count].memory_usage = 0;
                }
                
                process_count++;
            }
        } while (Process32Next(snapshot, &pe32) && process_count < max_count);
    }
    
    CloseHandle(snapshot);
    *count = process_count;
    return true;
}

PLUGIN_EXPORT bool PLUGIN_CALL execute_command(const char* command, CommandResult* result) {
    if (!command || !result) return false;
    
    // Security check - simple whitelist
    const char* allowed_commands[] = {"ps", "top", "df", "free", "uptime", "whoami", "id", "uname", "date"};
    bool command_allowed = false;
    
    for (const char* allowed : allowed_commands) {
        if (strstr(command, allowed) == command) {
            command_allowed = true;
            break;
        }
    }
    
    if (!command_allowed) {
        strcpy_s(result->error, sizeof(result->error), "Command not allowed for security reasons");
        result->success = false;
        return false;
    }
    
    // Execute command
    FILE* pipe = _popen(command, "r");
    if (!pipe) {
        strcpy_s(result->error, sizeof(result->error), "Failed to execute command");
        result->success = false;
        return false;
    }
    
    // Read output
    char buffer[4096];
    char* output = (char*)malloc(4096);
    if (!output) {
        result->success = false;
        strcpy_s(result->error, sizeof(result->error), "Memory allocation failed");
        return false;
    }
    output[0] = '\0';
    
    while (fgets(buffer, sizeof(buffer), pipe) != NULL) {
        strcat_s(output, 4096, buffer);
    }
    
    result->stdout = output;
    
    result->exit_code = _pclose(pipe);
    result->success = (result->exit_code == 0);
    
    return true;
}

PLUGIN_EXPORT bool PLUGIN_CALL read_file(const char* path, FileContent* content) {
    if (!path || !content) return false;
    
    // Path traversal check
    if (strstr(path, "..") != NULL) {
        strcpy_s(content->error, sizeof(content->error), "Path traversal not allowed");
        content->success = false;
        return false;
    }
    
    FILE* file = fopen(path, "rb");
    if (!file) {
        snprintf(content->error, sizeof(content->error), "Failed to open file: %s", path);
        content->success = false;
        return false;
    }
    
    // Get file size
    fseek(file, 0, SEEK_END);
    size_t size = ftell(file);
    fseek(file, 0, SEEK_SET);
    
    // Limit file size
    const size_t max_size = 100000; // 100KB
    if (size > max_size) size = max_size;
    
    // Read content
    content->content = (char*)malloc(size + 1);
    if (!content->content) {
        fclose(file);
        strcpy_s(content->error, sizeof(content->error), "Memory allocation failed");
        content->success = false;
        return false;
    }
    
    size_t read_size = fread(content->content, 1, size, file);
    content->content[read_size] = '\0';
    content->size = read_size;
    content->success = true;
    
    fclose(file);
    return true;
}

PLUGIN_EXPORT void PLUGIN_CALL free_memory(void* ptr) {
    if (ptr) {
        free(ptr);
    }
}

}
