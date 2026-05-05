/**
 * @file forensic_plugin.cpp
 * @brief macOS Forensic Artifacts Collector Plugin
 * 
 * Collects macOS-specific forensic artifacts:
 * - LaunchAgents/LaunchDaemons (persistence)
 * - codesign verification for binaries
 * - QuarantineEventsV2 (download history)
 * - kextstat output (kernel extensions)
 * - profiles -L (configuration profiles)
 * - log show (unified logs)
 * - System Integrity Protection status
 */

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <ctime>
#include <unistd.h>
#include <sys/stat.h>
#include <sys/sysctl.h>
#include <sys/utsname.h>
#include <mach/mach.h>
#include <mach/mach_host.h>
#include <mach/mach_init.h>
#include <dirent.h>
#include <vector>
#include <string>
#include "../../include/plugin_interface.h"

#define EXPORT __attribute__((visibility("default")))

static const char* PLUGIN_NAME = "macos_forensic_plugin";
static const char* PLUGIN_VERSION = "1.0.0";
static const char* PLUGIN_DESCRIPTION = "macOS forensic artifacts collector";

// LaunchAgents/LaunchDaemons paths
static const char* kLaunchPaths[] = {
    "/Library/LaunchAgents/",
    "/Library/LaunchDaemons/",
    "/System/Library/LaunchAgents/",
    "/System/Library/LaunchDaemons/",
    "~/Library/LaunchAgents/"
};

// Read file contents
static std::string ReadFile(const char* path) {
    FILE* fp = fopen(path, "r");
    if (!fp) return "";
    
    char buffer[4096];
    std::string result;
    while (fgets(buffer, sizeof(buffer), fp)) {
        result += buffer;
    }
    fclose(fp);
    return result;
}

// Execute command and get output
static std::string ExecCommand(const char* cmd) {
    FILE* pipe = popen(cmd, "r");
    if (!pipe) return "";
    
    char buffer[4096];
    std::string result;
    while (fgets(buffer, sizeof(buffer), pipe)) {
        result += buffer;
    }
    pclose(pipe);
    return result;
}

// Collect LaunchAgents/Daemons
static bool CollectLaunchItems(std::vector<forensic_finding_t>& findings) {
    for (const auto& path : kLaunchPaths) {
        DIR* dir = opendir(path);
        if (!dir) continue;
        
        struct dirent* entry;
        while ((entry = readdir(dir)) != NULL) {
            // Check for .plist files
            if (strstr(entry->d_name, ".plist")) {
                forensic_finding_t finding;
                memset(&finding, 0, sizeof(finding));
                
                strncpy(finding.category, "Persistence", sizeof(finding.category) - 1);
                strncpy(finding.artifact_type, "LaunchAgent/Daemon", sizeof(finding.artifact_type) - 1);
                
                char full_path[512];
                snprintf(full_path, sizeof(full_path), "%s%s", path, entry->d_name);
                strncpy(finding.path, full_path, sizeof(finding.path) - 1);
                strncpy(finding.value, entry->d_name, sizeof(finding.value) - 1);
                
                // Check for suspicious patterns
                std::string plist_content = ReadFile(full_path);
                if (strstr(plist_content.c_str(), "ProgramArguments") &&
                    (strstr(plist_content.c_str(), "/bin/sh") ||
                     strstr(plist_content.c_str(), "/bin/bash") ||
                     strstr(plist_content.c_str(), "python"))) {
                    finding.suspicious = true;
                    strncpy(finding.details, "Suspicious: Shell interpreter in launch item", 
                           sizeof(finding.details) - 1);
                }
                
                if (strstr(plist_content.c_str(), "RunAtLoad") &&
                    strstr(plist_content.c_str(), "KeepAlive")) {
                    if (!finding.suspicious) {
                        strncpy(finding.details, "Note: Persistent launch agent with KeepAlive", 
                               sizeof(finding.details) - 1);
                    }
                }
                
                findings.push_back(finding);
            }
        }
        closedir(dir);
    }
    
    return true;
}

