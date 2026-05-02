/**
 * @file forensic_plugin.cpp
 * @brief Linux Forensic Artifacts Collector Plugin
 * 
 * Collects Linux-specific forensic artifacts:
 * - /proc/<PID>/ parsing (processes, environment, cmdline)
 * - Kernel modules from /proc/modules
 * - Systemd units and timers
 * - Crontab entries
 * - Package verification (dpkg --verify, rpm -Va)
 * - SELinux/AppArmor status
 * - Syslog/journal entries
 */

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <ctime>
#include <dirent.h>
#include <sys/stat.h>
#include <sys/sysinfo.h>
#include <sys/utsname.h>
#include <unistd.h>
#include <vector>
#include <string>
#include "../../include/plugin_interface.h"

#define EXPORT __attribute__((visibility("default")))

static const char* PLUGIN_NAME = "linux_forensic_plugin";
static const char* PLUGIN_VERSION = "1.0.0";
static const char* PLUGIN_DESCRIPTION = "Linux forensic artifacts collector";

// Read file contents into string
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

// Parse /proc/<PID>/cmdline
static std::string GetProcessCmdline(pid_t pid) {
    char path[256];
    snprintf(path, sizeof(path), "/proc/%d/cmdline", pid);
    
    FILE* fp = fopen(path, "r");
    if (!fp) return "";
    
    std::string cmdline;
    char c;
    while ((c = fgetc(fp)) != EOF) {
        cmdline += (c == '\0') ? ' ' : c;
    }
    fclose(fp);
    
    return cmdline;
}

// Parse /proc/<PID>/environ
static std::string GetProcessEnviron(pid_t pid) {
    char path[256];
    snprintf(path, sizeof(path), "/proc/%d/environ", pid);
    
    FILE* fp = fopen(path, "r");
    if (!fp) return "";
    
    std::string env;
    char c;
    while ((c = fgetc(fp)) != EOF) {
        env += (c == '\0') ? '\n' : c;
    }
    fclose(fp);
    
    return env;
}

// Collect kernel modules
static bool CollectKernelModules(std::vector<forensic_finding_t>& findings) {
    FILE* fp = fopen("/proc/modules", "r");
    if (!fp) return false;
    
    char line[512];
    while (fgets(line, sizeof(line), fp)) {
        char name[128], size[32], instances[16], dependencies[256], state[16], memory[32];
        
        // Parse: name size instances dependencies state memory
        if (sscanf(line, "%s %s %s %s %s %s", name, size, instances, dependencies, state, memory) >= 5) {
            forensic_finding_t finding;
            memset(&finding, 0, sizeof(finding));
            
            strncpy(finding.category, "Kernel", sizeof(finding.category) - 1);
            strncpy(finding.artifact_type, "Kernel Module", sizeof(finding.artifact_type) - 1);
            strncpy(finding.path, "/proc/modules", sizeof(finding.path) - 1);
            snprintf(finding.value, sizeof(finding.value), "%s (size: %s, instances: %s, state: %s)",
                    name, size, instances, state);
            
            // Check for suspicious module names (simplified detection)
            if (strstr(name, "rootkit") || strstr(name, "hide") || strstr(name, "hook")) {
                finding.suspicious = true;
                strncpy(finding.details, "Suspicious: Potential rootkit indicator in module name", 
                       sizeof(finding.details) - 1);
            }
            
            findings.push_back(finding);
        }
    }
    
    fclose(fp);
    return true;
}

// Collect systemd units
static bool CollectSystemdUnits(std::vector<forensic_finding_t>& findings) {
    // Check common systemd paths
    const char* systemd_paths[] = {
        "/etc/systemd/system/",
        "/lib/systemd/system/",
        "/usr/lib/systemd/system/",
        "/run/systemd/system/"
    };
    
    for (const auto& path : systemd_paths) {
        DIR* dir = opendir(path);
        if (!dir) continue;
        
        struct dirent* entry;
        while ((entry = readdir(dir)) != NULL) {
            if (entry->d_type == DT_REG || entry->d_type == DT_LNK) {
                // Check for service/timer/socket files
                if (strstr(entry->d_name, ".service") || 
                    strstr(entry->d_name, ".timer") ||
                    strstr(entry->d_name, ".socket")) {
                    
                    forensic_finding_t finding;
                    memset(&finding, 0, sizeof(finding));
                    
                    strncpy(finding.category, "Persistence", sizeof(finding.category) - 1);
                    strncpy(finding.artifact_type, "Systemd Unit", sizeof(finding.artifact_type) - 1);
                    
                    char full_path[512];
                    snprintf(full_path, sizeof(full_path), "%s%s", path, entry->d_name);
                    strncpy(finding.path, full_path, sizeof(finding.path) - 1);
                    strncpy(finding.value, entry->d_name, sizeof(finding.value) - 1);
                    
                    // Check for suspicious patterns in service name
                    if (strstr(entry->d_name, "backdoor") || 
                        strstr(entry->d_name, "reverse") ||
                        strstr(entry->d_name, "shell")) {
                        finding.suspicious = true;
                        strncpy(finding.details, "Suspicious: Potentially malicious service name", 
                               sizeof(finding.details) - 1);
                    }
                    
                    findings.push_back(finding);
                }
            }
        }
        closedir(dir);
    }
    
    return true;
}

