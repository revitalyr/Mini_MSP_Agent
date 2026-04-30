#include "../include/plugin_interface.h"
#include <cstring>

#ifdef _WIN32
    #include <windows.h>
    #define EXPORT __declspec(dllexport)
    #define PLUGIN_PLATFORM "windows"
#elif defined(__APPLE__)
    #define EXPORT __attribute__((visibility("default")))
    #define PLUGIN_PLATFORM "macos"
#elif defined(__linux__)
    #define EXPORT __attribute__((visibility("default")))
    #define PLUGIN_PLATFORM "linux"
#else
    #define EXPORT
    #define PLUGIN_PLATFORM "unknown"
#endif

static const char* PLUGIN_NAME = "modern_system_plugin";
static const char* PLUGIN_VERSION = "1.0.0";
static const char* PLUGIN_DESCRIPTION = "Modern system monitoring plugin";

extern "C" {
    EXPORT const char* get_plugin_name() {
        return PLUGIN_NAME;
    }

    EXPORT const char* get_plugin_version() {
        return PLUGIN_VERSION;
    }

    EXPORT const char* get_plugin_platform() {
        return PLUGIN_PLATFORM;
    }

    static plugin_info_t plugin_info = {
        PLUGIN_NAME,
        PLUGIN_VERSION,
        PLUGIN_DESCRIPTION
    };

    static plugin_info_t* get_plugin_info_impl() {
        return &plugin_info;
    }

    static bool init_impl() {
        return true;
    }

    static void cleanup_impl() {
        // Cleanup if needed
    }

    static plugin_interface_t plugin_interface = {
        get_plugin_info_impl,
        init_impl,
        cleanup_impl,
        nullptr,  // get_system_metrics
        nullptr,  // handle_command
        nullptr,  // get_file_info
        nullptr,  // watch_directory
        nullptr   // get_directory_info
    };
    
    EXPORT plugin_interface_t* get_plugin_interface() {
        return &plugin_interface;
    }
}
