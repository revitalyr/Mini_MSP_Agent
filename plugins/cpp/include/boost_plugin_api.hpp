/**
 * @file boost_plugin_api.hpp
 * @brief Modern C++23 Plugin API using Boost.DLL
 * 
 * Replaces legacy C-style interface with type-safe C++23 API.
 * Provides automatic plugin discovery, loading, and lifecycle management.
 */

#pragma once

#include <boost/dll.hpp>
#include <boost/filesystem.hpp>
#include <string>
#include <vector>
#include <memory>
#include <functional>
#include <expected>
#include <span>
#include <chrono>
#include <format>
#include <source_location>

namespace msp::plugins {

namespace dll = boost::dll;
namespace fs = boost::filesystem;

// =============================================================================
// Error Handling
// =============================================================================

enum class PluginError {
    NotFound,
    LoadFailed,
    SymbolMissing,
    InitFailed,
    InvalidVersion,
    AlreadyLoaded,
    NotLoaded,
    CommandNotSupported,
    ExecutionFailed
};

struct PluginErrorInfo {
    PluginError code;
    std::string message;
    std::source_location location;
    
    PluginErrorInfo(PluginError c, std::string m, 
                    std::source_location loc = std::source_location::current())
        : code(c), message(std::move(m)), location(loc) {}
};

template<typename T>
using PluginResult = std::expected<T, PluginErrorInfo>;

// =============================================================================
// Data Structures (JSON-serializable)
// =============================================================================

struct SystemInfo {
    std::string platform;
    std::string hostname;
    std::string architecture;
    std::string version;
    double cpu_usage{0.0};
    double memory_usage{0.0};
    uint64_t total_memory{0};
    uint64_t available_memory{0};
    double disk_usage{0.0};
    uint64_t uptime_seconds{0};
    
    // Serialize to JSON
    [[nodiscard]] std::string to_json() const;
    static std::expected<SystemInfo, std::string> from_json(std::string_view json);
};

struct ProcessInfo {
    uint32_t pid{0};
    std::string name;
    double cpu_usage{0.0};
    uint64_t memory_bytes{0};
    uint64_t start_time{0};
};

struct CommandResult {
    bool success{false};
    std::string output;
    std::string error;
    int exit_code{0};
    std::chrono::milliseconds execution_time{0};
};

// =============================================================================
// Plugin Interface (Abstract Base)
// =============================================================================

class IPlugin {
public:
    virtual ~IPlugin() = default;
    
    // Metadata
    [[nodiscard]] virtual std::string name() const = 0;
    [[nodiscard]] virtual std::string version() const = 0;
    [[nodiscard]] virtual std::string description() const = 0;
    [[nodiscard]] virtual std::vector<std::string> supported_commands() const = 0;
    
    // Lifecycle
    [[nodiscard]] virtual bool initialize() = 0;
    virtual void shutdown() = 0;
    [[nodiscard]] virtual bool is_healthy() const = 0;
    
    // Command execution
    [[nodiscard]] virtual PluginResult<CommandResult> execute_command(
        std::string_view command, 
        std::span<const std::byte> params = {}) = 0;
    
    // Type-safe convenience methods
    [[nodiscard]] virtual PluginResult<SystemInfo> get_system_info();
    [[nodiscard]] virtual PluginResult<std::vector<ProcessInfo>> get_processes();
    [[nodiscard]] virtual PluginResult<std::string> read_file(std::string_view path);
    
    // JSON API for dynamic commands
    [[nodiscard]] virtual std::string execute_json(std::string_view json_request);
};

// Plugin factory function type
using PluginFactory = std::unique_ptr<IPlugin>(*)();

// =============================================================================
// Plugin Metadata (from shared library)
// =============================================================================

struct PluginMetadata {
    std::string name;
    std::string version;
    std::string description;
    std::string author;
    std::string api_version;  // "2.0" for Boost.DLL plugins
    std::vector<std::string> supported_platforms;
    std::vector<std::string> dependencies;
};

// Exported C++ functions that plugins must provide
extern "C" {
    // Factory function - creates plugin instance
    MSP_PLUGIN_EXPORT std::unique_ptr<IPlugin> msp_create_plugin();
    
    // Metadata query
    MSP_PLUGIN_EXPORT const char* msp_get_plugin_metadata();
    
    // API version check
    MSP_PLUGIN_EXPORT const char* msp_get_api_version();  // Returns "2.0"
}

// =============================================================================
// Boost.DLL Plugin Manager
// =============================================================================

class BoostPluginManager {
public:
    struct LoadedPlugin {
        std::string id;
        std::string path;
        PluginMetadata metadata;
        std::unique_ptr<IPlugin> instance;
        dll::shared_library library;
        std::chrono::steady_clock::time_point loaded_at;
        uint64_t commands_executed{0};
        uint64_t errors_count{0};
    };
    