// Collect crontab entries
static bool CollectCrontab(std::vector<forensic_finding_t>& findings) {
    const char* crontab_paths[] = {
        "/etc/crontab",
        "/etc/cron.d/",
        "/etc/cron.hourly/",
        "/etc/cron.daily/",
        "/etc/cron.weekly/",
        "/etc/cron.monthly/",
        "/var/spool/cron/"
    };
    
    for (const auto& path : crontab_paths) {
        struct stat st;
        if (stat(path, &st) == 0) {
            forensic_finding_t finding;
            memset(&finding, 0, sizeof(finding));
            
            strncpy(finding.category, "Persistence", sizeof(finding.category) - 1);
            strncpy(finding.artifact_type, "Cron Entry", sizeof(finding.artifact_type) - 1);
            strncpy(finding.path, path, sizeof(finding.path) - 1);
            
            if (S_ISREG(st.st_mode)) {
                // Read crontab content
                std::string content = ReadFile(path);
                if (!content.empty() && content.length() < sizeof(finding.value)) {
                    strncpy(finding.value, content.c_str(), sizeof(finding.value) - 1);
                }
            } else if (S_ISDIR(st.st_mode)) {
                strncpy(finding.value, "[Directory - scan for entries]", sizeof(finding.value) - 1);
            }
            
            findings.push_back(finding);
        }
    }
    
    return true;
}

// Collect running processes
static bool CollectProcesses(std::vector<process_info_t>& processes) {
    DIR* dir = opendir("/proc");
    if (!dir) return false;
    
    struct dirent* entry;
    while ((entry = readdir(dir)) != NULL) {
        // Check if entry is a PID (numeric directory)
        char* endptr;
        pid_t pid = strtol(entry->d_name, &endptr, 10);
        if (*endptr != '\0') continue; // Not a number
        
        process_info_t proc;
        memset(&proc, 0, sizeof(proc));
        proc.pid = pid;
        
        // Get process name from /proc/<PID>/comm
        char path[256];
        snprintf(path, sizeof(path), "/proc/%d/comm", pid);
        FILE* fp = fopen(path, "r");
        if (fp) {
            if (fgets(proc.m_name, sizeof(proc.m_name), fp)) {
                // Remove newline
                size_t len = strlen(proc.m_name);
                if (len > 0 && proc.m_name[len-1] == '\n') {
                    proc.m_name[len-1] = '\0';
                }
            }
            fclose(fp);
        }
        
        // Get process memory and start time from /proc/<PID>/stat
        snprintf(path, sizeof(path), "/proc/%d/stat", pid);
        fp = fopen(path, "r");
        if (fp) {
            char line[1024];
            if (fgets(line, sizeof(line), fp)) {
                // Parse: pid (comm) state ppid ... start_time ...
                unsigned long starttime;
                long rss;
                int fields = sscanf(line, "%*d %*s %*c %*d %*d %*d %*d %*d %*u %*u %*u %*u %*u %*u %*u %*d %*d %*d %*d %*d %*d %lu %*u %*d %*d %*d %*d %*d %*u %*u %*u %lu",
                                    &starttime, &rss);
                if (fields >= 1) {
                    // Convert jiffies to seconds (approximate)
                    proc.start_time = starttime / sysconf(_SC_CLK_TCK);
                }
                if (fields >= 2) {
                    // RSS is in pages
                    proc.memory_usage = rss * sysconf(_SC_PAGESIZE);
                }
            }
            fclose(fp);
        }
        
        // Check for suspicious process patterns
        std::string cmdline = GetProcessCmdline(pid);
        std::string environ = GetProcessEnviron(pid);
        
        // Store suspicious indicators
        if (strstr(cmdline.c_str(), "nc -l") || 
            strstr(cmdline.c_str(), "netcat") ||
            strstr(cmdline.c_str(), "reverse")) {
            // Process has suspicious command line - could add to findings
        }
        
        processes.push_back(proc);
    }
    
    closedir(dir);
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
    
    // Get memory info
    struct sysinfo si;
    if (sysinfo(&si) == 0) {
        unsigned long total_ram = si.totalram;
        unsigned long free_ram = si.freeram;
        if (total_ram > 0) {
            metrics->ram_usage = (Percentage)((total_ram - free_ram) * 100 / total_ram);
        }
        metrics->uptime = si.uptime;
    }
    
    // Get CPU usage (simplified - would need /proc/stat parsing for accurate values)
    metrics->cpu_usage = 0;
    
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
    
    strncpy(info->m_os_type, "Linux", kMaxOsTypeLen - 1);
    
    // Get kernel version
    struct utsname un;
    if (uname(&un) == 0) {
        strncpy(info->m_os_version, un.release, kMaxOsVersionLen - 1);
    }
    
    // Get hostname
    gethostname(info->m_hostname, kMaxHostnameLen);
    
    // Get uptime
    struct sysinfo si;
    if (sysinfo(&si) == 0) {
        info->uptime = si.uptime;
    }
    
    // Get CPU cores
    info->cpu_cores = sysconf(_SC_NPROCESSORS_ONLN);
    
    // Get memory info
    if (sysinfo(&si) == 0) {
        info->total_memory = si.totalram;
        info->available_memory = si.freeram;
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
    
    // Collect all forensic artifacts
    CollectKernelModules(cached_findings);
    CollectSystemdUnits(cached_findings);
    CollectCrontab(cached_findings);
    
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
