#include "../include/plugin_interface.h"
#include <windows.h>
#include <cstring>

static const char* PLUGIN_NAME = "modern_system_plugin";
static const char* PLUGIN_VERSION = "1.0.0";
static const char* PLUGIN_DESCRIPTION = "Modern system monitoring plugin for Windows";

extern "C" {
    __declspec(dllexport) const char* get_plugin_name() {
        return PLUGIN_NAME;
    }
    
    __declspec(dllexport) const char* get_plugin_version() {
        return PLUGIN_VERSION;
    }
    
    __declspec(dllexport) const char* get_plugin_platform() {
        return "windows";
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
    
    __declspec(dllexport) plugin_interface_t* get_plugin_interface() {
        return &plugin_interface;
    }
}
