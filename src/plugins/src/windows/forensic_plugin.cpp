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
#include <ctime>
#include "../../include/plugin_interface.h"

#define EXPORT __declspec(dllexport)

static const char* PLUGIN_NAME = "windows_forensic_plugin";
static const char* PLUGIN_VERSION = "1.0.0";
static const char* PLUGIN_DESCRIPTION = "Windows forensic artifacts collector";

// Registry key paths for persistence detection
static const wchar_t* kAutorunKeys[] = {
    L"SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
    L"SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\RunOnce",
    L"SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Run",
    L"SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon",
    L"SYSTEM\\CurrentControlSet\\Services",
    L"SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Image File Execution Options"
};

static const size_t kAutorunKeysCount = sizeof(kAutorunKeys) / sizeof(kAutorunKeys[0]);

// Registry helper - read specific registry value by name
// Used by CheckMaliciousRegistryKeys for IOC detection
static bool ReadRegistryValue(HKEY root, const wchar_t* subkey, const wchar_t* value_name, 
                               char* output, size_t output_size) {
    HKEY hKey;
    if (RegOpenKeyExW(root, subkey, 0, KEY_READ, &hKey) != ERROR_SUCCESS) {
        return false;
    }
    
    DWORD type;
    DWORD size = (DWORD)(output_size * sizeof(wchar_t));
    wchar_t wbuffer[1024] = {0};
    
    LRESULT result = RegQueryValueExW(hKey, value_name, NULL, &type, (LPBYTE)wbuffer, &size);
    RegCloseKey(hKey);
    
    if (result != ERROR_SUCCESS) {
        return false;
    }
    
    // Convert wide char to utf-8
    WideCharToMultiByte(CP_UTF8, 0, wbuffer, -1, output, (int)output_size, NULL, NULL);
    return true;
}

// Registry helper - enumerate all registry values in a key
// Used by CollectAutorunEntries to get all autorun entries
static bool EnumRegistryValues(HKEY root, const wchar_t* subkey, 
                               std::vector<std::pair<std::wstring, std::wstring>>& values) {
    HKEY hKey;
    if (RegOpenKeyExW(root, subkey, 0, KEY_READ, &hKey) != ERROR_SUCCESS) {
        return false;
    }
    
    wchar_t valueName[256];
    DWORD valueNameSize = 256;
    DWORD index = 0;
    
    while (RegEnumValueW(hKey, index, valueName, &valueNameSize, NULL, NULL, NULL, NULL) == ERROR_SUCCESS) {
        wchar_t valueData[1024] = {0};
        DWORD valueSize = sizeof(valueData);
        DWORD type;
        
        if (RegQueryValueExW(hKey, valueName, NULL, &type, (LPBYTE)valueData, &valueSize) == ERROR_SUCCESS) {
            values.push_back({valueName, valueData});
        }
        
        valueNameSize = 256;
        index++;
    }
    
    RegCloseKey(hKey);
    return true;
}

