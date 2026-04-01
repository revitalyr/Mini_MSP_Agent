/**
 * @file event_data_plugin.c
 * @brief Event Data Plugin for Mini MSP Agent
 * 
 * Provides system event monitoring and data collection including
 * system logs, security events, and application events.
 * 
 * @author Mini MSP Agent Team
 * @version 1.0.0
 * @date 2026
 */

#include "../include/plugin_interface.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#ifdef _WIN32
#include <windows.h>
#include <evntprov.h>
#include <winevt.h>
#pragma comment(lib, "wevtapi.lib")
#else
#include <syslog.h>
#include <unistd.h>
#include <sys/inotify.h>
#include <errno.h>
#endif

// Plugin information
static plugin_info_t event_data_plugin_info = {
    .name = "event_data",
    .version = "1.0.0",
    .description = "Monitors and collects system event data"
};

/**
 * @brief Event data structure
 */
typedef struct {
    uint64_t timestamp;
    char source[128];
    char event_id[64];
    char level[32];
    char message[1024];
    char category[64];
    uint32_t event_code;
} event_data_t;

/**
 * @brief Event filter structure
 */
typedef struct {
    char source[128];
    char level[32];
    uint32_t event_code;
    bool enabled;
} event_filter_t;

// Plugin state
static bool event_monitoring_active = false;
static event_filter_t* event_filters = NULL;
static size_t filter_count = 0;

// Plugin implementation
static bool event_data_init(void) {
#ifdef _WIN32
    // Initialize Windows Event Log subsystem
    // This would typically involve setting up event subscriptions
#else
    // Initialize Linux syslog monitoring
    openlog("mini_msp_agent", LOG_PID | LOG_CONS, LOG_USER);
#endif
    return true;
}

static void event_data_cleanup(void) {
    if (event_filters) {
        free(event_filters);
        event_filters = NULL;
    }
    filter_count = 0;
    
#ifdef _WIN32
    // Cleanup Windows Event Log resources
#else
    closelog();
#endif
}

static plugin_info_t* event_data_get_plugin_info(void) {
    return &event_data_plugin_info;
}

static bool event_data_get_system_metrics(system_metrics_t* metrics) {
    // Not applicable for event data plugin
    return false;
}

static bool event_data_get_processes(process_info_t** processes, size_t* count) {
    // Not applicable for event data plugin
    return false;
}

static bool event_data_execute_command(const char* command, command_result_t* result) {
    // Not applicable for event data plugin
    return false;
}

static bool event_data_read_file(const char* path, file_content_t* content) {
    // Not applicable for event data plugin
    return false;
}

static bool event_data_get_system_info(system_info_t* info) {
    // Not applicable for event data plugin
    return false;
}

/**
 * @brief Get recent system events
 */
