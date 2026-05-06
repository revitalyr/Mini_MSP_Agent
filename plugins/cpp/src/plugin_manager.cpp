/**
 * @file plugin_manager.cpp
 * @brief Boost.DLL Plugin Manager Implementation
 */

#include "boost_plugin_api.hpp"
#include <mutex>
#include <shared_mutex>
#include <algorithm>
#include <fstream>
#include <nlohmann/json.hpp>

namespace msp::plugins {

// PIMPL implementation
class BoostPluginManager::Impl {
public:
    mutable std::shared_mutex mutex;
    std::unordered_map<std::string, std::unique_ptr<LoadedPlugin>> plugins;
    std::unordered_map<std::string, std::string> command_to_plugin_map;
    std::string last_error;
    
    [[nodiscard]] std::optional<std::string> get_plugin_for_command(std::string_view command) {
        std::shared_lock lock(mutex);
        auto it = command_to_plugin_map.find(std::string(command));
        if (it != command_to_plugin_map.end()) {
            return it->second;
        }
        
        // Search through all plugins
        for (const auto& [id, plugin] : plugins) {
            auto supported = plugin->instance->supported_commands();
            if (std::find(supported.begin(), supported.end(), command) != supported.end()) {
                return id;
            }
        }
        return std::nullopt;
    }
    
