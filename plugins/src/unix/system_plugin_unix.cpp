#include "../../include/base_plugin.h"
#include "../../include/plugin_interface.h"
#include <unistd.h>
#include <sys/types.h>
#include <sys/sysinfo.h>
#include <sys/utsname.h>
#include <sys/statvfs.h>
#include <dirent.h>
#include <fstream>
#include <algorithm>
#include <chrono>
#include <memory>
#include <cstring>
#include <cstdlib>

// Unix/Linux system plugin implementation
class UnixSystemPlugin : public IPlugin, public ISystemOperations {
private:
    std::string name_;
    std::string version_;
    std::string description_;
    PluginStatus status_;
    std::string status_message_;
    PluginEventCallback event_callback_;
    bool initialized_;
    std::chrono::system_clock::time_point start_time_;

public:
    UnixSystemPlugin() 
        : name_("unix_system_plugin")
        , version_("1.0.0")
        , description_("Unix/Linux system metrics and operations plugin")
        , status_(PluginStatus::Unloaded)
        , initialized_(false)
        , start_time_(std::chrono::system_clock::now()) {
    }

    // IPlugin implementation
    bool initialize() override {
        status_ = PluginStatus::Loading;
        notify_event(PluginEventType::Loaded, "Initializing Unix system plugin");
        
        try {
            // Validate Unix environment
            if (!validate_unix_environment()) {
                status_ = PluginStatus::Error;
                status_message_ = "Unix environment validation failed";
                return false;
            }
            
            initialized_ = true;
            status_ = PluginStatus::Active;
            status_message_ = "Unix system plugin initialized successfully";
            notify_event(PluginEventType::StatusChanged, "Plugin activated");
            return true;
        }
        catch (const std::exception& e) {
            status_ = PluginStatus::Error;
            status_message_ = std::string("Initialization failed: ") + e.what();
            notify_event(PluginEventType::Error, status_message_);
            return false;
        }
    }

    void cleanup() override {
        status_ = PluginStatus::Unloading;
        notify_event(PluginEventType::Unloaded, "Cleaning up Unix system plugin");
        
        initialized_ = false;
        status_ = PluginStatus::Unloaded;
        status_message_ = "Plugin cleaned up";
    }

    bool is_initialized() const override {
        return initialized_;
    }

    std::string get_name() const override {
        return name_;
    }

    std::string get_version() const override {
        return version_;
    }

    std::string get_description() const override {
        return description_;
    }

    std::string get_platform() const override {
        return "unix";
    }

    std::vector<std::string> get_capabilities() const override {
        return {
            "system_metrics",
            "process_management", 
            "command_execution",
            "file_operations",
            "system_info"
        };
    }

    bool has_capability(const std::string& capability) const override {
        auto caps = get_capabilities();
        return std::find(caps.begin(), caps.end(), capability) != caps.end();
    }

    PluginStatus get_status() const override {
        return status_;
    }

    std::string get_status_message() const override {
        return status_message_;
    }

    bool is_healthy() const override {
        return status_ == PluginStatus::Active && initialized_;
    }

    void set_event_callback(PluginEventCallback callback) override {
        event_callback_ = callback;
    }

    void notify_event(PluginEventType type, const std::string& message) override {
        if (event_callback_) {
            event_callback_(type, name_, message);
        }
    }

    bool configure(const std::string& config_json) override {
        // Parse configuration (simplified)
        notify_event(PluginEventType::StatusChanged, "Configuration updated");
        return true;
    }

    std::string get_configuration() const override {
        return "{\"update_interval\": 30, \"max_processes\": 1000}";
    }

    bool prepare_reload() override {
        notify_event(PluginEventType::StatusChanged, "Preparing for reload");
        return true;
    }

    bool complete_reload() override {
        cleanup();
        return initialize();
    }

    bool can_reload() const override {
        return true;
    }

    // ISystemOperations implementation
    bool get_system_metrics(SystemMetrics* metrics) override {
        if (!initialized_ || !metrics) return false;
        
        try {
            metrics->cpu_usage = get_cpu_usage();
            metrics->ram_usage = get_memory_usage();
            metrics->disk_usage = get_disk_usage();
            metrics->uptime = get_system_uptime();
            
            if (gethostname(metrics->hostname, sizeof(metrics->hostname)) != 0) {
                strncpy(metrics->hostname, "unknown", sizeof(metrics->hostname) - 1);
            }
            
            return true;
        }
        catch (...) {
            return false;
        }
    }

