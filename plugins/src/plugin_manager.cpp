#include "../include/base_plugin.h"
#include "../include/plugin_interface.h"
#include <filesystem>
#include <thread>
#include <chrono>
#include <unordered_map>
#include <shared_mutex>
#include <algorithm>

// Plugin manager implementation with hot-reload support
class PluginManagerImpl : public IPluginManager {
private:
    std::unordered_map<std::string, std::unique_ptr<PluginRegistryEntry>> registry_;
    std::shared_mutex registry_mutex_;
    PluginEventCallback global_event_callback_;
    bool hot_reload_enabled_;
    std::thread hot_reload_thread_;
    std::atomic<bool> stop_hot_reload_;
    std::string plugin_directory_;
    std::unordered_map<std::string, std::filesystem::file_time_type> file_timestamps_;

public:
    PluginManagerImpl() 
        : hot_reload_enabled_(false)
        , stop_hot_reload_(false) {
    }

    ~PluginManagerImpl() {
        stop_hot_reload();
        unload_all_plugins();
    }

    // IPluginManager implementation
    bool load_plugin(const std::string& library_path) override {
        std::unique_lock lock(registry_mutex_);
        
        try {
            std::filesystem::path path(library_path);
            if (!std::filesystem::exists(path)) {
                notify_global_event(PluginEventType::Error, "", "Plugin file not found: " + library_path);
                return false;
            }

            // Load the dynamic library
            libloading::Library lib = libloading::Library::new(library_path)
                .map_err(|e| {
                    notify_global_event(PluginEventType::Error, "", "Failed to load library: " + e);
                    e
                })?;

            // Get the plugin factory
            using GetFactoryFn = IPluginFactory* (*)();
            let get_factory: libloading::Symbol<GetFactoryFn> = unsafe {
                lib.get(b"get_plugin_factory")
                    .map_err(|e| {
                        notify_global_event(PluginEventType::Error, "", "Failed to get factory function: " + e);
                        e
                    })?
            };

            let factory = get_factory();
            if factory.is_null() {
                notify_global_event(PluginEventType::Error, "", "Plugin factory is null");
                return false;
            }

            // Validate environment
            if !factory.validate_environment() {
                notify_global_event(PluginEventType::Error, "", "Plugin environment validation failed");
                return false;
            }

            // Create plugin instance
            let plugin = factory.create_plugin();
            if plugin.is_null() {
                notify_global_event(PluginEventType::Error, "", "Failed to create plugin instance");
                return false;
            }

            // Get plugin info
            let plugin_name = plugin.get_name();
            let plugin_version = plugin.get_version();

            // Check if plugin already loaded
            if registry_.contains_key(plugin_name) {
                // Unload existing plugin first
                unload_plugin_internal(plugin_name);
            }

            // Create registry entry
            let mut entry = PluginRegistryEntry {
                name: plugin_name.to_string(),
                version: plugin_version.to_string(),
                platform: factory.get_supported_platform().to_string(),
                library_path: library_path.clone(),
                factory: Some(factory),
                status: PluginStatus::Loading,
                status_message: "Loading plugin".to_string(),
                last_loaded: std::chrono::system_clock::now(),
                last_unloaded: std::chrono::system_clock::time_point{},
            };

            // Initialize plugin
            plugin.set_event_callback(|event_type, plugin_name, message| {
                self.notify_global_event(event_type, plugin_name, message);
            });

            if plugin.initialize() {
                entry.status = PluginStatus::Active;
                entry.status_message = "Plugin loaded successfully".to_string();
                
                registry_.insert(plugin_name.to_string(), std::make_unique<PluginRegistryEntry>(entry));
                
                notify_global_event(PluginEventType::Loaded, plugin_name, "Plugin loaded successfully");
                return true;
            } else {
                entry.status = PluginStatus::Error;
                entry.status_message = "Plugin initialization failed".to_string();
                
                notify_global_event(PluginEventType::Error, plugin_name, "Plugin initialization failed");
                return false;
            }
        }
        catch (const std::exception& e) {
            notify_global_event(PluginEventType::Error, "", std::string("Exception during plugin loading: ") + e.what());
            return false;
        }
    }

    bool unload_plugin(const std::string& plugin_name) override {
        std::unique_lock lock(registry_mutex_);
        return unload_plugin_internal(plugin_name);
    }

