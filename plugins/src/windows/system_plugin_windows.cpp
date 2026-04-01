#include "../../include/base_plugin.h"
#include "../../include/plugin_interface.h"
#include <windows.h>
#include <psapi.h>
#include <tlhelp32.h>
#include <algorithm>
#include <chrono>
#include <memory>

// Windows-specific system plugin implementation
class WindowsSystemPlugin : public IPlugin, public ISystemOperations {
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
    WindowsSystemPlugin() 
        : name_("windows_system_plugin")
        , version_("1.0.0")
        , description_("Windows system metrics and operations plugin")
        , status_(PluginStatus::Unloaded)
        , initialized_(false)
        , start_time_(std::chrono::system_clock::now()) {
    }

    // IPlugin implementation
    bool initialize() override {
        status_ = PluginStatus::Loading;
        notify_event(PluginEventType::Loaded, "Initializing Windows system plugin");
        
        try {
            // Validate Windows environment
            if (!validate_windows_environment()) {
                status_ = PluginStatus::Error;
                status_message_ = "Windows environment validation failed";
                return false;
            }
            
            initialized_ = true;
            status_ = PluginStatus::Active;
            status_message_ = "Windows system plugin initialized successfully";
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
        notify_event(PluginEventType::Unloaded, "Cleaning up Windows system plugin");
        
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
        return "windows";
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
        // In real implementation, would parse JSON config
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
            
            char hostname[256] = {0};
            DWORD hostname_size = sizeof(hostname);
            if (GetComputerNameA(hostname, &hostname_size)) {
                strncpy_s(metrics->hostname, sizeof(metrics->hostname), hostname, _TRUNCATE);
            }
            
            return true;
        }
        catch (...) {
            return false;
        }
    }

    bool get_processes(std::vector<ProcessInfo>* processes) override {
        if (!initialized_ || !processes) return false;
        
        HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if (snapshot == INVALID_HANDLE_VALUE) return false;
        
        PROCESSENTRY32 pe32;
        pe32.dwSize = sizeof(PROCESSENTRY32);
        
        processes->clear();
        
        if (Process32First(snapshot, &pe32)) {
            do {
                ProcessInfo info = {0};
                info.pid = pe32.th32ProcessID;
                strncpy_s(info.name, sizeof(info.name), pe32.szExeFile, _TRUNCATE);
                
                // Get additional process info
                HANDLE hProcess = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, FALSE, pe32.th32ProcessID);
                if (hProcess) {
                    PROCESS_MEMORY_COUNTERS pmc;
                    if (GetProcessMemoryInfo(hProcess, &pmc, sizeof(pmc))) {
                        info.memory_usage = pmc.WorkingSetSize;
                    }
                    CloseHandle(hProcess);
                }
                
                processes->push_back(info);
            } while (Process32Next(snapshot, &pe32));
        }
        
