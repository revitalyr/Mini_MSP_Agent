//! Custom Plugin Template for Mini MSP Agent
//!
//! This plugin demonstrates how to create custom functionality:
//! - Custom metrics collection
//! - Custom command handlers
//! - Configuration support
//! - State management
//!
//! Build as shared library and place in plugins/ directory.

#include <cstring>
#include <cstdlib>
#include <ctime>
#include <cstdio>

#ifdef _WIN32
    #include <windows.h>
    #define PLUGIN_EXPORT __declspec(dllexport)
#else
    #define PLUGIN_EXPORT __attribute__((visibility("default")))
#endif

// Plugin metadata
static const char* PLUGIN_NAME = "custom_plugin";
static const char* PLUGIN_VERSION = "1.0.0";
static const char* PLUGIN_DESCRIPTION = "Example custom plugin for extensible functionality";

// Plugin state
static struct {
    bool initialized;
    int command_count;
    char last_command[256];
    time_t start_time;
} plugin_state = {false, 0, "", 0};

// Custom metrics structure
struct CustomMetrics {
    int commands_executed;
    int errors_encountered;
    double uptime_seconds;
    char status[64];
};

// Custom command result
struct CommandResult {
    bool success;
    char output[1024];
    char error[256];
};

// Plugin interface structure (simplified)
struct PluginInterface {
    const char* (*get_plugin_info)();
    bool (*init)();
    void (*cleanup)();
    bool (*execute_custom_command)(const char* command, CommandResult* result);
    bool (*get_custom_metrics)(CustomMetrics* metrics);
    bool (*set_config)(const char* key, const char* value);
};

// Initialize plugin
static bool custom_init() {
    if (plugin_state.initialized) {
        return false;
    }
    
    plugin_state.initialized = true;
    plugin_state.command_count = 0;
    plugin_state.start_time = time(nullptr);
    plugin_state.last_command[0] = '\0';
    
    return true;
}

// Cleanup plugin
static void custom_cleanup() {
    plugin_state.initialized = false;
    plugin_state.command_count = 0;
}

// Execute custom command
static bool execute_custom_command(const char* command, CommandResult* result) {
    if (!plugin_state.initialized || !command || !result) {
        return false;
    }
    
    // Store last command
    strncpy_s(plugin_state.last_command, sizeof(plugin_state.last_command), command, sizeof(plugin_state.last_command) - 1);
    plugin_state.last_command[sizeof(plugin_state.last_command) - 1] = '\0';
    plugin_state.command_count++;
    
    // Parse command
    if (strncmp(command, "echo ", 5) == 0) {
        // Echo command - return the rest of the string
        strncpy_s(result->output, sizeof(result->output), command + 5, sizeof(result->output) - 1);
        result->output[sizeof(result->output) - 1] = '\0';
        result->success = true;
        result->error[0] = '\0';
    }
    else if (strcmp(command, "status") == 0) {
        // Status command
        snprintf(result->output, sizeof(result->output),
            "Plugin: %s v%s\nCommands executed: %d\nUptime: %ld seconds",
            PLUGIN_NAME, PLUGIN_VERSION, plugin_state.command_count,
            (long)(time(nullptr) - plugin_state.start_time));
        result->success = true;
        result->error[0] = '\0';
    }
    else if (strcmp(command, "ping") == 0) {
        strncpy_s(result->output, sizeof(result->output), "pong", sizeof(result->output) - 1);
        result->success = true;
        result->error[0] = '\0';
    }
    else if (strncmp(command, "config ", 7) == 0) {
        // Config command placeholder
        strncpy_s(result->output, sizeof(result->output), "Configuration updated", sizeof(result->output) - 1);
        result->success = true;
        result->error[0] = '\0';
    }
    else {
        snprintf(result->error, sizeof(result->error), 
            "Unknown command: %s", command);
        result->success = false;
        result->output[0] = '\0';
        return false;
    }
    
    return true;
}

// Get custom metrics
static bool get_custom_metrics(CustomMetrics* metrics) {
    if (!plugin_state.initialized || !metrics) {
        return false;
    }
    
    metrics->commands_executed = plugin_state.command_count;
    metrics->errors_encountered = 0; // Placeholder
    metrics->uptime_seconds = difftime(time(nullptr), plugin_state.start_time);
    strncpy_s(metrics->status, sizeof(metrics->status), 
            plugin_state.initialized ? "running" : "stopped",
            sizeof(metrics->status) - 1);
    
    return true;
}

// Set configuration
static bool set_config(const char* key, const char* value) {
    // Placeholder for configuration storage
    // In production, store in file or registry
    (void)key;
    (void)value;
    return true;
}

// Exported functions
extern "C" {
    PLUGIN_EXPORT const char* get_plugin_name() {
        return PLUGIN_NAME;
    }
    
    PLUGIN_EXPORT const char* get_plugin_version() {
        return PLUGIN_VERSION;
    }
    
    PLUGIN_EXPORT const char* get_plugin_description() {
        return PLUGIN_DESCRIPTION;
    }
    
    PLUGIN_EXPORT const char* get_plugin_info() {
        static char info[256];
        snprintf(info, sizeof(info), "%s:%s:%s",
                 PLUGIN_NAME, PLUGIN_VERSION, PLUGIN_DESCRIPTION);
        return info;
    }
    
    PLUGIN_EXPORT bool plugin_initialize() {
        return custom_init();
    }
    
    PLUGIN_EXPORT void plugin_cleanup() {
        custom_cleanup();
    }
    
    PLUGIN_EXPORT bool plugin_execute_command(const char* command, char* output, size_t output_size) {
        CommandResult result;
        bool success = execute_custom_command(command, &result);
        
        if (success) {
            strncpy_s(output, output_size, result.output, output_size - 1);
            output[output_size - 1] = '\0';
        } else {
            strncpy_s(output, output_size, result.error, output_size - 1);
            output[output_size - 1] = '\0';
        }
        
        return success;
    }
    
    PLUGIN_EXPORT bool plugin_get_metrics(char* metrics_json, size_t size) {
        CustomMetrics metrics;
        if (!get_custom_metrics(&metrics)) {
            return false;
        }
        
        snprintf(metrics_json, size,
            "{\"commands_executed\":%d,\"errors\":%d,\"uptime\":%.0f,\"status\":\"%s\"}",
            metrics.commands_executed,
            metrics.errors_encountered,
            metrics.uptime_seconds,
            metrics.status);
        
        return true;
    }
    
    // Full interface getter for advanced usage
    PLUGIN_EXPORT PluginInterface* get_custom_plugin_interface() {
        static PluginInterface iface;
        static bool initialized = false;
        
        if (!initialized) {
            iface.get_plugin_info = get_plugin_info;
            iface.init = custom_init;
            iface.cleanup = custom_cleanup;
            iface.execute_custom_command = execute_custom_command;
            iface.get_custom_metrics = get_custom_metrics;
            iface.set_config = set_config;
            initialized = true;
        }
        
        return &iface;
    }
}