    bool get_processes(std::vector<ProcessInfo>* processes) override {
        if (!initialized_ || !processes) return false;
        
        DIR* proc_dir = opendir("/proc");
        if (!proc_dir) return false;
        
        processes->clear();
        
        struct dirent* entry;
        while ((entry = readdir(proc_dir)) != NULL) {
            if (entry->d_name[0] >= '0' && entry->d_name[0] <= '9') {
                ProcessInfo info = {0};
                info.pid = atoi(entry->d_name);
                
                // Read process name from /proc/[pid]/comm
                char comm_path[256];
                snprintf(comm_path, sizeof(comm_path), "/proc/%s/comm", entry->d_name);
                std::ifstream comm_file(comm_path);
                if (comm_file.is_open()) {
                    comm_file.getline(info.name, sizeof(info.name));
                }
                
                // Read memory info from /proc/[pid]/statm
                char statm_path[256];
                snprintf(statm_path, sizeof(statm_path), "/proc/%s/statm", entry->d_name);
                std::ifstream statm_file(statm_path);
                if (statm_file.is_open()) {
                    unsigned long size, resident, share, text, lib, data, dt;
                    if (statm_file >> size >> resident >> share >> text >> lib >> data >> dt) {
                        info.memory_usage = resident * getpagesize();
                    }
                }
                
                // Read start time from /proc/[pid]/stat
                char stat_path[256];
                snprintf(stat_path, sizeof(stat_path), "/proc/%s/stat", entry->d_name);
                std::ifstream stat_file(stat_path);
                if (stat_file.is_open()) {
                    std::string line;
                    if (std::getline(stat_file, line)) {
                        // Parse start time (field 22)
                        std::istringstream iss(line);
                        std::string token;
                        for (int i = 0; i < 22; i++) {
                            if (!(iss >> token)) break;
                        }
                        info.start_time = std::stoull(token) / sysconf(_SC_CLK_TCK);
                    }
                }
                
                processes->push_back(info);
            }
        }
        
        closedir(proc_dir);
        return true;
    }

    bool execute_command(const std::string& command, CommandResult* result) override {
        if (!initialized_ || !result) return false;
        
        // Security check - implement whitelist
        if (!is_command_allowed(command)) {
            strncpy(result->error, "Command not allowed", sizeof(result->error) - 1);
            return false;
        }
        
        // Execute command
        FILE* pipe = popen(command.c_str(), "r");
        if (!pipe) {
            strncpy(result->error, "Failed to execute command", sizeof(result->error) - 1);
            return false;
        }
        
        // Read output
        char buffer[4096];
        std::string output;
        while (fgets(buffer, sizeof(buffer), pipe) != NULL) {
            output += buffer;
        }
        
        result->stdout = (char*)malloc(output.length() + 1);
        if (result->stdout) {
            strcpy(result->stdout, output.c_str());
        }
        
        result->exit_code = pclose(pipe);
        result->success = (result->exit_code == 0);
        
        return true;
    }

    bool read_file(const std::string& path, FileContent* content) override {
        if (!initialized_ || !content) return false;
        
        // Path traversal check
        if (path.find("..") != std::string::npos) {
            strncpy(content->error, "Path traversal not allowed", sizeof(content->error) - 1);
            return false;
        }
        
        FILE* file = fopen(path.c_str(), "rb");
        if (!file) {
            snprintf(content->error, sizeof(content->error), "Failed to open file: %s", path.c_str());
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
            strncpy(content->error, "Memory allocation failed", sizeof(content->error) - 1);
            return false;
        }
        
        size_t read_size = fread(content->content, 1, size, file);
        content->content[read_size] = '\0';
        content->size = read_size;
        content->success = true;
        
        fclose(file);
        return true;
    }

