extern "C" {
    __declspec(dllexport) const char* get_plugin_name() {
        return "modern_directory_info_plugin";
    }
    
    __declspec(dllexport) const char* get_plugin_version() {
        return "1.0.0";
    }
    
    __declspec(dllexport) const char* get_plugin_platform() {
        return "windows";
    }
    
    __declspec(dllexport) void* get_plugin_interface() {
        return nullptr;
    }
}
