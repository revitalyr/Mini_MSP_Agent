/**
 * Custom Command Plugin - C++ Implementation for Windows
 * 
 * Provides custom command execution functionality via C FFI
 * Replaces the Rust custom_plugin.rs with native C++ implementation
 */

#include <windows.h>
#include <string>
#include <cstring>
#include <ctime>
#include <chrono>
#include <sstream>

#define EXPORT extern "C" __declspec(dllexport)

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

// Helper function to execute Windows command
static bool execute_windows_command(const char* command, char* output, size_t output_len) {
    SECURITY_ATTRIBUTES sa;
    sa.nLength = sizeof(SECURITY_ATTRIBUTES);
    sa.bInheritHandle = TRUE;
    sa.lpSecurityDescriptor = NULL;

    HANDLE hRead, hWrite;
    if (!CreatePipe(&hRead, &hWrite, &sa, 0)) {
        return false;
    }

    STARTUPINFOA si;
    PROCESS_INFORMATION pi;
    ZeroMemory(&si, sizeof(si));
    si.cb = sizeof(si);
    si.hStdOutput = hWrite;
    si.hStdError = hWrite;
    si.dwFlags |= STARTF_USESTDHANDLES;

    ZeroMemory(&pi, sizeof(pi));

    // Create command string
    std::string cmd = "cmd /c ";
    cmd += command;

    BOOL success = CreateProcessA(NULL, const_cast<char*>(cmd.c_str()), NULL, NULL, TRUE,
                                 CREATE_NO_WINDOW, NULL, NULL, &si, &pi);

    if (!success) {
        CloseHandle(hWrite);
        CloseHandle(hRead);
        return false;
    }

    CloseHandle(hWrite);

    // Read output
    DWORD bytesRead;
    std::string result;
    char buffer[1024];
    while (ReadFile(hRead, buffer, sizeof(buffer) - 1, &bytesRead, NULL) && bytesRead > 0) {
        buffer[bytesRead] = '\0';
        result += buffer;
    }

    WaitForSingleObject(pi.hProcess, INFINITE);

    DWORD exitCode;
    GetExitCodeProcess(pi.hProcess, &exitCode);

    CloseHandle(pi.hProcess);
    CloseHandle(pi.hThread);
    CloseHandle(hRead);

    // Copy to output buffer
    strncpy_s(output, output_len, result.c_str(), output_len - 1);
    output[output_len - 1] = '\0';

    return exitCode == 0;
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

    bool success = execute_windows_command(command, output, output_len);
    
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
    strncpy_s(metrics_buffer, buffer_len, metrics.c_str(), buffer_len - 1);
    metrics_buffer[buffer_len - 1] = '\0';

    return true;
}

EXPORT void plugin_cleanup() {
    // Cleanup resources if needed
    g_commands_executed = 0;
    g_errors_encountered = 0;
}

// Windows DLL entry point
BOOL APIENTRY DllMain(HMODULE hModule, DWORD ul_reason_for_call, LPVOID lpReserved) {
    switch (ul_reason_for_call) {
        case DLL_PROCESS_ATTACH:
            plugin_initialize();
            break;
        case DLL_PROCESS_DETACH:
            plugin_cleanup();
            break;
    }
    return TRUE;
}