static bool get_recent_events(event_data_t** events, size_t* count, uint32_t max_events) {
    if (!events || !count) return false;
    
    *events = NULL;
    *count = 0;
    
#ifdef _WIN32
    EVT_HANDLE hResults = NULL;
    EVT_HANDLE hContext = NULL;
    DWORD status = ERROR_SUCCESS;
    DWORD countReturned = 0;
    
    // Query recent events from System log
    LPCWSTR query = L"*[System[TimeCreated[timediff(@SystemTime) <= 86400000]]]";
    
    hResults = EvtQuery(NULL, L"System", query, EvtQueryChannelPath | EvtQueryTolerateQueryErrors);
    if (hResults == NULL) {
        return false;
    }
    
    // Get event count
    if (!EvtNext(hResults, max_events, NULL, 0, 0, &countReturned)) {
        EvtClose(hResults);
        return false;
    }
    
    if (countReturned == 0) {
        EvtClose(hResults);
        return true; // No events is not an error
    }
    
    // Allocate memory for events
    *events = (event_data_t*)malloc(countReturned * sizeof(event_data_t));
    if (!*events) {
        EvtClose(hResults);
        return false;
    }
    
    // Process each event
    EVT_HANDLE* eventHandles = (EVT_HANDLE*)malloc(countReturned * sizeof(EVT_HANDLE));
    if (!eventHandles) {
        free(*events);
        *events = NULL;
        EvtClose(hResults);
        return false;
    }
    
    if (EvtNext(hResults, countReturned, eventHandles, INFINITE, 0, &countReturned)) {
        for (DWORD i = 0; i < countReturned; i++) {
            event_data_t* event = &(*events)[i];
            memset(event, 0, sizeof(event_data_t));
            
            // Extract event information
            EVT_VARIANT variant;
            DWORD bufferSize = 0;
            DWORD bufferUsed = 0;
            
            // Get timestamp
            if (EvtGetValue(eventHandles[i], NULL, 0, &bufferUsed, &bufferSize)) {
                EvtGetValue(eventHandles[i], &variant, bufferSize, &bufferUsed, NULL);
                // Convert FILETIME to Unix timestamp
                if (variant.Type == EvtVarTypeFileTime) {
                    ULARGE_INTEGER ull;
                    ull.LowPart = variant.FileTime.dwLowDateTime;
                    ull.HighPart = variant.FileTime.dwHighDateTime;
                    event->timestamp = (ull.QuadPart - 116444736000000000ULL) / 10000000ULL;
                }
            }
            
            // Get other event properties (simplified for this example)
            strcpy(event->source, "System");
            strcpy(event->level, "Information");
            strcpy(event->message, "System event");
            strcpy(event->category, "System");
            event->event_code = 1000;
            
            EvtClose(eventHandles[i]);
        }
    }
    
    *count = countReturned;
    free(eventHandles);
    EvtClose(hResults);
    
#else
    // Linux implementation using syslog
    FILE* fp = fopen("/var/log/syslog", "r");
    if (!fp) {
        fp = fopen("/var/log/messages", "r");
    }
    
    if (!fp) {
        return false;
    }
    
    // Count lines (simplified approach)
    char* line = NULL;
    size_t len = 0;
    size_t lineCount = 0;
    
    while (getline(&line, &len, fp) != -1 && lineCount < max_events) {
        lineCount++;
    }
    
    if (lineCount == 0) {
        fclose(fp);
        free(line);
        return true;
    }
    
    // Allocate memory for events
    *events = (event_data_t*)malloc(lineCount * sizeof(event_data_t));
    if (!*events) {
        fclose(fp);
        free(line);
        return false;
    }
    
    // Parse log lines
    rewind(fp);
    size_t index = 0;
    
    while (getline(&line, &len, fp) != -1 && index < max_events) {
        event_data_t* event = &(*events)[index];
        memset(event, 0, sizeof(event_data_t));
        
        // Parse syslog format (simplified)
        event->timestamp = time(NULL); // Use current time as fallback
        
        // Extract source and message (basic parsing)
        char* space1 = strchr(line, ' ');
        char* space2 = space1 ? strchr(space1 + 1, ' ') : NULL;
        char* colon = space2 ? strchr(space2 + 1, ':') : NULL;
        
        if (colon) {
            *colon = '\0';
            strncpy(event->source, space2 + 1, sizeof(event->source) - 1);
            strncpy(event->message, colon + 2, sizeof(event->message) - 1);
        } else {
            strncpy(event->message, line, sizeof(event->message) - 1);
        }
        
        strcpy(event->level, "Information");
        strcpy(event->category, "System");
        event->event_code = 1000;
        
        index++;
    }
    
    *count = index;
    fclose(fp);
    free(line);
#endif
    
    return true;
}

/**
 * @brief Add event filter
 */
static bool add_event_filter(const char* source, const char* level, uint32_t event_code) {
    event_filter_t* new_filters = (event_filter_t*)realloc(event_filters, 
                                                      (filter_count + 1) * sizeof(event_filter_t));
    if (!new_filters) {
        return false;
    }
    
    event_filters = new_filters;
    event_filter_t* filter = &event_filters[filter_count];
    
    if (source) {
        strncpy(filter->source, source, sizeof(filter->source) - 1);
    } else {
        filter->source[0] = '\0';
    }
    
    if (level) {
        strncpy(filter->level, level, sizeof(filter->level) - 1);
    } else {
        filter->level[0] = '\0';
    }
    
    filter->event_code = event_code;
    filter->enabled = true;
    
    filter_count++;
    return true;
}

/**
 * @brief Start event monitoring
 */
static bool start_event_monitoring(void) {
    if (event_monitoring_active) {
        return true; // Already monitoring
    }
    
#ifdef _WIN32
    // Start Windows Event Log monitoring
    // This would typically involve setting up event subscriptions
#else
    // Start Linux inotify monitoring for log files
    // This would involve setting up inotify watches on log directories
#endif
    
    event_monitoring_active = true;
    return true;
}

/**
 * @brief Stop event monitoring
 */
static bool stop_event_monitoring(void) {
    if (!event_monitoring_active) {
        return true; // Not monitoring
    }
    
#ifdef _WIN32
    // Stop Windows Event Log monitoring
#else
    // Stop Linux inotify monitoring
#endif
    
    event_monitoring_active = false;
    return true;
}

static void event_data_free_memory(void* ptr) {
    if (ptr) {
        free(ptr);
    }
}

// Plugin interface
static plugin_interface_t event_data_interface = {
    .get_plugin_info = event_data_get_plugin_info,
    .init = event_data_init,
    .cleanup = event_data_cleanup,
    .get_system_metrics = event_data_get_system_metrics,
    .get_processes = event_data_get_processes,
    .execute_command = event_data_execute_command,
    .read_file = event_data_read_file,
    .get_system_info = event_data_get_system_info,
    .free_memory = event_data_free_memory
};

// Plugin entry point
PLUGIN_EXPORT plugin_interface_t* PLUGIN_CALL get_plugin_interface(void) {
    return &event_data_interface;
}
