/**
 * @file c_api_wrapper.cpp
 * @brief C API wrapper for Rust FFI
 * 
 * Provides C-compatible interface for the Boost.DLL Plugin Manager
 * so it can be called from Rust code.
 */

#include "boost_plugin_api.hpp"
#include <cstring>
#include <cstdlib>
#include <mutex>

using namespace msp::plugins;

// =============================================================================
// Internal State
// =============================================================================

static thread_local char last_error_buffer[1024] = {0};

static void set_last_error(const std::string& msg) {
    std::strncpy(last_error_buffer, msg.c_str(), sizeof(last_error_buffer) - 1);
    last_error_buffer[sizeof(last_error_buffer) - 1] = '\0';
}

static void clear_last_error() {
    last_error_buffer[0] = '\0';
}

// =============================================================================
// Manager Lifecycle
// =============================================================================

extern "C" {

MSP_PLUGIN_EXPORT BoostPluginManager* msp_manager_create() {
    try {
        clear_last_error();
        return new BoostPluginManager();
    }
    catch (const std::exception& e) {
        set_last_error(std::format("Failed to create manager: {}", e.what()));
        return nullptr;
    }
}

MSP_PLUGIN_EXPORT void msp_manager_destroy(BoostPluginManager* manager) {
    if (manager) {
        delete manager;
    }
}

// =============================================================================
// Plugin Loading
// =============================================================================

MSP_PLUGIN_EXPORT bool msp_manager_load_plugin(
    BoostPluginManager* manager,
    const char* path,
    char* error_buffer,
    size_t error_buffer_size) {
    
    if (!manager || !path) {
        if (error_buffer && error_buffer_size > 0) {
            std::strncpy(error_buffer, "Invalid arguments", error_buffer_size - 1);
            error_buffer[error_buffer_size - 1] = '\0';
        }
        return false;
    }
    
    try {
        clear_last_error();
        auto result = manager->load_plugin(path);
        
        if (result) {
            return true;
        }
        else {
            const auto& err = result.error();
            std::string msg = std::format("{}: {}", 
                static_cast<int>(err.code), err.message);
            set_last_error(msg);
            
            if (error_buffer && error_buffer_size > 0) {
                std::strncpy(error_buffer, msg.c_str(), error_buffer_size - 1);
                error_buffer[error_buffer_size - 1] = '\0';
            }
            return false;
        }
    }
    catch (const std::exception& e) {
        set_last_error(e.what());
        if (error_buffer && error_buffer_size > 0) {
            std::strncpy(error_buffer, e.what(), error_buffer_size - 1);
            error_buffer[error_buffer_size - 1] = '\0';
        }
        return false;
    }
}

MSP_PLUGIN_EXPORT bool msp_manager_unload_plugin(
    BoostPluginManager* manager,
    const char* plugin_id) {
    
    if (!manager || !plugin_id) return false;
    
    try {
        clear_last_error();
        return manager->unload_plugin(plugin_id);
    }
    catch (const std::exception& e) {
        set_last_error(e.what());
        return false;
    }
}

MSP_PLUGIN_EXPORT void msp_manager_load_all_from_directory(
    BoostPluginManager* manager,
    const char* directory) {
    
    if (!manager || !directory) return;
    
    try {
        clear_last_error();
        manager->load_all_from_directory(directory);
    }
    catch (const std::exception& e) {
        set_last_error(e.what());
    }
}

// =============================================================================
// Command Execution (JSON)
// =============================================================================

MSP_PLUGIN_EXPORT char* msp_manager_execute_json(
    BoostPluginManager* manager,
    const char* plugin_name,
    const char* json_request) {
    
    if (!manager || !json_request) {
        return strdup(R"({"success":false,"error":"Invalid arguments"})");
    }
    
    try {
        clear_last_error();
        
        std::string_view plugin = plugin_name ? plugin_name : "";
        std::string result;
        
        if (!plugin.empty()) {
            result = manager->execute_json(plugin, json_request);
        }
        else {
            // Auto-route to first plugin that supports the command
            auto json = nlohmann::json::parse(json_request, nullptr, false);
            if (json.contains("command")) {
                std::string cmd = json["command"];
                auto cmd_result = manager->execute_command_auto(cmd, {});
                if (cmd_result) {
                    const auto& r = cmd_result.value();
                    result = std::format(R"({{"success":{},"output":{},"error":"{}"}})",
                        r.success ? "true" : "false",
                        r.success ? r.output : "null",
                        r.error);
                }
                else {
                    result = std::format(R"({{"success":false,"error":"{}"}})",
                        cmd_result.error().message);
                }
            }
            else {
                result = R"({"success":false,"error":"Missing command in request"})";
            }
        }
        
        return strdup(result.c_str());
    }
    catch (const std::exception& e) {
        set_last_error(e.what());
        return strdup(std::format(R"({{"success":false,"error":"Exception: {}"}})", e.what()).c_str());
    }
}

MSP_PLUGIN_EXPORT char* msp_manager_execute_json_auto(
    BoostPluginManager* manager,
    const char* json_request) {
    return msp_manager_execute_json(manager, nullptr, json_request);
}

// =============================================================================
// Plugin Queries
// =============================================================================

MSP_PLUGIN_EXPORT char* msp_manager_list_plugins(BoostPluginManager* manager) {
    if (!manager) {
        return strdup("[]");
    }
    
    try {
        auto plugins = manager->list_loaded_plugins();
        
        nlohmann::json array = nlohmann::json::array();
        for (const auto& id : plugins) {
            auto* plugin = manager->get_plugin(id);
            if (plugin) {
                nlohmann::json info;
                info["id"] = id;
                info["name"] = plugin->instance->name();
                info["version"] = plugin->instance->version();
                info["description"] = plugin->instance->description();
                info["healthy"] = plugin->instance->is_healthy();
                info["commands_executed"] = plugin->commands_executed;
                
                auto supported = plugin->instance->supported_commands();
                info["supported_commands"] = supported;
                
                array.push_back(info);
            }
        }
        
        return strdup(array.dump().c_str());
    }
    catch (const std::exception& e) {
        return strdup(std::format(R"([{{"error":"{}"}}])", e.what()).c_str());
    }
}

MSP_PLUGIN_EXPORT size_t msp_manager_get_plugin_count(BoostPluginManager* manager) {
    if (!manager) return 0;
    return manager->loaded_count();
}

// =============================================================================
// Health & Metrics
// =============================================================================

MSP_PLUGIN_EXPORT char* msp_manager_health_check(BoostPluginManager* manager) {
    if (!manager) {
        return strdup(R"({"error":"Invalid manager"})");
    }
    
    try {
        auto checks = manager->health_check_all();
        nlohmann::json obj;
        for (const auto& [id, healthy] : checks) {
            obj[id] = healthy;
        }
        return strdup(obj.dump().c_str());
    }
    catch (const std::exception& e) {
        return strdup(std::format(R"({{"error":"{}"}})", e.what()).c_str());
    }
}

MSP_PLUGIN_EXPORT char* msp_manager_get_metrics(BoostPluginManager* manager) {
    if (!manager) {
        return strdup(R"({"error":"Invalid manager"})");
    }
    
    try {
        auto m = manager->get_metrics();
        nlohmann::json obj;
        obj["total_commands"] = m.total_commands;
        obj["total_errors"] = m.total_errors;
        obj["active_plugins"] = m.active_plugins;
        obj["uptime_ms"] = m.total_uptime.count();
        return strdup(obj.dump().c_str());
    }
    catch (const std::exception& e) {
        return strdup(std::format(R"({{"error":"{}"}})", e.what()).c_str());
    }
}

// =============================================================================
// Memory Management
// =============================================================================

MSP_PLUGIN_EXPORT void msp_free_string(char* str) {
    if (str) {
        free(str);
    }
}

// =============================================================================
// Error Handling
// =============================================================================

MSP_PLUGIN_EXPORT bool msp_manager_get_last_error(
    char* buffer,
    size_t buffer_size) {
    
    if (!buffer || buffer_size == 0) {
        return false;
    }
    
    if (last_error_buffer[0] == '\0') {
        buffer[0] = '\0';
        return false;  // No error
    }
    
    std::strncpy(buffer, last_error_buffer, buffer_size - 1);
    buffer[buffer_size - 1] = '\0';
    return true;
}

MSP_PLUGIN_EXPORT bool msp_clear_last_error() {
    bool had_error = (last_error_buffer[0] != '\0');
    clear_last_error();
    return had_error;
}

} // extern "C"