    bool get_system_info(SystemInfo* info) override {
        if (!initialized_ || !info) return false;
        
        struct utsname uts;
        if (uname(&uts) == 0) {
            strncpy(info->os_type, uts.sysname, sizeof(info->os_type) - 1);
            strncpy(info->os_version, uts.release, sizeof(info->os_version) - 1);
        } else {
            strncpy(info->os_type, "Unix", sizeof(info->os_type) - 1);
        }
        
        info->cpu_cores = sysconf(_SC_NPROCESSORS_ONLN);
        info->total_memory = sysconf(_SC_PHYS_PAGES) * sysconf(_SC_PAGESIZE);
        
        struct sysinfo si;
        if (sysinfo(&si) == 0) {
            info->available_memory = si.freeram * si.mem_unit;
            info->uptime = si.uptime;
        }
        
        if (gethostname(info->hostname, sizeof(info->hostname)) != 0) {
            strncpy(info->hostname, "unknown", sizeof(info->hostname) - 1);
        }
        
        return true;
    }

private:
    bool validate_unix_environment() {
        // Check if we're running on Unix/Linux
        struct utsname uts;
        return uname(&uts) == 0;
    }

    float get_cpu_usage() {
        // Linux implementation using /proc/stat
        std::ifstream file("/proc/stat");
        if (!file.is_open()) return 0.0f;
        
        std::string line;
        if (std::getline(file, line)) {
            unsigned long long user, nice, system, idle, iowait, irq, softirq;
            if (sscanf(line.c_str(), "cpu %llu %llu %llu %llu %llu %llu %llu",
                       &user, &nice, &system, &idle, &iowait, &irq, &softirq) == 7) {
                
                unsigned long long total = user + nice + system + idle + iowait + irq + softirq;
                unsigned long long work = user + nice + system + iowait + irq + softirq;
                
                static unsigned long long last_total = 0, last_work = 0;
                if (last_total != 0) {
                    float diff_total = total - last_total;
                    float diff_work = work - last_work;
                    float usage = (diff_work / diff_total) * 100.0f;
                    
                    last_total = total;
                    last_work = work;
                    return usage;
                }
                last_total = total;
                last_work = work;
            }
        }
        return 0.0f;
    }

    float get_memory_usage() {
        struct sysinfo si;
        if (sysinfo(&si) == 0) {
            return ((float)(si.totalram - si.freeram) / si.totalram) * 100.0f;
        }
        return 0.0f;
    }

    float get_disk_usage() {
        struct statvfs fs;
        if (statvfs("/", &fs) == 0) {
            unsigned long long total = fs.f_blocks * fs.f_bsize;
            unsigned long long free = fs.f_bfree * fs.f_bsize;
            return ((float)(total - free) / total) * 100.0f;
        }
        return 0.0f;
    }

    uint64_t get_system_uptime() {
        struct sysinfo si;
        if (sysinfo(&si) == 0) {
            return si.uptime;
        }
        return 0;
    }

    bool is_command_allowed(const std::string& command) {
        // Simple whitelist implementation
        std::vector<std::string> allowed = {
            "ps", "top", "df", "free", "uptime", "whoami", "id", "uname", "date",
            "ls", "cat", "grep", "find", "wc", "head", "tail", "sort", "uniq",
            "netstat", "ss", "ip", "ifconfig", "ping", "systemctl", "service"
        };
        
        std::string first_word = command.substr(0, command.find(' '));
        return std::find(allowed.begin(), allowed.end(), first_word) != allowed.end();
    }
};

// Factory for Unix system plugin
class UnixSystemPluginFactory : public IPluginFactory {
public:
    std::unique_ptr<IPlugin> create_plugin() override {
        return std::make_unique<UnixSystemPlugin>();
    }

    std::string get_factory_name() const override {
        return "UnixSystemPluginFactory";
    }

    std::string get_plugin_name() const override {
        return "unix_system_plugin";
    }

    std::string get_plugin_version() const override {
        return "1.0.0";
    }

    std::string get_supported_platform() const override {
        return "unix";
    }

    bool validate_environment() const override {
        struct utsname uts;
        return uname(&uts) == 0;
    }

    std::vector<std::string> get_dependencies() const override {
        return {"libc", "libpthread"};
    }
};

// C interface exports
extern "C" {
    PLUGIN_EXPORT IPluginFactory* PLUGIN_CALL get_plugin_factory() {
        static UnixSystemPluginFactory factory;
        return &factory;
    }

    PLUGIN_EXPORT const char* PLUGIN_CALL get_plugin_api_version() {
        return "1.0.0";
    }

    PLUGIN_EXPORT bool PLUGIN_CALL validate_plugin_environment() {
        struct utsname uts;
        return uname(&uts) == 0;
    }
}
