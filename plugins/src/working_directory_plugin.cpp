#include <windows.h>
#include <cstring>

static const char* PLUGIN_NAME = "modern_directory_info_plugin";
static const char* PLUGIN_VERSION = "1.0.0";
static const char* PLUGIN_DESCRIPTION = "Modern directory monitoring plugin for Windows";

extern "C" {
    typedef struct {
        const char* name;
        const char* version;
        const char* description;
    } PluginInfo;
    
    typedef struct {
        PluginInfo* (*get_plugin_info)();
        bool (*init)();
        void (*cleanup)();
        void* (*get_system_metrics)();
        void* (*handle_command)(const char* command);
        void* (*get_file_info)(const char* path);
        void* (*watch_directory)(const char* path);
        void* (*get_directory_info)(const char* path);
    } PluginInterface;
    
    __declspec(dllexport) const char* get_plugin_name() {
        return PLUGIN_NAME;
    }
    
    __declspec(dllexport) const char* get_plugin_version() {
        return PLUGIN_VERSION;
    }
    
    __declspec(dllexport) const char* get_plugin_platform() {
        return "windows";
    }
    
    static PluginInfo plugin_info = {
        PLUGIN_NAME,
        PLUGIN_VERSION,
        PLUGIN_DESCRIPTION
    };
    
    static PluginInfo* get_plugin_info_impl() {
        return &plugin_info;
    }
    
    static bool init_impl() {
        return true;
    }
    
    static void cleanup_impl() {
        // Cleanup if needed
    }
    
    static PluginInterface plugin_interface = {
        get_plugin_info_impl,
        init_impl,
        cleanup_impl,
        nullptr,  // get_system_metrics
        nullptr,  // handle_command
        nullptr,  // get_file_info
        nullptr,  // watch_directory
        nullptr   // get_directory_info
    };
    
    __declspec(dllexport) void* get_plugin_interface() {
        return &plugin_interface;
    }
}
