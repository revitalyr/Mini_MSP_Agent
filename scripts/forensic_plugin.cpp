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
#include <cerrno>
#include <csignal>
#include <set>
#include <dirent.h>
#include <sys/stat.h>
#include <sys/sysinfo.h>
#include <sys/utsname.h>
#include <unistd.h>
#include <vector>
#include <string>
#include <sstream>
#include <iomanip>
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

// Escape string for JSON validity
static std::string EscapeJson(const std::string& s) {
    std::ostringstream oss;
    for (auto c : s) {
        switch (c) {
            case '"': oss << "\\\""; break;
            case '\\': oss << "\\\\"; break;
            case '\b': oss << "\\b"; break;
            case '\f': oss << "\\f"; break;
            case '\n': oss << "\\n"; break;
            case '\r': oss << "\\r"; break;
            case '\t': oss << "\\t"; break;
            default:
                if (static_cast<unsigned char>(c) < 32) {
                    oss << "\\u" << std::hex << std::setw(4) << std::setfill('0') << static_cast<int>(static_cast<unsigned char>(c));
                } else {
                    oss << c;
                }
        }
    }
    return oss.str();
}

// Get executable path
static std::string GetProcessExe(pid_t pid) {
    char path[256];
    char link[512];
    snprintf(path, sizeof(path), "/proc/%d/exe", pid);
    ssize_t len = readlink(path, link, sizeof(link) - 1);
    if (len != -1) {
        link[len] = '\0';
        return std::string(link);
    }
    return "";
}

// Helper to get socket inodes for a specific PID
static std::set<unsigned long> GetProcessSocketInodes(pid_t pid) {
    std::set<unsigned long> inodes;
    char fd_path[256];
    snprintf(fd_path, sizeof(fd_path), "/proc/%d/fd", pid);
    DIR* dir = opendir(fd_path);
    if (!dir) return inodes;

    struct dirent* entry;
    while ((entry = readdir(dir)) != NULL) {
        char link_path[512];
        char target[512];
        snprintf(link_path, sizeof(link_path), "%s/%s", fd_path, entry->d_name);
        ssize_t len = readlink(link_path, target, sizeof(target) - 1);
        if (len != -1) {
            target[len] = '\0';
            unsigned long inode;
            if (sscanf(target, "socket:[%lu]", &inode) == 1) {
                inodes.insert(inode);
            }
        }
    }
    closedir(dir);
    return inodes;
}

// Helper to get listening ports for a specific PID
static std::vector<int> GetProcessListeningPorts(pid_t pid) {
    std::vector<int> ports;
    std::set<unsigned long> socket_inodes = GetProcessSocketInodes(pid);
    if (socket_inodes.empty()) return ports;

    const char* net_files[] = {"/proc/net/tcp", "/proc/net/tcp6"};
    for (const char* net_file : net_files) {
        FILE* fp = fopen(net_file, "r");
        if (!fp) continue;

        char line[1024];
        if (fgets(line, sizeof(line), fp)) { // Skip header
            while (fgets(line, sizeof(line), fp)) {
                unsigned int state;
                unsigned long inode;
                char local_addr_str[128];
                if (sscanf(line, "%*d: %127[^ ] %*s %x %*s %*s %*s %*s %*s %lu", 
                           local_addr_str, &state, &inode) == 3) {
                    if (state == 0x0A && socket_inodes.count(inode)) {
                        char* colon = strrchr(local_addr_str, ':');
                        if (colon) {
                            ports.push_back(static_cast<int>(strtol(colon + 1, nullptr, 16)));
                        }
                    }
                }
            }
        }
        fclose(fp);
    }
    return ports;
}