    void update_command_map(LoadedPlugin* plugin) {
        auto commands = plugin->instance->supported_commands();
        for (const auto& cmd : commands) {
            command_to_plugin_map[cmd] = plugin->id;
        }
    }
};

// =============================================================================
// Manager Lifecycle
// =============================================================================

BoostPluginManager::BoostPluginManager() : pImpl(std::make_unique<Impl>()) {}

BoostPluginManager::~BoostPluginManager() {
    unload_all();
}

BoostPluginManager::BoostPluginManager(BoostPluginManager&&) noexcept = default;
BoostPluginManager& BoostPluginManager::operator=(BoostPluginManager&&) noexcept = default;

// =============================================================================
// Plugin Loading
// =============================================================================

PluginResult<std::string> BoostPluginManager::load_plugin(const fs::path& path) {
    std::unique_lock lock(pImpl->mutex);
    
    try {
        // Check if file exists
        if (!fs::exists(path)) {
            return std::unexpected(PluginErrorInfo{
                PluginError::NotFound, 
                std::format("Plugin file not found: {}", path.string())
            });
        }
        
        // Generate plugin ID from filename
        std::string plugin_id = path.stem().string();
        
        // Check if already loaded
        if (pImpl->plugins.contains(plugin_id)) {
            return std::unexpected(PluginErrorInfo{
                PluginError::AlreadyLoaded,
                std::format("Plugin '{}' is already loaded", plugin_id)
            });
        }
        
        // Load shared library with Boost.DLL
        dll::shared_library lib;
        try {
            lib.load(path, dll::load_mode::rtld_lazy | dll::load_mode::rtld_local);
        }
        catch (const std::exception& e) {
            return std::unexpected(PluginErrorInfo{
                PluginError::LoadFailed,
                std::format("Failed to load library: {}", e.what())
            });
        }
        
        // Check API version
        if (!lib.has("msp_get_api_version")) {
            return std::unexpected(PluginErrorInfo{
                PluginError::SymbolMissing,
                "Missing msp_get_api_version symbol"
            });
        }
        
        auto api_version_fn = lib.get<const char*(*)()>("msp_get_api_version");
        const char* api_version = api_version_fn();
        
        if (!check_api_compatibility(api_version)) {
            return std::unexpected(PluginErrorInfo{
                PluginError::InvalidVersion,
                std::format("API version mismatch: got {}, expected 2.0", api_version)
            });
        }
        
        // Parse metadata
        auto metadata_fn = lib.get<const char*(*)()>("msp_get_plugin_metadata");
        PluginMetadata metadata = parse_metadata(metadata_fn());
        
        // Create factory and instantiate plugin
        auto factory = lib.get<std::unique_ptr<IPlugin>(*)()>("msp_create_plugin");
        auto instance = factory();
        
        if (!instance) {
            return std::unexpected(PluginErrorInfo{
                PluginError::LoadFailed,
                "Factory returned null plugin instance"
            });
        }
        
        // Initialize plugin
        if (!instance->initialize()) {
            return std::unexpected(PluginErrorInfo{
                PluginError::InitFailed,
                "Plugin initialization failed"
            });
        }
        
        // Create loaded plugin record
        auto loaded = std::make_unique<LoadedPlugin>();
        loaded->id = plugin_id;
        loaded->path = path.string();
        loaded->metadata = std::move(metadata);
        loaded->instance = std::move(instance);
        loaded->library = std::move(lib);
        loaded->loaded_at = std::chrono::steady_clock::now();
        
        // Update command map
        auto* raw_ptr = loaded.get();
        pImpl->plugins[plugin_id] = std::move(loaded);
        pImpl->update_command_map(raw_ptr);
        
        return plugin_id;
    }
    catch (const std::exception& e) {
        return std::unexpected(PluginErrorInfo{
            PluginError::LoadFailed,
            std::format("Exception during plugin load: {}", e.what())
        });
    }
}

bool BoostPluginManager::unload_plugin(std::string_view plugin_id) {
    std::unique_lock lock(pImpl->mutex);
    
    auto it = pImpl->plugins.find(std::string(plugin_id));
    if (it == pImpl->plugins.end()) {
        return false;
    }
    
    // Shutdown plugin
    it->second->instance->shutdown();
    
    // Remove from command map
    auto commands = it->second->instance->supported_commands();
    for (const auto& cmd : commands) {
        pImpl->command_to_plugin_map.erase(cmd);
    }
    
    // Library will be unloaded when unique_ptr is destroyed
    pImpl->plugins.erase(it);
    return true;
}

void BoostPluginManager::unload_all() {
    std::unique_lock lock(pImpl->mutex);
    
    for (auto& [id, plugin] : pImpl->plugins) {
        plugin->instance->shutdown();
    }
    
    pImpl->plugins.clear();
    pImpl->command_to_plugin_map.clear();
}

// =============================================================================
// Plugin Discovery
// =============================================================================

std::vector<std::string> BoostPluginManager::discover_plugins(const fs::path& directory) const {
    std::vector<std::string> found;
    
    if (!fs::exists(directory) || !fs::is_directory(directory)) {
        return found;
    }
    
    for (const auto& entry : fs::directory_iterator(directory)) {
        if (!entry.is_regular_file()) continue;
        
        auto ext = entry.path().extension().string();
        
        // Check platform-specific extension
        bool is_plugin = false;
#ifdef _WIN32
        is_plugin = (ext == ".dll");
#elif __APPLE__
        is_plugin = (ext == ".dylib" || ext == ".so");
#else
        is_plugin = (ext == ".so");
#endif
        
        if (is_plugin) {
            found.push_back(entry.path().string());
        }
    }
    
    return found;
}

void BoostPluginManager::load_all_from_directory(const fs::path& directory) {
    auto plugins = discover_plugins(directory);
    for (const auto& path : plugins) {
        load_plugin(path);
    }
}

// =============================================================================
// Command Execution
// =============================================================================

PluginResult<CommandResult> BoostPluginManager::execute_command(
    std::string_view plugin_name,
    std::string_view command,
    std::span<const std::byte> params) {
    
    std::shared_lock lock(pImpl->mutex);
    
    auto it = pImpl->plugins.find(std::string(plugin_name));
    if (it == pImpl->plugins.end()) {
        return std::unexpected(PluginErrorInfo{
            PluginError::NotLoaded,
            std::format("Plugin '{}' not found", plugin_name)
        });
    }
    
    auto& plugin = it->second;
    lock.unlock();  // Unlock during execution
    
    auto start = std::chrono::steady_clock::now();
    auto result = plugin->instance->execute_command(command, params);
    auto end = std::chrono::steady_clock::now();
    
    // Update metrics
    plugin->commands_executed++;
    if (!result || !result->success) {
        plugin->errors_count++;
    }
    
    return result;
}

PluginResult<CommandResult> BoostPluginManager::execute_command_auto(
    std::string_view command,
    std::span<const std::byte> params) {
    
    auto plugin_id = pImpl->get_plugin_for_command(command);
    if (!plugin_id) {
        return std::unexpected(PluginErrorInfo{
            PluginError::CommandNotSupported,
            std::format("No plugin found for command: {}", command)
        });
    }
    
    return execute_command(*plugin_id, command, params);
}

std::string BoostPluginManager::execute_json(
    std::string_view plugin_name,
    std::string_view json_request) {
    
    std::shared_lock lock(pImpl->mutex);
    
    auto it = pImpl->plugins.find(std::string(plugin_name));
    if (it == pImpl->plugins.end()) {
        return std::format(R"({{"success":false,"error":"Plugin '{}' not found"}})", plugin_name);
    }
    
    auto& plugin = it->second;
    lock.unlock();
    
    auto start = std::chrono::steady_clock::now();
    auto result = plugin->instance->execute_json(json_request);
    auto end = std::chrono::steady_clock::now();
    
    // Update metrics
    plugin->commands_executed++;
    
    return result;
}

// =============================================================================
// Queries
// =============================================================================

std::vector<std::string> BoostPluginManager::list_loaded_plugins() const {
    std::shared_lock lock(pImpl->mutex);
    
    std::vector<std::string> ids;
    for (const auto& [id, _] : pImpl->plugins) {
        ids.push_back(id);
    }
    return ids;
}

BoostPluginManager::LoadedPlugin* BoostPluginManager::get_plugin(std::string_view plugin_id) {
    std::shared_lock lock(pImpl->mutex);
    auto it = pImpl->plugins.find(std::string(plugin_id));
    return it != pImpl->plugins.end() ? it->second.get() : nullptr;
}

const BoostPluginManager::LoadedPlugin* BoostPluginManager::get_plugin(std::string_view plugin_id) const {
    std::shared_lock lock(pImpl->mutex);
    auto it = pImpl->plugins.find(std::string(plugin_id));
    return it != pImpl->plugins.end() ? it->second.get() : nullptr;
}

size_t BoostPluginManager::loaded_count() const {
    std::shared_lock lock(pImpl->mutex);
    return pImpl->plugins.size();
}

// =============================================================================
// Health & Metrics
// =============================================================================

std::vector<std::pair<std::string, bool>> BoostPluginManager::health_check_all() const {
    std::shared_lock lock(pImpl->mutex);
    
    std::vector<std::pair<std::string, bool>> results;
    for (const auto& [id, plugin] : pImpl->plugins) {
        results.emplace_back(id, plugin->instance->is_healthy());
    }
    return results;
}

BoostPluginManager::Metrics BoostPluginManager::get_metrics() const {
    std::shared_lock lock(pImpl->mutex);
    
    Metrics m;
    m.active_plugins = pImpl->plugins.size();
    
    for (const auto& [_, plugin] : pImpl->plugins) {
        m.total_commands += plugin->commands_executed;
        m.total_errors += plugin->errors_count;
    }
    
    return m;
}

// =============================================================================
// Helper Methods
// =============================================================================

PluginMetadata BoostPluginManager::parse_metadata(const char* json_str) {
    PluginMetadata meta;
    
    try {
        auto json = nlohmann::json::parse(json_str);
        meta.name = json.value("name", "unknown");
        meta.version = json.value("version", "0.0.0");
        meta.description = json.value("description", "");
        meta.author = json.value("author", "");
        meta.api_version = json.value("api_version", "1.0");
        
        if (json.contains("supported_platforms")) {
            for (const auto& p : json["supported_platforms"]) {
                meta.supported_platforms.push_back(p.get<std::string>());
            }
        }
        
        if (json.contains("dependencies")) {
            for (const auto& d : json["dependencies"]) {
                meta.dependencies.push_back(d.get<std::string>());
            }
        }
    }
    catch (const std::exception&) {
        // Return defaults on parse error
    }
    
    return meta;
}

bool BoostPluginManager::check_api_compatibility(const char* api_version) {
    return std::string_view(api_version) == "2.0";
}

} // namespace msp::plugins
