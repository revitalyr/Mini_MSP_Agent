//! Safe System Plugin for Mini MSP Agent
//! Based on working linux system plugin but without memory issues

#ifdef _WIN32
#include <windows.h>
#include <stdio.h>
#else
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <unistd.h>
#include <sys/sysinfo.h>
#include <sys/utsname.h>
#include <cstring>
#endif

extern "C" {
    // FFI structures for Rust compatibility
    typedef struct {
        char* name;
        char* version;
        char* description;
        char* author;
        char* license;
        unsigned long long m_timestamp;
    } PluginInfo;
    
    typedef struct {
        void* get_plugin_info;
        void* init;
        void* cleanup;
        void* get_system_metrics;
        void* get_processes;
        void* execute_command;
        void* read_file;
        void* get_system_info;
        void* get_directory_info_data;
        void* free_directory_info_data;
        void* get_file_signature_data;
        void* free_file_signature_data;
        void* get_root_directory_info;
        void* scan_directory;
        void* free_scan_result;
        void* create_folder_watcher;
        void* destroy_folder_watcher;
        void* create_file_listener;
        void* destroy_file_listener;
        void* get_watcher_events;
        void* free_watcher_events;
    } PluginInterface;
    
    static const char* plugin_name = "safe_system_plugin";
    static const char* plugin_version = "1.0.0";
    static const char* plugin_description = "Safe system plugin without memory issues";
    static const char* plugin_author = "Mini MSP Agent Team";
    static const char* plugin_license = "MIT";
    
    PluginInfo* get_plugin_info() {
        static PluginInfo info = {
            const_cast<char*>(plugin_name),
            const_cast<char*>(plugin_version),
            const_cast<char*>(plugin_description),
            const_cast<char*>(plugin_author),
            const_cast<char*>(plugin_license),
            0
        };
        return &info;
    }
    
    bool plugin_initialize() {
        return true;
    }
    
    void plugin_cleanup() {
        // No cleanup needed
    }
    
    PluginInterface* get_plugin_interface() {
        static PluginInterface interface;
        // Initialize all fields to zero for MSVC compatibility
        memset(&interface, 0, sizeof(interface));
        interface.get_plugin_info = (void*)get_plugin_info;
        interface.init = (void*)plugin_initialize;
        interface.cleanup = (void*)plugin_cleanup;
        return &interface;
    }
}
