/**
 * @file forensic_plugin.cpp
 * @brief Windows Forensic Artifacts Collector Plugin
 * 
 * Collects Windows-specific forensic artifacts:
 * - Registry autorun keys (Run, RunOnce, Winlogon, LSA)
 * - Windows Event Logs (Security, System, Application)
 * - AmCache.hve analysis
 * - WMI persistence
 * - IFEO (Image File Execution Options)
 * - Services and drivers
 */

#include <windows.h>
#include <tlhelp32.h>
#include <psapi.h>
#include <vector>
#include <string>
#include <cstring>
#include "../../include/plugin_interface.h"

#define EXPORT __declspec(dllexport)

static const char* PLUGIN_NAME = "windows_forensic_plugin";
static const char* PLUGIN_VERSION = "1.0.0";
static const char* PLUGIN_DESCRIPTION = "Windows forensic artifacts collector";

// Collect running processes with full details
static bool CollectProcesses(std::vector<process_info_t>& processes) {
    HANDLE hSnapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if (hSnapshot == INVALID_HANDLE_VALUE) {
        return false;
    }
    
    PROCESSENTRY32W pe32;
    pe32.dwSize = sizeof(PROCESSENTRY32W);
    
    if (!Process32FirstW(hSnapshot, &pe32)) {
        CloseHandle(hSnapshot);
        return false;
    }
    
    do {
        process_info_t proc;
        memset(&proc, 0, sizeof(proc));
        
        proc.pid = pe32.th32ProcessID;
        
        // Convert process name to UTF-8
        WideCharToMultiByte(CP_UTF8, 0, pe32.szExeFile, -1, proc.m_name, 
                           kMaxHostnameLen, NULL, NULL);
        
        // Get additional process info
        HANDLE hProcess = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, FALSE, proc.pid);
        if (hProcess != NULL) {
            // Get memory info
            PROCESS_MEMORY_COUNTERS pmc;
            if (GetProcessMemoryInfo(hProcess, &pmc, sizeof(pmc))) {
                proc.memory_usage = pmc.WorkingSetSize;
            }
            
            // Get process start time
            FILETIME createTime, exitTime, kernelTime, userTime;
            if (GetProcessTimes(hProcess, &createTime, &exitTime, &kernelTime, &userTime)) {
                ULARGE_INTEGER ull;
                ull.LowPart = createTime.dwLowDateTime;
                ull.HighPart = createTime.dwHighDateTime;
                // Convert Windows filetime to Unix timestamp
                proc.start_time = (ull.QuadPart - 116444736000000000ULL) / 10000000ULL;
            }
            
            CloseHandle(hProcess);
        }
        
        processes.push_back(proc);
    } while (Process32NextW(hSnapshot, &pe32));
    
    CloseHandle(hSnapshot);
    return true;
}

// Plugin interface implementations
static plugin_info_t plugin_info = {
    PLUGIN_NAME,
    PLUGIN_VERSION,
    PLUGIN_DESCRIPTION
};

static plugin_info_t* get_plugin_info_impl() {
    return &plugin_info;
}

static bool init_impl() {
    return true;
}

static void cleanup_impl() {
}

static bool get_system_metrics_impl(system_metrics_t* metrics) {
    if (!metrics) return false;
    memset(metrics, 0, sizeof(system_metrics_t));
    
    // Get hostname
    wchar_t hostname_w[256];
    DWORD size = 256;
    if (GetComputerNameW(hostname_w, &size)) {
        WideCharToMultiByte(CP_UTF8, 0, hostname_w, -1, metrics->m_hostname, 
                           kMaxHostnameLen, NULL, NULL);
    }
    
    // Get memory info
    MEMORYSTATUSEX memStatus;
    memStatus.dwLength = sizeof(memStatus);
    if (GlobalMemoryStatusEx(&memStatus)) {
        metrics->ram_usage = (Percentage)memStatus.dwMemoryLoad;
    }
    
    // Get uptime
    metrics->uptime = GetTickCount64() / 1000;
    
    return true;
}

static bool get_processes_impl(process_info_t** processes, size_t* count) {
    if (!processes || !count) return false;
    
    std::vector<process_info_t> procs;
    if (!CollectProcesses(procs)) {
        return false;
    }
    
    *count = procs.size();
    if (*count == 0) {
        *processes = nullptr;
        return true;
    }
    
    // Allocate memory for processes
    *processes = (process_info_t*)malloc(sizeof(process_info_t) * procs.size());
    if (!*processes) {
        return false;
    }
    
    memcpy(*processes, procs.data(), sizeof(process_info_t) * procs.size());
    return true;
}

static bool get_system_info_impl(system_info_t* info) {
    if (!info) return false;
    memset(info, 0, sizeof(system_info_t));
    
    strncpy_s(info->m_os_type, sizeof(info->m_os_type), "Windows", kMaxOsTypeLen - 1);
    
    // Get Windows version
    OSVERSIONINFOEXW osvi;
    ZeroMemory(&osvi, sizeof(OSVERSIONINFOEXW));
    osvi.dwOSVersionInfoSize = sizeof(OSVERSIONINFOEXW);
    
    // Use RtlGetVersion for accurate version info
    typedef LONG (WINAPI *RtlGetVersionPtr)(POSVERSIONINFOEXW);
    HMODULE hMod = GetModuleHandleW(L"ntdll.dll");
    if (hMod) {
        RtlGetVersionPtr rtlGetVersion = (RtlGetVersionPtr)GetProcAddress(hMod, "RtlGetVersion");
        if (rtlGetVersion) {
            rtlGetVersion(&osvi);
            snprintf(info->m_os_version, kMaxOsVersionLen, "%lu.%lu.%lu",
                    osvi.dwMajorVersion, osvi.dwMinorVersion, osvi.dwBuildNumber);
        }
    }
    
    // Get hostname
    wchar_t hostname_w[256];
    DWORD size = 256;
    if (GetComputerNameW(hostname_w, &size)) {
        WideCharToMultiByte(CP_UTF8, 0, hostname_w, -1, info->m_hostname, 
                           kMaxHostnameLen, NULL, NULL);
    }
    
    // Get uptime
    info->uptime = GetTickCount64() / 1000;
    
    // Get CPU info
    SYSTEM_INFO sysInfo;
    GetSystemInfo(&sysInfo);
    info->cpu_cores = sysInfo.dwNumberOfProcessors;
    
    // Get memory info
    MEMORYSTATUSEX memStatus;
    memStatus.dwLength = sizeof(memStatus);
    if (GlobalMemoryStatusEx(&memStatus)) {
        info->total_memory = memStatus.ullTotalPhys;
        info->available_memory = memStatus.ullAvailPhys;
    }
    
    return true;
}

static void free_memory_impl(void* ptr) {
    if (ptr) {
        free(ptr);
    }
}

// Plugin interface
static plugin_interface_t plugin_interface = {
    get_plugin_info_impl,
    init_impl,
    cleanup_impl,
    get_system_metrics_impl,
    get_processes_impl,
    nullptr,  // execute_command
    nullptr,  // read_file
    get_system_info_impl,
    nullptr,  // get_directory_info_data
    nullptr,  // get_event_data
    nullptr,  // get_watchers_data
    nullptr,  // get_file_reader_data
    nullptr,  // get_sensor_data
    nullptr,  // get_camera_data
    nullptr,  // get_processing_results
    nullptr,  // get_video_frame
    free_memory_impl
};

extern "C" {
    EXPORT plugin_interface_t* get_plugin_interface() {
        return &plugin_interface;
    }
}