    BoostPluginManager();
    ~BoostPluginManager();
    
    // Non-copyable, movable
    BoostPluginManager(const BoostPluginManager&) = delete;
    BoostPluginManager& operator=(const BoostPluginManager&) = delete;
    BoostPluginManager(BoostPluginManager&&) noexcept;
    BoostPluginManager& operator=(BoostPluginManager&&) noexcept;
    
    // Plugin loading
    [[nodiscard]] PluginResult<std::string> load_plugin(const fs::path& path);
    [[nodiscard]] bool unload_plugin(std::string_view plugin_id);
    void unload_all();
    
    // Discovery
    [[nodiscard]] std::vector<std::string> discover_plugins(const fs::path& directory) const;
    void load_all_from_directory(const fs::path& directory);
    
    // Query
    [[nodiscard]] std::vector<std::string> list_loaded_plugins() const;
    [[nodiscard]] LoadedPlugin* get_plugin(std::string_view plugin_id);
    [[nodiscard]] const LoadedPlugin* get_plugin(std::string_view plugin_id) const;
    [[nodiscard]] size_t loaded_count() const;
    
    // Command execution with routing
    [[nodiscard]] PluginResult<CommandResult> execute_command(
        std::string_view plugin_name,
        std::string_view command,
        std::span<const std::byte> params = {});
    
    // Auto-routing: finds plugin that supports command
    [[nodiscard]] PluginResult<CommandResult> execute_command_auto(
        std::string_view command,
        std::span<const std::byte> params = {});
    
    // JSON API
    [[nodiscard]] std::string execute_json(std::string_view plugin_name, std::string_view json_request);
    
    // Health checks
    [[nodiscard]] std::vector<std::pair<std::string, bool>> health_check_all() const;
    
    // Metrics
    struct Metrics {
        uint64_t total_commands{0};
        uint64_t total_errors{0};
        size_t active_plugins{0};
        std::chrono::milliseconds total_uptime{0};
    };
    [[nodiscard]] Metrics get_metrics() const;
    
private:
    class Impl;
    std::unique_ptr<Impl> pImpl;  // PIMPL idiom for ABI stability
    
    [[nodiscard]] PluginMetadata parse_metadata(const char* json_str);
    [[nodiscard]] bool check_api_compatibility(const char* api_version);
};

// =============================================================================
// C API for Rust FFI (extern "C" wrapper)
// =============================================================================

extern "C" {
    // Manager lifecycle
    MSP_PLUGIN_EXPORT BoostPluginManager* msp_manager_create();
    MSP_PLUGIN_EXPORT void msp_manager_destroy(BoostPluginManager* manager);
    
    // Plugin loading
    MSP_PLUGIN_EXPORT bool msp_manager_load_plugin(
        BoostPluginManager* manager, 
        const char* path,
        char* error_buffer,
        size_t error_buffer_size);
    
    MSP_PLUGIN_EXPORT bool msp_manager_unload_plugin(
        BoostPluginManager* manager,
        const char* plugin_id);
    
    // Command execution (JSON in/out)
    MSP_PLUGIN_EXPORT char* msp_manager_execute_json(
        BoostPluginManager* manager,
        const char* plugin_name,
        const char* json_request);
    
    // Auto-routing execution
    MSP_PLUGIN_EXPORT char* msp_manager_execute_json_auto(
        BoostPluginManager* manager,
        const char* json_request);
    
    // List plugins (returns JSON array)
    MSP_PLUGIN_EXPORT char* msp_manager_list_plugins(BoostPluginManager* manager);
    
    // Free memory returned by C API
    MSP_PLUGIN_EXPORT void msp_free_string(char* str);
    
    // Last error info
    MSP_PLUGIN_EXPORT bool msp_manager_get_last_error(
        char* buffer,
        size_t buffer_size);
}

// =============================================================================
// Helper Macros for Plugin Implementation
// =============================================================================

#define MSP_DEFINE_PLUGIN(PluginClass) \
    extern "C" { \
        MSP_PLUGIN_EXPORT std::unique_ptr<msp::plugins::IPlugin> msp_create_plugin() { \
            return std::make_unique<PluginClass>(); \
        } \
        MSP_PLUGIN_EXPORT const char* msp_get_api_version() { \
            return "2.0"; \
        } \
        MSP_PLUGIN_EXPORT const char* msp_get_plugin_metadata() { \
            return PluginClass::get_metadata_json(); \
        } \
    }

} // namespace msp::plugins
