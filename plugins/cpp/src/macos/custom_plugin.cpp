/**
 * Custom Command Plugin - C++ Implementation for macOS
 * 
 * Provides custom command execution functionality via C FFI
 * Replaces the Rust custom_plugin.rs with native C++ implementation
 */

#define _DARWIN_C_SOURCE
#include <string>
#include <cstring>
#include <chrono>
#include <sstream>
#include <array>
#include <cstdio>
#include <cstdlib>

#define EXPORT extern "C"

// Plugin state
static std::string g_plugin_name = "custom_command";
static std::string g_plugin_version = "1.0.0";
static std::string g_plugin_description = "C++ custom command execution plugin";
static int g_commands_executed = 0;
static int g_errors_encountered = 0;
static auto g_start_time = std::chrono::steady_clock::now();

// Maximum sizes matching Rust FFI constants
#define MAX_COMMAND_LEN 1024
#define MAX_PATH_LEN 4096
#define MAX_OUTPUT_LEN 1024
#define MAX_METRICS_LEN 512

// Helper function to calculate uptime
static double get_uptime_seconds() {
    auto now = std::chrono::steady_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::seconds>(now - g_start_time);
    return duration.count();
}

// Helper function to execute macOS command
static bool execute_macos_command(const char* command, char* output, size_t output_len) {
    if (!command || !output || output_len == 0) {
        return false;
    }

    // Use popen to execute command and capture output
    std::array<char, 128> buffer;
    std::string result;
    
    FILE* pipe = popen(command, "r");
    if (!pipe) {
        strncpy(output, "Failed to execute command", output_len - 1);
        output[output_len - 1] = '\0';
        return false;
    }

    while (fgets(buffer.data(), buffer.size(), pipe) != nullptr) {
        result += buffer.data();
    }

    int exit_code = pclose(pipe);
    bool success = (exit_code == 0);

    // Copy to output buffer
    strncpy(output, result.c_str(), output_len - 1);
    output[output_len - 1] = '\0';

    return success;
}

// Plugin interface functions

EXPORT const char* get_plugin_info() {
    static std::string info;
    info = g_plugin_name + ":" + g_plugin_version + ":" + g_plugin_description;
    return info.c_str();
}

EXPORT bool plugin_initialize() {
    g_commands_executed = 0;
    g_errors_encountered = 0;
    g_start_time = std::chrono::steady_clock::now();
    return true;
}

EXPORT bool plugin_execute_command(const char* command, char* output, size_t output_len) {
    if (!command || !output || output_len == 0) {
        g_errors_encountered++;
        return false;
    }

    bool success = execute_macos_command(command, output, output_len);
    
    g_commands_executed++;
    if (!success) {
        g_errors_encountered++;
    }

    return success;
}

EXPORT bool plugin_get_metrics(char* metrics_buffer, size_t buffer_len) {
    if (!metrics_buffer || buffer_len == 0) {
        return false;
    }

    std::ostringstream oss;
    oss << "{"
        << "\"commands_executed\":" << g_commands_executed << ","
        << "\"errors_encountered\":" << g_errors_encountered << ","
        << "\"uptime_seconds\":" << get_uptime_seconds() << ","
        << "\"status\":\"" << (g_errors_encountered > 0 ? "degraded" : "healthy") << "\""
        << "}";

    std::string metrics = oss.str();
    strncpy(metrics_buffer, metrics.c_str(), buffer_len - 1);
    metrics_buffer[buffer_len - 1] = '\0';

    return true;
}

EXPORT void plugin_cleanup() {
    // Cleanup resources if needed
    g_commands_executed = 0;
    g_errors_encountered = 0;
}

// Library constructor/destructor for macOS
class PluginInitializer {
public:
    PluginInitializer() {
        plugin_initialize();
    }
    ~PluginInitializer() {
        plugin_cleanup();
    }
};

static PluginInitializer g_initializer;
