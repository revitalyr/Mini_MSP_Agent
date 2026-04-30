//! NULL System Plugin - minimal implementation

#ifdef _WIN32
#include <windows.h>
#include <stdio.h>
#else
#include <stdlib.h>
#include <cstring>
#endif

extern "C" {
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
    
    PluginInfo* get_plugin_info() {
        static PluginInfo info = {
            const_cast<char*>("null_system_plugin"),
            const_cast<char*>("1.0.0"),
            const_cast<char*>("Null plugin"),
            const_cast<char*>("Mini MSP Agent Team"),
            const_cast<char*>("MIT"),
            0
        };
        return &info;
    }
    
    int plugin_initialize() {
        return 1;
    }
    
    void plugin_cleanup() {
    }
    
    PluginInterface* get_plugin_interface() {
        static PluginInterface interface;
        memset(&interface, 0, sizeof(interface));
        return &interface;
    }
}