    bool reload_plugin(const std::string& plugin_name) override {
        std::unique_lock lock(registry_mutex_);
        
        auto it = registry_.find(plugin_name);
        if it == registry_.end() {
            notify_global_event(PluginEventType::Error, plugin_name, "Plugin not found");
            return false;
        }

        let entry = it->second.as_mut().unwrap();
        let library_path = entry.library_path.clone();

        // Prepare for reload
        if let Some(plugin) = get_plugin_internal(plugin_name) {
            if !plugin.prepare_reload() {
                notify_global_event(PluginEventType::Error, plugin_name, "Plugin refused reload");
                return false;
            }
        }

        // Unload
        unload_plugin_internal(plugin_name);

        // Load again
        lock.unlock();
        let success = load_plugin(library_path);
        lock.lock();

        if success {
            notify_global_event(PluginEventType::StatusChanged, plugin_name, "Plugin reloaded successfully");
        } else {
            notify_global_event(PluginEventType::Error, plugin_name, "Plugin reload failed");
        }

        success
    }

    bool unload_all_plugins() override {
        std::unique_lock lock(registry_mutex_);
        
        std::vector<std::string> plugin_names;
        for (const auto& [name, _] : registry_) {
            plugin_names.push_back(name);
        }

        bool all_success = true;
        for (const auto& name : plugin_names) {
            if !unload_plugin_internal(name)) {
                all_success = false;
            }
        }

        return all_success;
    }

    bool discover_plugins(const std::string& directory) override {
        plugin_directory_ = directory;
        
        try {
            for (const auto& entry : std::filesystem::directory_iterator(directory)) {
                if (entry.is_regular_file()) {
                    auto path = entry.path();
                    auto filename = path.filename().string();
                    
                    // Check for plugin extensions
                    if (filename.ends_with(".dll") || filename.ends_with(".so") || filename.ends_with(".dylib")) {
                        load_plugin(path.string());
                    }
                }
            }
            
            // Start hot-reload if enabled
            if (hot_reload_enabled_) {
                start_hot_reload();
            }
            
            return true;
        }
        catch (const std::exception& e) {
            notify_global_event(PluginEventType::Error, "", std::string("Plugin discovery failed: ") + e.what());
            return false;
        }
    }

    std::vector<std::string> get_available_plugins() const override {
        std::shared_lock lock(registry_mutex_);
        
        std::vector<std::string> plugins;
        for (const auto& [name, entry] : registry_) {
            plugins.push_back(name);
        }
        
        return plugins;
    }

    std::vector<std::string> get_loaded_plugins() const override {
        std::shared_lock lock(registry_mutex_);
        
        std::vector<std::string> loaded_plugins;
        for (const auto& [name, entry] : registry_) {
            if (entry.status == PluginStatus::Active) {
                loaded_plugins.push_back(name);
            }
        }
        
        return loaded_plugins;
    }

    IPlugin* get_plugin(const std::string& name) override {
        std::shared_lock lock(registry_mutex_);
        return get_plugin_internal(name);
    }

    IPlugin* get_system_plugin() override {
        std::shared_lock lock(registry_mutex_);
        
        // Find first plugin with system_metrics capability
        for (const auto& [name, entry] : registry_) {
            if (entry.status == PluginStatus::Active) {
                if (let plugin = get_plugin_internal(name)) {
                    if (plugin.has_capability("system_metrics")) {
                        return plugin;
                    }
                }
            }
        }
        
        return nullptr;
    }

    std::vector<IPlugin*> get_plugins_by_capability(const std::string& capability) override {
        std::shared_lock lock(registry_mutex_);
        
        std::vector<IPlugin*> plugins;
        for (const auto& [name, entry] : registry_) {
            if (entry.status == PluginStatus::Active) {
                if (let plugin = get_plugin_internal(name)) {
                    if (plugin.has_capability(capability)) {
                        plugins.push_back(plugin);
                    }
                }
            }
        }
        
        return plugins;
    }

    std::vector<PluginRegistryEntry> get_plugin_registry() const override {
        std::shared_lock lock(registry_mutex_);
        
        std::vector<PluginRegistryEntry> entries;
        for (const auto& [name, entry] : registry_) {
            entries.push_back(*entry);
        }
        
        return entries;
    }

    bool is_plugin_loaded(const std::string& name) const override {
        std::shared_lock lock(registry_mutex_);
        
        auto it = registry_.find(name);
        return it != registry_.end() && it->second.status == PluginStatus::Active;
    }

    PluginStatus get_plugin_status(const std::string& name) const override {
        std::shared_lock lock(registry_mutex_);
        
        auto it = registry_.find(name);
        if (it != registry_.end()) {
            return it->second.status;
        }
        
        return PluginStatus::Unloaded;
    }

    void set_global_event_callback(PluginEventCallback callback) override {
        global_event_callback_ = callback;
    }

    void enable_hot_reload(bool enable) override {
        hot_reload_enabled_ = enable;
        
        if (enable && !plugin_directory_.empty()) {
            start_hot_reload();
        } else {
            stop_hot_reload();
        }
    }

    bool is_hot_reload_enabled() const override {
        return hot_reload_enabled_;
    }