        CloseHandle(snapshot);
        return true;
    }

    bool execute_command(const std::string& command, CommandResult* result) override {
        if (!initialized_ || !result) return false;
        
        // Security check - implement whitelist
        if (!is_command_allowed(command)) {
            strncpy_s(result->error, sizeof(result->error), "Command not allowed", _TRUNCATE);
            return false;
        }
        
        // Execute command
        FILE* pipe = _popen(command.c_str(), "r");
        if (!pipe) {
            strncpy_s(result->error, sizeof(result->error), "Failed to execute command", _TRUNCATE);
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
            strcpy_s(result->stdout, output.length() + 1, output.c_str());
        }
        
        result->exit_code = _pclose(pipe);
        result->success = (result->exit_code == 0);
        
        return true;
    }

    bool read_file(const std::string& path, FileContent* content) override {
        if (!initialized_ || !content) return false;
        
        // Path traversal check
        if (path.find("..") != std::string::npos) {
            strncpy_s(content->error, sizeof(content->error), "Path traversal not allowed", _TRUNCATE);
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
            strncpy_s(content->error, sizeof(content->error), "Memory allocation failed", _TRUNCATE);
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
        
        strcpy_s(info->os_type, sizeof(info->os_type), "Windows");
        
        OSVERSIONINFOEX osvi;
        osvi.dwOSVersionInfoSize = sizeof(OSVERSIONINFOEX);
        if (GetVersionEx((OSVERSIONINFO*)&osvi)) {
            snprintf(info->os_version, sizeof(info->os_version), 
                    "Windows %d.%d Build %d", 
                    osvi.dwMajorVersion, osvi.dwMinorVersion, osvi.dwBuildNumber);
        }
        
        SYSTEM_INFO sysInfo;
        GetSystemInfo(&sysInfo);
        info->cpu_cores = sysInfo.dwNumberOfProcessors;
        
        MEMORYSTATUSEX memInfo;
        memInfo.dwLength = sizeof(MEMORYSTATUSEX);
        if (GlobalMemoryStatusEx(&memInfo)) {
            info->total_memory = memInfo.ullTotalPhys;
            info->available_memory = memInfo.ullAvailPhys;
        char hostname[256] = {0};
        DWORD hostname_size = sizeof(hostname);
        if (GetComputerNameA(hostname, &hostname_size)) {
            strncpy_s(info->os_type, sizeof(info->os_type), "Windows", _TRUNCATE);
        }
        
        info->uptime = get_system_uptime();
        
        return true;
    }

private:
    bool validate_windows_environment() {
        // Check if we're running on Windows
        OSVERSIONINFO osvi;
        osvi.dwOSVersionInfoSize = sizeof(OSVERSIONINFO);
        return GetVersionEx(&osvi) != FALSE;
    }

    float get_cpu_usage() {
        static ULARGE_INTEGER last_cpu, last_sys_cpu, last_user_cpu;
        static int num_processors = 0;
        static SYSTEM_INFO sysInfo;
        
        if (num_processors == 0) {
            GetSystemInfo(&sysInfo);
            num_processors = sysInfo.dwNumberOfProcessors;
        }
        
        ULARGE_INTEGER now, sys, user;
        
        if (GetProcessTimes(GetCurrentProcess(), 
                           (FILETIME*)&now.LowPart, (FILETIME*)&now.HighPart,
                           (FILETIME*)&sys.LowPart, (FILETIME*)&user.HighPart)) {
            
            float percent = (sys.QuadPart - last_sys_cpu.QuadPart) + 
                           (user.QuadPart - last_user_cpu.QuadPart);
            percent /= (now.QuadPart - last_cpu.QuadPart);
            percent /= num_processors;
            
            last_cpu = now;
            last_user_cpu = user;
            last_sys_cpu = sys;
            
            return percent * 100;
        }
        return 0.0f;
    }

    float get_memory_usage() {
        MEMORYSTATUSEX memInfo;
        memInfo.dwLength = sizeof(MEMORYSTATUSEX);
        if (GlobalMemoryStatusEx(&memInfo)) {
            return ((float)(memInfo.ullTotalPhys - memInfo.ullAvailPhys) / memInfo.ullTotalPhys) * 100.0f;
        }
        return 0.0f;
    }

    float get_disk_usage() {
        ULARGE_INTEGER free_bytes, total_bytes;
        if (GetDiskFreeSpaceExA("C:\\", &free_bytes, &total_bytes, NULL)) {
            return ((float)(total_bytes.QuadPart - free_bytes.QuadPart) / total_bytes.QuadPart) * 100.0f;
        }
        return 0.0f;
    }

    uint64_t get_system_uptime() {
        return GetTickCount64() / 1000;
    }

    bool is_command_allowed(const std::string& command) {
        // Simple whitelist implementation
        std::vector<std::string> allowed = {
            "ps", "top", "df", "free", "uptime", "whoami", "id", "uname", "date",
            "ls", "cat", "grep", "find", "wc", "head", "tail", "sort", "uniq",
            "netstat", "ss", "ip", "ifconfig", "ping", "systemctl", "service",
            "dir", "type", "tasklist", "wmic", "powershell"
        };
        
        std::string first_word = command.substr(0, command.find(' '));
        return std::find(allowed.begin(), allowed.end(), first_word) != allowed.end();
    }
};

// Factory for Windows system plugin
class WindowsSystemPluginFactory : public IPluginFactory {
public:
    std::unique_ptr<IPlugin> create_plugin() override {
        return std::make_unique<WindowsSystemPlugin>();
    }

    std::string get_factory_name() const override {
        return "WindowsSystemPluginFactory";
    }

    std::string get_plugin_name() const override {
        return "windows_system_plugin";
    }

    std::string get_plugin_version() const override {
        return "1.0.0";
    }

    std::string get_supported_platform() const override {
        return "windows";
    }

    bool validate_environment() const override {
        return GetVersionEx((OSVERSIONINFO*)&std::make_unique<OSVERSIONINFO>()) != FALSE;
    }

    std::vector<std::string> get_dependencies() const override {
        return {"kernel32.dll", "psapi.dll", "user32.dll"};
    }
};

// C interface exports
extern "C" {
    PLUGIN_EXPORT IPluginFactory* PLUGIN_CALL get_plugin_factory() {
        static WindowsSystemPluginFactory factory;
        return &factory;
    }

    PLUGIN_EXPORT const char* PLUGIN_CALL get_plugin_api_version() {
        return "1.0.0";
    }

    PLUGIN_EXPORT bool PLUGIN_CALL validate_plugin_environment() {
        OSVERSIONINFO osvi;
        osvi.dwOSVersionInfoSize = sizeof(OSVERSIONINFO);
        return GetVersionEx(&osvi) != FALSE;
    }
}