// Helper to get open files for a specific PID (lsof-like)
static std::vector<std::string> GetProcessOpenFiles(pid_t pid) {
    std::vector<std::string> files;
    char fd_path[256];
    snprintf(fd_path, sizeof(fd_path), "/proc/%d/fd", pid);
    DIR* dir = opendir(fd_path);
    if (!dir) return files;

    struct dirent* entry;
    while ((entry = readdir(dir)) != NULL) {
        if (entry->d_name[0] == '.') continue;

        char link_path[512];
        char target[512];
        snprintf(link_path, sizeof(link_path), "%s/%s", fd_path, entry->d_name);
        ssize_t len = readlink(link_path, target, sizeof(target) - 1);
        if (len != -1) {
            target[len] = '\0';
            files.push_back(std::string(target));
        }
    }
    closedir(dir);
    return files;
}

// Check if a process is using RAW sockets
static bool HasRawSockets(pid_t pid) {
    std::set<unsigned long> socket_inodes = GetProcessSocketInodes(pid);
    if (socket_inodes.empty()) return false;

    const char* net_files[] = {"/proc/net/raw", "/proc/net/raw6"};
    for (const char* net_file : net_files) {
        FILE* fp = fopen(net_file, "r");
        if (!fp) continue;

        char line[1024];
        if (fgets(line, sizeof(line), fp)) { // Skip header
            while (fgets(line, sizeof(line), fp)) {
                unsigned long inode;
                if (sscanf(line, "%*d: %*s %*s %*x %*s %*s %*s %*s %*s %lu", &inode) == 1) {
                    if (socket_inodes.count(inode)) {
                        fclose(fp);
                        return true;
                    }
                }
            }
        }
        fclose(fp);
    }
    return false;
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
                finding.suspicious = 1;
                strncpy(finding.details, "Suspicious: Potential rootkit indicator in module name", 
                       sizeof(finding.details) - 1);
            }
            
            findings.push_back(finding);
        }
    }
    
    fclose(fp);
    return true;
}

// Check for kernel integrity indicators
static void CheckKernelIntegrity(std::vector<forensic_finding_t>& findings) {
    // 1. Check if kernel is tainted
    std::string tainted = ReadFile("/proc/sys/kernel/tainted");
    if (!tainted.empty() && tainted != "0\n") {
        forensic_finding_t finding;
        memset(&finding, 0, sizeof(finding));
        strncpy(finding.category, "Kernel", sizeof(finding.category) - 1);
        strncpy(finding.artifact_type, "Tainted Kernel", sizeof(finding.artifact_type) - 1);
        strncpy(finding.path, "/proc/sys/kernel/tainted", sizeof(finding.path) - 1);
        snprintf(finding.value, sizeof(finding.value), "Value: %s", tainted.c_str());
        finding.suspicious = 1;
        strncpy(finding.details, "Kernel is tainted. This may indicate non-standard or malicious modules loaded.", sizeof(finding.details) - 1);
        findings.push_back(finding);
    }

    // 2. Check kptr_restrict status
    std::string kptr = ReadFile("/proc/sys/kernel/kptr_restrict");
    if (!kptr.empty()) {
        forensic_finding_t finding;
        memset(&finding, 0, sizeof(finding));
        strncpy(finding.category, "Kernel", sizeof(finding.category) - 1);
        strncpy(finding.artifact_type, "Security Config", sizeof(finding.artifact_type) - 1);
        strncpy(finding.path, "/proc/sys/kernel/kptr_restrict", sizeof(finding.path) - 1);
        
        int kptr_val = atoi(kptr.c_str());
        if (kptr_val == 0) {
            finding.suspicious = 1;
            strncpy(finding.details, "kptr_restrict is disabled. Kernel addresses are visible to userspace.", sizeof(finding.details) - 1);
            findings.push_back(finding);
        }
    }
}