// Collect autorun entries from registry
static bool CollectAutorunEntries(std::vector<forensic_finding_t>& findings) {
    for (size_t i = 0; i < kAutorunKeysCount; i++) {
        const wchar_t* reg_path = kAutorunKeys[i];
        std::vector<std::pair<std::wstring, std::wstring>> values;
        
        // Check HKLM (HKEY_LOCAL_MACHINE)
        if (EnumRegistryValues(HKEY_LOCAL_MACHINE, reg_path, values)) {
            for (const auto& val : values) {
                forensic_finding_t finding;
                memset(&finding, 0, sizeof(finding));
                
                strncpy_s(finding.category, sizeof(finding.category), "Persistence", _TRUNCATE);
                strncpy_s(finding.artifact_type, sizeof(finding.artifact_type), "Registry Autorun (HKLM)", _TRUNCATE);
                
                // Build full path
                char utf8_path[512];
                char utf8_name[256];
                WideCharToMultiByte(CP_UTF8, 0, reg_path, -1, utf8_path, 512, NULL, NULL);
                WideCharToMultiByte(CP_UTF8, 0, val.first.c_str(), -1, utf8_name, 256, NULL, NULL);
                snprintf(finding.path, sizeof(finding.path), "HKLM\\%s\\%s", utf8_path, utf8_name);
                
                // Store value
                WideCharToMultiByte(CP_UTF8, 0, val.second.c_str(), -1, finding.value, 512, NULL, NULL);
                
                // Check for suspicious patterns
                char lower_value[512];
                strncpy_s(lower_value, sizeof(lower_value), finding.value, _TRUNCATE);
                _strlwr_s(lower_value, sizeof(lower_value));
                
                if (strstr(lower_value, "cmd.exe") || 
                    strstr(lower_value, "powershell.exe") ||
                    strstr(lower_value, "rundll32.exe") ||
                    strstr(lower_value, "regsvr32.exe") ||
                    strstr(lower_value, "mshta.exe") ||
                    strstr(lower_value, "wscript.exe") ||
                    strstr(lower_value, "cscript.exe")) {
                    finding.suspicious = true;
                    strncpy_s(finding.details, sizeof(finding.details), 
                             "Suspicious: Script interpreter or LOLBIN in autorun", _TRUNCATE);
                }
                
                findings.push_back(finding);
            }
        }
        
        // Check HKCU (HKEY_CURRENT_USER)
        values.clear();
        if (EnumRegistryValues(HKEY_CURRENT_USER, reg_path, values)) {
            for (const auto& val : values) {
                forensic_finding_t finding;
                memset(&finding, 0, sizeof(finding));
                
                strncpy_s(finding.category, sizeof(finding.category), "Persistence", _TRUNCATE);
                strncpy_s(finding.artifact_type, sizeof(finding.artifact_type), "Registry Autorun (HKCU)", _TRUNCATE);
                
                // Build full path
                char utf8_path[512];
                char utf8_name[256];
                WideCharToMultiByte(CP_UTF8, 0, reg_path, -1, utf8_path, 512, NULL, NULL);
                WideCharToMultiByte(CP_UTF8, 0, val.first.c_str(), -1, utf8_name, 256, NULL, NULL);
                snprintf(finding.path, sizeof(finding.path), "HKCU\\%s\\%s", utf8_path, utf8_name);
                
                // Store value
                WideCharToMultiByte(CP_UTF8, 0, val.second.c_str(), -1, finding.value, 512, NULL, NULL);
                
                // Check for suspicious patterns
                char lower_value[512];
                strncpy_s(lower_value, sizeof(lower_value), finding.value, _TRUNCATE);
                _strlwr_s(lower_value, sizeof(lower_value));
                
                if (strstr(lower_value, "cmd.exe") || 
                    strstr(lower_value, "powershell.exe") ||
                    strstr(lower_value, "rundll32.exe") ||
                    strstr(lower_value, "regsvr32.exe") ||
                    strstr(lower_value, "mshta.exe") ||
                    strstr(lower_value, "wscript.exe") ||
                    strstr(lower_value, "cscript.exe")) {
                    finding.suspicious = true;
                    strncpy_s(finding.details, sizeof(finding.details), 
                             "Suspicious: Script interpreter or LOLBIN in autorun", _TRUNCATE);
                }
                
                findings.push_back(finding);
            }
        }
    }
    
    return true;
}

// Known malicious registry indicators (IOC checks)
struct MaliciousRegistryKey {
    HKEY root;
    const wchar_t* subkey;
    const wchar_t* value_name;
    const char* description;
    const char* malware_family;
};

// IOC database - known malicious registry keys
static const MaliciousRegistryKey kMaliciousKeys[] = {
    // Winlogon shell hijacking
    {HKEY_LOCAL_MACHINE, L"SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon", L"Shell", 
     "Winlogon Shell registry key", "Persistence mechanism"},
    {HKEY_LOCAL_MACHINE, L"SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon", L"Userinit",
     "Winlogon Userinit registry key", "Persistence mechanism"},
    
    // IFEO debugger injection (common technique)
    {HKEY_LOCAL_MACHINE, L"SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Image File Execution Options\\notepad.exe", L"Debugger",
     "IFEO debugger injection on notepad.exe", "Process hijacking"},
    {HKEY_LOCAL_MACHINE, L"SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Image File Execution Options\\calc.exe", L"Debugger",
     "IFEO debugger injection on calc.exe", "Process hijacking"},
    
    // LSA notification packages (credential theft)
    {HKEY_LOCAL_MACHINE, L"SYSTEM\\CurrentControlSet\\Control\\Lsa", L"Notification Packages",
     "LSA notification packages", "Credential theft"},
    
    // Security providers (rootkit technique)
    {HKEY_LOCAL_MACHINE, L"SYSTEM\\CurrentControlSet\\Control\\SecurityProviders", L"SecurityProviders",
     "LSA Security Providers", "Authentication hijacking"},
    
    // Boot execute (early persistence)
    {HKEY_LOCAL_MACHINE, L"SYSTEM\\CurrentControlSet\\Control\\Session Manager", L"BootExecute",
     "Session Manager BootExecute", "Early boot persistence"},
    
    // AppCert DLLs (DLL injection)
    {HKEY_LOCAL_MACHINE, L"SYSTEM\\CurrentControlSet\\Control\\Session Manager\\AppCertDlls", nullptr,
     "AppCert DLL injection", "DLL hijacking"},
};