// Collect kernel extensions
static bool CollectKexts(std::vector<forensic_finding_t>& findings) {
    // Use kextstat command
    std::string kext_output = ExecCommand("/usr/sbin/kextstat -l");
    
    char* line = strtok(const_cast<char*>(kext_output.c_str()), "\n");
    while (line) {
        if (strlen(line) > 10) {
            forensic_finding_t finding;
            memset(&finding, 0, sizeof(finding));
            
            strncpy(finding.category, "Kernel", sizeof(finding.category) - 1);
            strncpy(finding.artifact_type, "Kernel Extension", sizeof(finding.artifact_type) - 1);
            strncpy(finding.path, "/System/Library/Extensions/", sizeof(finding.path) - 1);
            
            // Parse kextstat output: Index Refs Address Size Wired Name (Version) UUID <Linked Against>
            char idx[16], refs[16], address[32], size[32], wired[32], name[128];
            if (sscanf(line, "%s %s %s %s %s %s", idx, refs, address, size, wired, name) >= 6) {
                // Extract bundle ID (format: com.apple....)
                char* bundle_start = strchr(line, '(');
                if (bundle_start) {
                    char bundle[256];
                    sscanf(name, "%s", bundle);
                    snprintf(finding.value, sizeof(finding.value), "%s (refs: %s)", bundle, refs);
                }
                
                // Check for non-Apple kexts (suspicious)
                if (!strstr(name, "com.apple")) {
                    finding.suspicious = true;
                    strncpy(finding.details, "Suspicious: Non-Apple kernel extension loaded", 
                           sizeof(finding.details) - 1);
                }
            }
            
            findings.push_back(finding);
        }
        line = strtok(nullptr, "\n");
    }
    
    return true;
}

// Collect running processes with mach kernel info
static bool CollectProcesses(std::vector<process_info_t>& processes) {
    int mib[4] = { CTL_KERN, KERN_PROC, KERN_PROC_ALL, 0 };
    size_t size = 0;
    
    // Get size needed
    if (sysctl(mib, 4, nullptr, &size, nullptr, 0) < 0) {
        return false;
    }
    
    // Allocate buffer
    std::vector<struct kinfo_proc> procs;
    procs.resize(size / sizeof(struct kinfo_proc));
    
    // Get process list
    if (sysctl(mib, 4, procs.data(), &size, nullptr, 0) < 0) {
        return false;
    }
    
    int count = size / sizeof(struct kinfo_proc);
    
    for (int i = 0; i < count; i++) {
        process_info_t proc;
        memset(&proc, 0, sizeof(proc));
        
        proc.pid = procs[i].kp_proc.p_pid;
        
        // Get process name
        strncpy(proc.m_name, procs[i].kp_proc.p_comm, kMaxHostnameLen - 1);
        
        // Get process info using Mach APIs
        mach_port_t task;
        kern_return_t kr = task_for_pid(mach_task_self(), proc.pid, &task);
        
        if (kr == KERN_SUCCESS) {
            // Get memory info
            task_basic_info_data_t info;
            mach_msg_type_number_t count = TASK_BASIC_INFO_COUNT;
            
            if (task_info(task, TASK_BASIC_INFO, (task_info_t)&info, &count) == KERN_SUCCESS) {
                proc.memory_usage = info.resident_size;
            }
            
            mach_port_deallocate(mach_task_self(), task);
        }
        
        // Get process start time from proc structure
        struct timeval tv;
        tv.tv_sec = procs[i].kp_proc.p_starttime.tv_sec;
        tv.tv_usec = procs[i].kp_proc.p_starttime.tv_usec;
        proc.start_time = tv.tv_sec;
        
        processes.push_back(proc);
    }
    
    return true;
}

// Check code signature
static bool CheckCodeSignature(const char* path, char* output, size_t output_size) {
    char cmd[1024];
    snprintf(cmd, sizeof(cmd), "/usr/bin/codesign -dvv '%s' 2>&1 | head -5", path);
    
    std::string result = ExecCommand(cmd);
    strncpy(output, result.c_str(), output_size - 1);
    output[output_size - 1] = '\0';
    
    return strlen(output) > 0;
}