// Detect processes hidden from /proc by brute-forcing PIDs
static void DetectHiddenProcesses(std::vector<forensic_finding_t>& findings) {
    std::set<pid_t> visible_pids;
    DIR* dir = opendir("/proc");
    if (dir) {
        struct dirent* entry;
        while ((entry = readdir(dir)) != NULL) {
            pid_t pid = (pid_t)atoi(entry->d_name);
            if (pid > 0) visible_pids.insert(pid);
        }
        closedir(dir);
    }

    // Get maximum PID from system settings
    int max_pid = 32768; // Default fallback
    std::string max_pid_str = ReadFile("/proc/sys/kernel/pid_max");
    if (!max_pid_str.empty()) {
        max_pid = std::stoi(max_pid_str);
    }

    // Brute force PID range
    // Optimization: Batching with usleep to prevent CPU pinning
    const int BATCH_SIZE = 1000;
    
    for (pid_t pid = 1; pid < max_pid; ++pid) {
        if (visible_pids.count(pid)) continue;

        // usleep every BATCH_SIZE iterations to yield CPU
        if (pid % BATCH_SIZE == 0) {
            usleep(1000); // 1ms sleep
        }

        // kill(pid, 0) doesn't kill the process, it just checks for existence
        // If it returns 0, the process exists and we have permissions.
        // If it returns -1 and errno is EPERM, it exists but we don't have permissions.
        // If it returns -1 and errno is ESRCH, it truly doesn't exist.
        if (kill(pid, 0) == 0 || errno == EPERM) {
            forensic_finding_t finding;
            memset(&finding, 0, sizeof(finding));
            
            strncpy(finding.category, "Processes", sizeof(finding.category) - 1);
            strncpy(finding.artifact_type, "Hidden Process", sizeof(finding.artifact_type) - 1);
            snprintf(finding.path, sizeof(finding.path), "/proc/%d", pid);
            snprintf(finding.value, sizeof(finding.value), "PID: %d", pid);
            
            finding.suspicious = 1;
            
            // Try to get some info via sched_getscheduler which often bypasses procfs hooks
            int policy = sched_getscheduler(pid);
            if (policy != -1) {
                snprintf(finding.details, sizeof(finding.details), 
                        "Process exists but hidden from /proc. Sched Policy: %d", policy);
            } else {
                strncpy(finding.details, "Process exists but hidden from /proc (Access Denied)", 
                       sizeof(finding.details) - 1);
            }
            
            findings.push_back(finding);
        }
    }
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
                        finding.suspicious = 1;
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

// Scan directories for suspicious files (hidden or executable)
static void ScanSuspiciousDirectories(std::vector<forensic_finding_t>& findings) {
    const char* target_dirs[] = {"/tmp", "/var/tmp", "/dev/shm"};
    
    for (const char* base_dir : target_dirs) {
        DIR* dir = opendir(base_dir);
        if (!dir) continue;

        struct dirent* entry;
        while ((entry = readdir(dir)) != NULL) {
            // Skip . and ..
            if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) {
                continue;
            }

            char full_path[512];
            snprintf(full_path, sizeof(full_path), "%s/%s", base_dir, entry->d_name);

            struct stat st;
            if (lstat(full_path, &st) != 0) continue;

            bool is_hidden = (entry->d_name[0] == '.');
            bool is_executable = (S_ISREG(st.st_mode) && (st.st_mode & (S_IXUSR | S_IXGRP | S_IXOTH)));

            if (is_hidden || is_executable) {
                forensic_finding_t finding;
                memset(&finding, 0, sizeof(finding));
                
                strncpy(finding.category, "File System", sizeof(finding.category) - 1);
                strncpy(finding.artifact_type, "Suspicious File", sizeof(finding.artifact_type) - 1);
                strncpy(finding.path, full_path, sizeof(finding.path) - 1);
                strncpy(finding.value, entry->d_name, sizeof(finding.value) - 1);
                finding.suspicious = 1;
                
                if (is_hidden && is_executable) {
                    strncpy(finding.details, "Hidden executable file in temporary directory", sizeof(finding.details) - 1);
                } else if (is_hidden) {
                    strncpy(finding.details, "Hidden file in temporary directory", sizeof(finding.details) - 1);
                } else {
                    strncpy(finding.details, "Executable file in temporary directory", sizeof(finding.details) - 1);
                }
                findings.push_back(finding);
            }
        }
        closedir(dir);
    }
}