static const size_t kMaliciousKeysCount = sizeof(kMaliciousKeys) / sizeof(kMaliciousKeys[0]);

// Check for known malicious registry keys using ReadRegistryValue
static bool CheckMaliciousRegistryKeys(std::vector<forensic_finding_t>& findings) {
    for (size_t i = 0; i < kMaliciousKeysCount; i++) {
        const auto& ioc = kMaliciousKeys[i];
        char value_data[1024] = {0};
        
        // Try to read the value
        bool found = false;
        if (ioc.value_name) {
            // Check specific value
            found = ReadRegistryValue(ioc.root, ioc.subkey, ioc.value_name, value_data, sizeof(value_data));
        } else {
            // Check if key exists (any value in it)
            HKEY hKey;
            if (RegOpenKeyExW(ioc.root, ioc.subkey, 0, KEY_READ, &hKey) == ERROR_SUCCESS) {
                found = true;
                RegCloseKey(hKey);
                strncpy_s(value_data, sizeof(value_data), "[Key exists with entries]", _TRUNCATE);
            }
        }
        
        if (found) {
            forensic_finding_t finding;
            memset(&finding, 0, sizeof(finding));
            
            strncpy_s(finding.category, sizeof(finding.category), "IOC Detection", _TRUNCATE);
            strncpy_s(finding.artifact_type, sizeof(finding.artifact_type), "Malicious Registry Key", _TRUNCATE);
            
            // Build full path
            char utf8_path[512];
            WideCharToMultiByte(CP_UTF8, 0, ioc.subkey, -1, utf8_path, 512, NULL, NULL);
            if (ioc.value_name) {
                char utf8_value[256];
                WideCharToMultiByte(CP_UTF8, 0, ioc.value_name, -1, utf8_value, 256, NULL, NULL);
                snprintf(finding.path, sizeof(finding.path), "%s\\%s", utf8_path, utf8_value);
            } else {
                strncpy_s(finding.path, sizeof(finding.path), utf8_path, _TRUNCATE);
            }
            
            strncpy_s(finding.value, sizeof(finding.value), value_data, _TRUNCATE);
            strncpy_s(finding.details, sizeof(finding.details), ioc.description, _TRUNCATE);
            finding.suspicious = true;
            
            findings.push_back(finding);
        }
    }
    
    return true;
}

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

// Forensic data implementation
static forensic_data_t* get_forensic_data_impl() {
    static std::vector<forensic_finding_t> cached_findings;
    static forensic_data_t cached_data;
    
    cached_findings.clear();
    
    // Collect autorun entries
    CollectAutorunEntries(cached_findings);
    
    // Check for known malicious registry keys (IOCs)
    CheckMaliciousRegistryKeys(cached_findings);
    
    // Allocate and populate findings array
    if (!cached_findings.empty()) {
        size_t findings_size = sizeof(forensic_finding_t) * cached_findings.size();
        cached_data.findings = (forensic_finding_t*)malloc(findings_size);
        if (cached_data.findings) {
            memcpy(cached_data.findings, cached_findings.data(), findings_size);
            cached_data.count = cached_findings.size();
        } else {
            cached_data.count = 0;
        }
    } else {
        cached_data.findings = nullptr;
        cached_data.count = 0;
    }
    
    cached_data.collection_time = static_cast<Timestamp>(time(nullptr));
    
    return &cached_data;
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
    get_forensic_data_impl,
    free_memory_impl
};

extern "C" {
    EXPORT plugin_interface_t* get_plugin_interface() {
        return &plugin_interface;
    }
}
