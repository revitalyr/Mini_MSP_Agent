//! Custom Plugin Template for Mini MSP Agent (Windows)
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
#include <windows.h>

#define PLUGIN_EXPORT __declspec(dllexport)

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

// Function declarations (implementations follow)
extern "C" {
    PLUGIN_EXPORT const char* get_plugin_name();
    PLUGIN_EXPORT const char* get_plugin_version();
    PLUGIN_EXPORT const char* get_plugin_description();
    PLUGIN_EXPORT bool initialize_plugin();
    PLUGIN_EXPORT void cleanup_plugin();
    PLUGIN_EXPORT bool execute_custom_command(const char* command, char* output, size_t output_size);
    PLUGIN_EXPORT bool get_custom_metrics(void* metrics_buffer, size_t buffer_size);
}

// Plugin implementation
const char* get_plugin_name() {
    return PLUGIN_NAME;
}

const char* get_plugin_version() {
    return PLUGIN_VERSION;
}

const char* get_plugin_description() {
    return PLUGIN_DESCRIPTION;
}

bool initialize_plugin() {
    if (plugin_state.initialized) {
        return true;
    }

    plugin_state.start_time = time(NULL);
    plugin_state.initialized = true;
    return true;
}

void cleanup_plugin() {
    plugin_state.initialized = false;
}

bool execute_custom_command(const char* command, char* output, size_t output_size) {
    if (!plugin_state.initialized || !command) {
        return false;
    }

    // Store command
    strncpy_s(plugin_state.last_command, sizeof(plugin_state.last_command), command, _TRUNCATE);
    plugin_state.command_count++;

    // Simple command processing
    if (strcmp(command, "hello") == 0) {
        strncpy_s(output, output_size, "Hello from custom plugin!", _TRUNCATE);
        return true;
    } else if (strcmp(command, "status") == 0) {
        snprintf(output, output_size, "Plugin running for %ld seconds, %d commands executed",
                (long)(time(NULL) - plugin_state.start_time), plugin_state.command_count);
        return true;
    }

    // Unknown command
    strncpy_s(output, output_size, "Unknown command", _TRUNCATE);
    return false;
}

bool get_custom_metrics(void* metrics_buffer, size_t buffer_size) {
    if (!plugin_state.initialized || buffer_size < sizeof(CustomMetrics)) {
        return false;
    }

    CustomMetrics* metrics = static_cast<CustomMetrics*>(metrics_buffer);
    metrics->commands_executed = plugin_state.command_count;
    metrics->errors_encountered = 0; // Not tracking errors in this example
    metrics->uptime_seconds = difftime(time(NULL), plugin_state.start_time);
    strncpy_s(metrics->status, sizeof(metrics->status), "active", _TRUNCATE);

    return true;
}