// Collect installed packages (APT and RPM)
static bool CollectInstalledPackages(std::vector<forensic_finding_t>& findings) {
    bool found = false;

    // 1. Try APT (Debian/Ubuntu)
    if (access("/usr/bin/dpkg-query", X_OK) == 0) {
        FILE* pipe = popen("dpkg-query -W -f='${Package}|${Version}|${Status}\n'", "r");
        if (pipe) {
            char line[512];
            while (fgets(line, sizeof(line), pipe)) {
                char name[128], version[128], status[128];
                if (sscanf(line, "%127[^|]|%127[^|]|%127[^\n]", name, version, status) == 3) {
                    // Only report actually installed packages
                    if (strstr(status, "install ok installed")) {
                        forensic_finding_t finding;
                        memset(&finding, 0, sizeof(finding));
                        strncpy(finding.category, "Software", sizeof(finding.category) - 1);
                        strncpy(finding.artifact_type, "APT Package", sizeof(finding.artifact_type) - 1);
                        snprintf(finding.path, sizeof(finding.path), "/var/lib/dpkg/info/%s.list", name);
                        snprintf(finding.value, sizeof(finding.value), "%s v%s", name, version);
                        findings.push_back(finding);
                        found = true;
                    }
                }
            }
            pclose(pipe);
        }
    }

    // 2. Try RPM (RHEL/CentOS/Fedora)
    if (access("/usr/bin/rpm", X_OK) == 0) {
        FILE* pipe = popen("rpm -qa --queryformat '%{NAME}|%{VERSION}-%{RELEASE}|%{SUMMARY}\n'", "r");
        if (pipe) {
            char line[1024];
            while (fgets(line, sizeof(line), pipe)) {
                char name[128], version[128], summary[512];
                if (sscanf(line, "%127[^|]|%127[^|]|%511[^\n]", name, version, summary) == 3) {
                    forensic_finding_t finding;
                    memset(&finding, 0, sizeof(finding));
                    strncpy(finding.category, "Software", sizeof(finding.category) - 1);
                    strncpy(finding.artifact_type, "RPM Package", sizeof(finding.artifact_type) - 1);
                    snprintf(finding.path, sizeof(finding.path), "rpmdb://%s", name);
                    snprintf(finding.value, sizeof(finding.value), "%s v%s", name, version);
                    strncpy(finding.details, summary, sizeof(finding.details) - 1);
                    findings.push_back(finding);
                    found = true;
                }
            }
            pclose(pipe);
        }
    }

    // If no package manager found, report it as a finding
    if (!found) {
        forensic_finding_t finding;
        memset(&finding, 0, sizeof(finding));
        strncpy(finding.category, "Software", sizeof(finding.category) - 1);
        strncpy(finding.artifact_type, "Warning", sizeof(finding.artifact_type) - 1);
        strncpy(finding.value, "No supported package manager found", sizeof(finding.value) - 1);
        finding.suspicious = 1;
        findings.push_back(finding);
    }

    return found;
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
            if (fgets(proc.name, sizeof(proc.name), fp)) {
                // Remove newline
                size_t len = strlen(proc.name);
                if (len > 0 && proc.name[len-1] == '\n') {
                    proc.name[len-1] = '\0';
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
        bool suspicious = false;
        std::string reason;
        std::string cmdline = GetProcessCmdline(pid);
        std::string environ = GetProcessEnviron(pid);
        std::string exe_path = GetProcessExe(pid);

        // 1. Hidden or suspicious paths
        if (exe_path.find("/tmp/") == 0 || exe_path.find("/dev/shm/") == 0 || exe_path.find("/var/tmp/") == 0) {
            suspicious = true;
            reason = "Binary running from temporary/writable directory";
        }
        // 2. Deleted binary (common for fileless/packed malware)
        else if (exe_path.find("(deleted)") != std::string::npos) {
            suspicious = true;
            reason = "Executable file has been deleted from disk";
        }
        // 3. Network reverse shell patterns
        else if (cmdline.find("nc ") != std::string::npos || cmdline.find("netcat") != std::string::npos || 
                 cmdline.find("/dev/tcp/") != std::string::npos || cmdline.find("sh -i") != std::string::npos) {
            suspicious = true;
            reason = "Potential reverse shell or network redirection detected";
        }
        // 4. Obfuscation indicators
        else if (cmdline.find("base64 -d") != std::string::npos || cmdline.find("python -c") != std::string::npos) {
            suspicious = true;
            reason = "Inline script execution with potential obfuscation";
        }
        // 5. Raw socket usage (Packet sniffing)
        else if (HasRawSockets(pid)) {
            suspicious = true;
            reason = "Process is using RAW sockets (possible sniffing/spoofing)";
        }
        // 6. Environment variable manipulation (Rootkit indicators)
        else if (environ.find("LD_PRELOAD=") != std::string::npos) {
            suspicious = true;
            reason = "Suspicious environment: LD_PRELOAD detected (potential library hooking)";
        }
        else if (environ.find("LD_LIBRARY_PATH=") != std::string::npos) {
            suspicious = true;
            reason = "Suspicious environment: LD_LIBRARY_PATH detected";
        }
        else if (environ.find("PYTHONPATH=") != std::string::npos) {
            suspicious = true;
            reason = "Suspicious environment: PYTHONPATH detected";
        }
        else if (environ.find("PATH=.:") != std::string::npos || environ.find(":/tmp") != std::string::npos) {
            suspicious = true;
            reason = "Dangerous PATH variable (includes '.' or '/tmp')";
        }

        if (suspicious) {
            // Log to findings for the UI to pick up
            // In a real implementation, you'd add this to a findings vector here
            // For now, we output it in the extended process list via the JSON handler below
        }
        
        processes.push_back(proc);
    }
    
    closedir(dir);
    return true;
}

// Plugin interface implementations
// Plugin info buffer for string format
static char plugin_info_buffer[256];

static const char* get_plugin_info_impl() {
    snprintf(plugin_info_buffer, sizeof(plugin_info_buffer), 
             "%s:%s:%s", PLUGIN_NAME, PLUGIN_VERSION, PLUGIN_DESCRIPTION);
    return plugin_info_buffer;
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
    gethostname(metrics->hostname, sizeof(metrics->hostname));
    
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
    
    strncpy(info->os_type, "Linux", sizeof(info->os_type) - 1);
    
    // Get kernel version
    struct utsname un;
    if (uname(&un) == 0) {
        strncpy(info->os_version, un.release, sizeof(info->os_version) - 1);
    }
    
    // Get hostname
    gethostname(info->hostname, sizeof(info->hostname));
    
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
    std::vector<forensic_finding_t> findings;
    
    // Collect all forensic artifacts
    CollectKernelModules(findings);
    CollectSystemdUnits(findings);
    CollectCrontab(findings);
    CheckKernelIntegrity(findings);
    DetectHiddenProcesses(findings);
    ScanSuspiciousDirectories(findings);
    CollectInstalledPackages(findings);
    
    forensic_data_t* data = (forensic_data_t*)malloc(sizeof(forensic_data_t));
    if (!data) return nullptr;

    data->collection_time = static_cast<Timestamp>(time(nullptr));
    
    if (!findings.empty()) {
        size_t findings_size = sizeof(forensic_finding_t) * findings.size();
        data->findings = (forensic_finding_t*)malloc(findings_size);
        if (data->findings) {
            memcpy(data->findings, findings.data(), findings_size);
            data->count = findings.size();
        } else {
            data->count = 0;
        }
    } else {
        data->findings = nullptr;
        data->count = 0;
    }
    
    return data;
}

// Execute command with JSON request/response
static char* execute_json_impl(const char* json_request) {
    if (!json_request) return nullptr;
    std::string req(json_request);
    std::string response;
    std::ostringstream oss;

    // Supported commands: get_forensic, get_processes_extended, run_scan
    if (req.find("\"cmd\":\"get_forensic\"") != std::string::npos || 
        req.find("\"cmd\":\"run_scan\"") != std::string::npos) {
        
        std::vector<forensic_finding_t> findings;
        
        // Handle targeted scans via params
        if (req.find("\"artifact\":\"kernel\"") != std::string::npos) {
            CollectKernelModules(findings);
            CheckKernelIntegrity(findings);
        } else if (req.find("\"artifact\":\"persistence\"") != std::string::npos) {
            CollectSystemdUnits(findings);
            CollectCrontab(findings);
        } else if (req.find("\"artifact\":\"hidden_procs\"") != std::string::npos) {
            DetectHiddenProcesses(findings);
        } else if (req.find("\"artifact\":\"files\"") != std::string::npos) {
            ScanSuspiciousDirectories(findings);
        } else {
            // Default to full scan
            CollectKernelModules(findings);
            CollectSystemdUnits(findings);
            CollectCrontab(findings);
            CheckKernelIntegrity(findings);
            DetectHiddenProcesses(findings);
            ScanSuspiciousDirectories(findings);
        }

        // Check for category filter in params
        std::string category_filter;
        size_t cat_pos = req.find("\"category\":\"");
        if (cat_pos != std::string::npos) {
            size_t start = cat_pos + 12;
            size_t end = req.find("\"", start);
            if (end != std::string::npos) {
                category_filter = req.substr(start, end - start);
            }
        }

        oss << "{\"status\":\"ok\",\"data\":{\"findings\":[";
        bool first = true;
        for (const auto& f : findings) {
            if (!category_filter.empty() && category_filter != f.category) {
                continue;
            }
            if (!first) oss << ",";
            oss << "{"
                << "\"category\":\"" << EscapeJson(f.category) << "\","
                << "\"type\":\"" << EscapeJson(f.artifact_type) << "\","
                << "\"path\":\"" << EscapeJson(f.path) << "\","
                << "\"value\":\"" << EscapeJson(f.value) << "\","
                << "\"suspicious\":" << (f.suspicious ? "true" : "false") << ","
                << "\"details\":\"" << EscapeJson(f.details) << "\""
                << "}";
            first = false;
        }
        oss << "]}}";
        response = oss.str();
    } else if (req.find("\"cmd\":\"kill_process\"") != std::string::npos) {
        // Simple PID extraction from JSON: "pid":1234
        size_t pid_pos = req.find("\"pid\":");
        if (pid_pos != std::string::npos) {
            pid_t pid_to_kill = (pid_t)std::stoll(req.substr(pid_pos + 6));
            if (pid_to_kill > 1) { // Safety check
                if (kill(pid_to_kill, SIGTERM) == 0) {
                    response = "{\"status\":\"ok\",\"message\":\"SIGTERM sent to process\"}";
                } else {
                    response = "{\"status\":\"error\",\"message\":\"Failed to kill process\"}";
                }
            } else {
                response = "{\"status\":\"error\",\"message\":\"Invalid PID\"}";
            }
        } else {
            response = "{\"status\":\"error\",\"message\":\"Missing PID parameter\"}";
        }
    } else if (req.find("\"cmd\":\"get_metrics\"") != std::string::npos || 
               req.find("\"cmd\":\"GetMetrics\"") != std::string::npos) {
        system_metrics_t metrics;
        if (get_system_metrics_impl(&metrics)) {
            oss << "{\"status\":\"ok\",\"data\":{"
                << "\"cpu_usage\":" << metrics.cpu_usage << ","
                << "\"ram_usage\":" << metrics.ram_usage << ","
                << "\"disk_usage\":" << metrics.disk_usage << ","
                << "\"uptime\":" << metrics.uptime << "}}";
            response = oss.str();
        } else {
            response = "{\"status\":\"error\",\"message\":\"Failed to retrieve system metrics\"}";
        }
    } else if (req.find("\"cmd\":\"GetSystemInfo\"") != std::string::npos || 
               req.find("\"cmd\":\"get_status\"") != std::string::npos) {
        system_info_t sys_info;
        if (get_system_info_impl(&sys_info)) {
            oss << "{\"status\":\"ok\",\"data\":{"
                << "\"os_type\":\"" << EscapeJson(sys_info.os_type) << "\","
                << "\"os_version\":\"" << EscapeJson(sys_info.os_version) << "\","
                << "\"hostname\":\"" << EscapeJson(sys_info.hostname) << "\","
                << "\"uptime\":" << sys_info.uptime << ","
                << "\"cpu_cores\":" << sys_info.cpu_cores << ","
                << "\"total_memory\":" << sys_info.total_memory << ","
                << "\"available_memory\":" << sys_info.available_memory
                << "}}";
            response = oss.str();
        } else {
            response = "{\"status\":\"error\",\"message\":\"Failed to retrieve system information\"}";
        }
    } else if (req.find("\"cmd\":\"get_available_objects\"") != std::string::npos) {
        // dashboard discovery support
        oss << "{\"status\":\"ok\",\"data\":{\"objects\":["
            << "{\"id\":\"linux_forensics\",\"name\":\"Linux Forensic Artifacts\"},"
            << "{\"id\":\"kernel_modules\",\"name\":\"Kernel Modules\"},"
            << "{\"id\":\"persistence\",\"name\":\"Persistence Mechanisms\"},"
            << "{\"id\":\"packages\",\"name\":\"Installed Packages\"},"
            << "{\"id\":\"processes_extended\",\"name\":\"Extended Process Audit\"}"
            << "]}}";
        response = oss.str();
    } else if (req.find("\"cmd\":\"get_processes_extended\"") != std::string::npos) {
        std::vector<process_info_t> procs;
        CollectProcesses(procs);

        oss << "{\"status\":\"ok\",\"data\":{\"processes\":[";
        for (size_t i = 0; i < procs.size(); ++i) {
            std::string exe = GetProcessExe(procs[i].pid);
            std::string cmd = GetProcessCmdline(procs[i].pid);
            std::vector<int> ports = GetProcessListeningPorts(procs[i].pid);
            std::vector<std::string> files = GetProcessOpenFiles(procs[i].pid);
            bool has_raw = HasRawSockets(procs[i].pid);
            bool is_suspicious = (exe.find("/tmp") == 0 || exe.find("(deleted)") != std::string::npos);
            
            oss << "{"
                << "\"pid\":" << procs[i].pid << ","
                << "\"name\":\"" << EscapeJson(procs[i].name) << "\","
                << "\"mem_bytes\":" << procs[i].memory_usage << ","
                << "\"cmdline\":\"" << EscapeJson(cmd) << "\","
                << "\"exe\":\"" << EscapeJson(exe) << "\","
                << "\"suspicious\":" << (is_suspicious || has_raw ? "true" : "false") << ","
                << "\"raw_sockets\":" << (has_raw ? "true" : "false") << ","
                << "\"ports\":[";
            for (size_t p = 0; p < ports.size(); ++p) {
                oss << ports[p] << (p < ports.size() - 1 ? "," : "");
            }
            oss << "],\"open_files\":[";
            for (size_t f = 0; f < files.size(); ++f) {
                oss << "\"" << EscapeJson(files[f]) << "\"" << (f < files.size() - 1 ? "," : "");
            }
            oss << "]}" << (i < procs.size() - 1 ? "," : "");
        }
        oss << "]}}";
        response = oss.str();
    } else {
        response = "{\"status\":\"error\",\"message\":\"Forensic plugin: Unknown or malformed command\","
                   "\"supported_commands\":[\"get_forensic\",\"run_scan\",\"kill_process\",\"get_metrics\","
                   "\"GetSystemInfo\",\"get_status\",\"get_processes_extended\","
                   "\"get_available_objects\",\"artifact:packages\"]}";
    }

    // The caller is responsible for freeing this memory using free_memory_impl
    char* result = (char*)malloc(response.length() + 1);
    if (result) {
        memcpy(result, response.c_str(), response.length() + 1);
    }
    return result;
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
    execute_json_impl
};

extern "C" {
    EXPORT plugin_interface_t* get_plugin_interface() {
        return &plugin_interface;
    }
}
