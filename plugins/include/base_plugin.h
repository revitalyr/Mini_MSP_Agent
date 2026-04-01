#pragma once

#include <memory>
#include <string>
#include <vector>
#include <functional>
#include <chrono>

#ifdef _WIN32
    #define PLUGIN_EXPORT __declspec(dllexport)
    #define PLUGIN_CALL __cdecl
#else
    #define PLUGIN_EXPORT __attribute__((visibility("default")))
    #define PLUGIN_CALL
#endif

// Forward declarations
struct SystemMetrics;
struct ProcessInfo;
struct FileContent;
struct CommandResult;
struct SystemInfo;

// Data structures
struct SystemMetrics {
    float cpu_usage;
    float ram_usage;
    float disk_usage;
    uint64_t uptime;
    char hostname[256];
};

struct ProcessInfo {
    uint32_t pid;
    char name[256];
    uint64_t memory_usage;
    uint64_t start_time;
    float cpu_usage;
};

struct FileContent {
    char* content;
    size_t size;
    bool success;
    char error[256];
};

struct CommandResult {
    char* output;
    int exit_code;
    bool success;
    char error[256];
};

struct SystemInfo {
    char os_type[64];
    char os_version[64];
    char hostname[256];
    uint32_t cpu_cores;
    uint64_t total_memory;
    uint64_t available_memory;
    uint64_t uptime;
};

// Plugin event types
enum class PluginEventType {
    Loaded,
    Unloaded,
    Error,
    StatusChanged
};

// Plugin status
enum class PluginStatus {
    Unloaded,
    Loading,
    Loaded,
    Active,
    Error,
    Unloading
};

// Plugin event callback
using PluginEventCallback = std::function<void(PluginEventType, const std::string&, const std::string&)>;

// Pure virtual base plugin interface
class IPlugin {
public:
    virtual ~IPlugin() = default;
    
    // Plugin lifecycle
    virtual bool initialize() = 0;
    virtual void cleanup() = 0;
    virtual bool is_initialized() const = 0;
    
    // Plugin information
    virtual std::string get_name() const = 0;
    virtual std::string get_version() const = 0;
    virtual std::string get_description() const = 0;
    virtual std::string get_platform() const = 0;
    
    // Capabilities
    virtual std::vector<std::string> get_capabilities() const = 0;
    virtual bool has_capability(const std::string& capability) const = 0;
    
    // Status and health
    virtual PluginStatus get_status() const = 0;
    virtual std::string get_status_message() const = 0;
    virtual bool is_healthy() const = 0;
    
    // Event handling
    virtual void set_event_callback(PluginEventCallback callback) = 0;
    virtual void notify_event(PluginEventType type, const std::string& message) = 0;
    
    // Configuration
    virtual bool configure(const std::string& config_json) = 0;
    virtual std::string get_configuration() const = 0;
    
    // Hot-reload support
    virtual bool prepare_reload() = 0;
    virtual bool complete_reload() = 0;
    virtual bool can_reload() const = 0;
};

// System operations interface (subset of IPlugin)
class ISystemOperations {
public:
    virtual ~ISystemOperations() = default;
    
    // System metrics
    virtual bool get_system_metrics(SystemMetrics* metrics) = 0;
    
    // Process management
    virtual bool get_processes(std::vector<ProcessInfo>* processes) = 0;
    
    // Command execution
    virtual bool execute_command(const std::string& command, CommandResult* result) = 0;
    
    // File operations
    virtual bool read_file(const std::string& path, FileContent* content) = 0;
    
    // System information
    virtual bool get_system_info(SystemInfo* info) = 0;
};

// Plugin factory interface
class IPluginFactory {
public:
    virtual ~IPluginFactory() = default;
    
    // Plugin creation
    virtual std::unique_ptr<IPlugin> create_plugin() = 0;
    
    // Factory information
    virtual std::string get_factory_name() const = 0;
    virtual std::string get_plugin_name() const = 0;
    virtual std::string get_plugin_version() const = 0;
    virtual std::string get_supported_platform() const = 0;
    
    // Validation
    virtual bool validate_environment() const = 0;
    virtual std::vector<std::string> get_dependencies() const = 0;
};

// Plugin registry entry
struct PluginRegistryEntry {
    std::string name;
    std::string version;
    std::string platform;
    std::string library_path;
    std::unique_ptr<IPluginFactory> factory;
    PluginStatus status;
    std::string status_message;
    std::chrono::system_clock::time_point last_loaded;
    std::chrono::system_clock::time_point last_unloaded;
    
    PluginRegistryEntry() : status(PluginStatus::Unloaded) {}
};

// Plugin manager interface
class IPluginManager {
public:
    virtual ~IPluginManager() = default;
    
    // Plugin lifecycle
    virtual bool load_plugin(const std::string& library_path) = 0;
    virtual bool unload_plugin(const std::string& plugin_name) = 0;
    virtual bool reload_plugin(const std::string& plugin_name) = 0;
    virtual bool unload_all_plugins() = 0;
    
    // Plugin discovery
    virtual bool discover_plugins(const std::string& directory) = 0;
    virtual std::vector<std::string> get_available_plugins() const = 0;
    virtual std::vector<std::string> get_loaded_plugins() const = 0;
    
    // Plugin access
    virtual IPlugin* get_plugin(const std::string& name) = 0;
    virtual IPlugin* get_system_plugin() = 0;
    virtual std::vector<IPlugin*> get_plugins_by_capability(const std::string& capability) = 0;
    
    // Status and monitoring
    virtual std::vector<PluginRegistryEntry> get_plugin_registry() const = 0;
    virtual bool is_plugin_loaded(const std::string& name) const = 0;
    virtual PluginStatus get_plugin_status(const std::string& name) const = 0;
    
    // Event handling
    virtual void set_global_event_callback(PluginEventCallback callback) = 0;
    virtual void enable_hot_reload(bool enable) = 0;
    virtual bool is_hot_reload_enabled() const = 0;
};

// C interface for FFI compatibility
extern "C" {
    // Plugin entry points
    PLUGIN_EXPORT IPluginFactory* PLUGIN_CALL get_plugin_factory();
    PLUGIN_EXPORT const char* PLUGIN_CALL get_plugin_api_version();
    PLUGIN_EXPORT bool PLUGIN_CALL validate_plugin_environment();
    
    // Manager entry point (for the main plugin manager)
    PLUGIN_EXPORT IPluginManager* PLUGIN_CALL create_plugin_manager();
    PLUGIN_EXPORT void PLUGIN_CALL destroy_plugin_manager(IPluginManager* manager);
}