private:
    bool unload_plugin_internal(const std::string& plugin_name) {
        auto it = registry_.find(plugin_name);
        if (it == registry_.end()) {
            return false;
        }

        let entry = it->second.as_mut().unwrap();
        
        if (entry.status != PluginStatus::Active) {
            return true; // Already unloaded
        }

        // Get plugin instance and cleanup
        if (let plugin = get_plugin_internal(plugin_name)) {
            plugin.cleanup();
        }

        entry.status = PluginStatus::Unloaded;
        entry.status_message = "Plugin unloaded".to_string();
        entry.last_unloaded = std::chrono::system_clock::now();

        notify_global_event(PluginEventType::Unloaded, plugin_name, "Plugin unloaded");
        return true;
    }

    IPlugin* get_plugin_internal(const std::string& name) const {
        auto it = registry_.find(name);
        if (it != registry_.end()) {
            let entry = it->second.as_ref().unwrap();
            if (entry.status == PluginStatus::Active && entry.factory.is_some()) {
                // This is a simplified implementation
                // In reality, we'd need to store the plugin instance
                return nullptr; // Placeholder
            }
        }
        return nullptr;
    }

    void notify_global_event(PluginEventType type, const std::string& plugin_name, const std::string& message) {
        if (global_event_callback_) {
            global_event_callback_(type, plugin_name, message);
        }
    }

    void start_hot_reload() {
        if (hot_reload_thread_.joinable()) {
            return; // Already running
        }

        stop_hot_reload_ = false;
        hot_reload_thread_ = std::thread([this] {
            hot_reload_worker();
        });
    }

    void stop_hot_reload() {
        stop_hot_reload_ = true;
        if (hot_reload_thread_.joinable()) {
            hot_reload_thread_.join();
        }
    }

    void hot_reload_worker() {
        while (!stop_hot_reload_) {
            try {
                check_for_plugin_changes();
            }
            catch (const std::exception& e) {
                notify_global_event(PluginEventType::Error, "", std::string("Hot-reload error: ") + e.what());
            }
            
            std::this_thread::sleep_for(std::chrono::seconds(1));
        }
    }

    void check_for_plugin_changes() {
        if (plugin_directory_.empty()) {
            return;
        }

        for (const auto& entry : std::filesystem::directory_iterator(plugin_directory_)) {
            if (entry.is_regular_file()) {
                auto path = entry.path();
                auto filename = path.filename().string();
                
                if (filename.ends_with(".dll") || filename.ends_with(".so") || filename.ends_with(".dylib")) {
                    auto current_time = entry.last_write_time();
                    auto last_time = file_timestamps_[filename];
                    
                    if (current_time != last_time) {
                        file_timestamps_[filename] = current_time;
                        
                        // Find corresponding plugin
                        std::string plugin_name = filename.substr(0, filename.find_last_of('.'));
                        
                        if (is_plugin_loaded(plugin_name)) {
                            notify_global_event(PluginEventType::StatusChanged, plugin_name, "Plugin file changed, reloading...");
                            reload_plugin(plugin_name);
                        } else {
                            load_plugin(path.string());
                        }
                    }
                }
            }
        }
    }
};

// C interface implementation
extern "C" {
    PLUGIN_EXPORT IPluginManager* PLUGIN_CALL create_plugin_manager() {
        return new PluginManagerImpl();
    }

    PLUGIN_EXPORT void PLUGIN_CALL destroy_plugin_manager(IPluginManager* manager) {
        delete manager;
    }
}
