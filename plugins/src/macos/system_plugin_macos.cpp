#include "../../include/base_plugin.h"
#include "../../include/plugin_interface.h"
#include <unistd.h>
#include <sys/types.h>
#include <sys/sysctl.h>
#include <sys/utsname.h>
#include <sys/statvfs.h>
#include <dirent.h>
#include <fstream>
#include <algorithm>
#include <chrono>
#include <memory>
#include <cstring>
#include <cstdlib>

// macOS system plugin implementation
class MacOSSystemPlugin : public IPlugin, public ISystemOperations {
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
    MacOSSystemPlugin() 
        : name_("macos_system_plugin")
        , version_("1.0.0")
        , description_("macOS system metrics and operations plugin")
        , status_(PluginStatus::Unloaded)
        , initialized_(false)
        , start_time_(std::chrono::system_clock::now()) {
    }

    // IPlugin implementation
    bool initialize() override {
        status_ = PluginStatus::Loading;
        notify_event(PluginEventType::Loaded, "Initializing macOS system plugin");
        
        try {
            // Validate macOS environment
            if (!validate_macos_environment()) {
                status_ = PluginStatus::Error;
                status_message_ = "macOS environment validation failed";
                return false;
            }
            
            initialized_ = true;
            status_ = PluginStatus::Active;
            status_message_ = "macOS system plugin initialized successfully";
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
        notify_event(PluginEventType::Unloaded, "Cleaning up macOS system plugin");
        
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
        return "macos";
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
        
        // Use sysctl to get process list on macOS
        int mib[4] = {CTL_KERN, KERN_PROC, KERN_PROC_ALL, 0};
        size_t size = 0;
        
        // Get required buffer size
        if (sysctl(mib, 4, NULL, &size, NULL, 0) < 0) {
            return false;
        }
        
        // Allocate buffer
        std::vector<char> buffer(size);
        struct kinfo_proc* proc_list = (struct kinfo_proc*)buffer.data();
        
        // Get process list
        if (sysctl(mib, 4, proc_list, &size, NULL, 0) < 0) {
            return false;
        }
        
        int proc_count = size / sizeof(struct kinfo_proc);
        processes->clear();
        
        for (int i = 0; i < proc_count; i++) {
            ProcessInfo info = {0};
            info.pid = proc_list[i].kp_proc.p_pid;
            
            // Get process name
            strncpy(info.name, proc_list[i].kp_proc.p_comm, sizeof(info.name) - 1);
            
            // Get memory info (simplified)
            info.memory_usage = 0; // Would need additional sysctl calls
            
            processes->push_back(info);
        }
        
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
            strncpy(info->os_type, "macOS", sizeof(info->os_type) - 1);
        }
        
        info->cpu_cores = sysconf(_SC_NPROCESSORS_ONLN);
        
        // Get memory info using sysctl
        int mib[2] = {CTL_HW, HW_MEMSIZE};
        uint64_t mem_size = 0;
        size_t len = sizeof(mem_size);
        if (sysctl(mib, 2, &mem_size, &len, NULL, 0) == 0) {
            info->total_memory = mem_size;
            
            // Get free memory
            mib[0] = CTL_VM;
            mib[1] = VM_FREE_COUNT;
            uint64_t free_count = 0;
            len = sizeof(free_count);
            if (sysctl(mib, 2, &free_count, &len, NULL, 0) == 0) {
                info->available_memory = free_count * getpagesize();
            }
        }
        
        if (gethostname(info->hostname, sizeof(info->hostname)) != 0) {
            strncpy(info->hostname, "unknown", sizeof(info->hostname) - 1);
        }
        
        info->uptime = get_system_uptime();
        
        return true;
    }

private:
    bool validate_macos_environment() {
        // Check if we're running on macOS
        struct utsname uts;
        if (uname(&uts) == 0) {
            return strcmp(uts.sysname, "Darwin") == 0;
        }
        return false;
    }

    float get_cpu_usage() {
        // macOS implementation using sysctl
        static uint64_t last_idle = 0, last_total = 0;
        
        int mib[2] = {CTL_HW, HW_CPU_FREQ};
        uint64_t cpu_freq = 0;
        size_t len = sizeof(cpu_freq);
        
        // Get CPU usage (simplified - would need more detailed implementation)
        // This is a basic placeholder
        return 0.0f;
    }

    float get_memory_usage() {
        int mib[2] = {CTL_HW, HW_MEMSIZE};
        uint64_t total_mem = 0;
        size_t len = sizeof(total_mem);
        
        if (sysctl(mib, 2, &total_mem, &len, NULL, 0) == 0) {
            mib[1] = VM_FREE_COUNT;
            uint64_t free_count = 0;
            len = sizeof(free_count);
            
            if (sysctl(mib, 2, &free_count, &len, NULL, 0) == 0) {
                uint64_t free_mem = free_count * getpagesize();
                return ((float)(total_mem - free_mem) / total_mem) * 100.0f;
            }
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
        int mib[2] = {CTL_KERN, KERN_BOOTTIME};
        struct timeval boottime;
        size_t len = sizeof(boottime);
        
        if (sysctl(mib, 2, &boottime, &len, NULL, 0) == 0) {
            uint64_t now = time(NULL);
            return now - boottime.tv_sec;
        }
        
        return 0;
    }

    bool is_command_allowed(const std::string& command) {
        // Simple whitelist implementation
        std::vector<std::string> allowed = {
            "ps", "top", "df", "free", "uptime", "whoami", "id", "uname", "date",
            "ls", "cat", "grep", "find", "wc", "head", "tail", "sort", "uniq",
            "netstat", "ss", "ip", "ifconfig", "ping", "systemctl", "service",
            "launchctl", "diskutil", "system_profiler"
        };
        
        std::string first_word = command.substr(0, command.find(' '));
        return std::find(allowed.begin(), allowed.end(), first_word) != allowed.end();
    }
};

// Factory for macOS system plugin
class MacOSSystemPluginFactory : public IPluginFactory {
public:
    std::unique_ptr<IPlugin> create_plugin() override {
        return std::make_unique<MacOSSystemPlugin>();
    }

    std::string get_factory_name() const override {
        return "MacOSSystemPluginFactory";
    }

    std::string get_plugin_name() const override {
        return "macos_system_plugin";
    }

    std::string get_plugin_version() const override {
        return "1.0.0";
    }

    std::string get_supported_platform() const override {
        return "macos";
    }

    bool validate_environment() const override {
        struct utsname uts;
        if (uname(&uts) == 0) {
            return strcmp(uts.sysname, "Darwin") == 0;
        }
        return false;
    }

    std::vector<std::string> get_dependencies() const override {
        return {"libSystem", "libpthread"};
    }
};

// C interface exports
extern "C" {
    PLUGIN_EXPORT IPluginFactory* PLUGIN_CALL get_plugin_factory() {
        static MacOSSystemPluginFactory factory;
        return &factory;
    }

    PLUGIN_EXPORT const char* PLUGIN_CALL get_plugin_api_version() {
        return "1.0.0";
    }

    PLUGIN_EXPORT bool PLUGIN_CALL validate_plugin_environment() {
        struct utsname uts;
        if (uname(&uts) == 0) {
            return strcmp(uts.sysname, "Darwin") == 0;
        }
        return false;
    }
}
