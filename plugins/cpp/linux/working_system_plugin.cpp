#include "../../include/plugin_interface.h"
#include <cstring>

#define EXPORT __attribute__((visibility("default")))
#define PLUGIN_PLATFORM "linux"

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
        nullptr,  // get_processes
        nullptr,  // execute_command
        nullptr,  // read_file
        nullptr,  // get_system_info
        nullptr,  // get_directory_info_data
        nullptr,  // get_event_data
        nullptr,  // get_watchers_data
        nullptr,  // get_file_reader_data
        nullptr,  // get_sensor_data
        nullptr,  // get_processing_results
        nullptr,  // get_video_frame
        nullptr,  // get_forensic_data
        nullptr,  // free_memory
        nullptr   // execute_json
    };

    EXPORT plugin_interface_t* get_plugin_interface() {
        return &plugin_interface;
    }
}