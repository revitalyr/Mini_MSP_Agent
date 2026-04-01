#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include "../../include/plugin_interface.h"

static int g_initialized = 0;

int initialize() {
    g_initialized = 1;
    return 1;
}

void cleanup() {
    g_initialized = 0;
}

int get_system_events(EventData* events, int max_count) {
    if (!g_initialized || !events) return 0;
    
    // Simple implementation - return some fake events
    if (max_count >= 2) {
        strcpy(events[0].source, "system");
        strcpy(events[0].type, "startup");
        events[0].timestamp = time(NULL);
        strcpy(events[0].message, "System started successfully");
        
        strcpy(events[1].source, "kernel");
        strcpy(events[1].type, "info");
        events[1].timestamp = time(NULL) - 3600;
        strcpy(events[1].message, "Kernel module loaded");
        
        return 2;
    }
    
    return 0;
}

const char* get_plugin_name() {
    return "Event Data Plugin";
}

const char* get_plugin_version() {
    return "1.0.0";
}

const char* get_plugin_description() {
    return "Plugin for retrieving system event logs on Unix/Linux";
}