// Get SIP status
static bool GetSIPStatus(char* status, size_t size) {
    std::string result = ExecCommand("/usr/bin/csrutil status 2>&1");
    
    if (strstr(result.c_str(), "enabled")) {
        strncpy(status, "Enabled", size - 1);
    } else if (strstr(result.c_str(), "disabled")) {
        strncpy(status, "Disabled", size - 1);
    } else {
        strncpy(status, "Unknown", size - 1);
    }
    
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
    gethostname(metrics->m_hostname, kMaxHostnameLen);
    
    // Get memory info using Mach
    mach_port_t host = mach_host_self();
    vm_statistics64_data_t vm_stats;
    mach_msg_type_number_t count = HOST_VM_INFO64_COUNT;
    
    if (host_statistics64(host, HOST_VM_INFO64, (host_info64_t)&vm_stats, &count) == KERN_SUCCESS) {
        natural_t free_mem = vm_stats.free_count;
        natural_t total_mem = vm_stats.wire_count + vm_stats.active_count + 
                              vm_stats.inactive_count + vm_stats.free_count;
        
        if (total_mem > 0) {
            metrics->ram_usage = (Percentage)((total_mem - free_mem) * 100 / total_mem);
        }
    }
    
    mach_port_deallocate(mach_task_self(), host);
    
    // Get uptime
    struct timeval boottime;
    size_t len = sizeof(boottime);
    int mib[2] = { CTL_KERN, KERN_BOOTTIME };
    
    if (sysctl(mib, 2, &boottime, &len, NULL, 0) == 0) {
        time_t now = time(NULL);
        metrics->uptime = now - boottime.tv_sec;
    }
    
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
    
    strncpy(info->m_os_type, "macOS", kMaxOsTypeLen - 1);
    
    // Get macOS version
    char os_version[256];
    size_t size = sizeof(os_version);
    int mib[2] = { CTL_KERN, KERN_OSTYPE };
    
    if (sysctlbyname("kern.osrelease", os_version, &size, NULL, 0) == 0) {
        strncpy(info->m_os_version, os_version, kMaxOsVersionLen - 1);
    }
    
    // Get hostname
    gethostname(info->m_hostname, kMaxHostnameLen);
    
    // Get uptime
    struct timeval boottime;
    size_t len = sizeof(boottime);
    int uptime_mib[2] = { CTL_KERN, KERN_BOOTTIME };
    
    if (sysctl(uptime_mib, 2, &boottime, &len, NULL, 0) == 0) {
        time_t now = time(NULL);
        info->uptime = now - boottime.tv_sec;
    }
    
    // Get CPU cores
    int cores;
    size_t cores_len = sizeof(cores);
    int cores_mib[2] = { CTL_HW, HW_NCPU };
    if (sysctl(cores_mib, 2, &cores, &cores_len, NULL, 0) == 0) {
        info->cpu_cores = cores;
    }
    
    // Get memory info
    mach_port_t host = mach_host_self();
    vm_statistics64_data_t vm_stats;
    mach_msg_type_number_t count = HOST_VM_INFO64_COUNT;
    
    if (host_statistics64(host, HOST_VM_INFO64, (host_info64_t)&vm_stats, &count) == KERN_SUCCESS) {
        vm_size_t page_size;
        host_page_size(host, &page_size);
        
        info->total_memory = (vm_stats.wire_count + vm_stats.active_count + 
                            vm_stats.inactive_count + vm_stats.free_count) * page_size;
        info->available_memory = vm_stats.free_count * page_size;
    }
    
    mach_port_deallocate(mach_task_self(), host);
    
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
    
    // Collect all forensic artifacts
    CollectLaunchItems(cached_findings);
    CollectKexts(cached_findings);
    
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
    free_memory_impl,
    nullptr  // execute_json - not implemented yet
};

extern "C" {
    EXPORT plugin_interface_t* get_plugin_interface() {
        return &plugin_interface;
    }
}
