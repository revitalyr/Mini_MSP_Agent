//! Minimal System Plugin for Mini MSP Agent
//! Simplified version to avoid memory issues

extern "C" {
    // FFI structures for Rust compatibility
    struct PluginInfo {
        char* name;
        char* version;
        char* description;
        char* author;
        char* license;
        unsigned long long m_timestamp;
    };
    
    struct PluginInterface {
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
    };
    
    const char* get_plugin_info() {
        return "minimal_system_plugin:1.0.0:Minimal system plugin";
    }
    
    bool plugin_initialize() {
        return true;
    }
    
    void plugin_cleanup() {
        // No cleanup needed
    }
    
    PluginInterface* get_plugin_interface() {
        static PluginInterface interface = {};
        return &interface;
    }
}
