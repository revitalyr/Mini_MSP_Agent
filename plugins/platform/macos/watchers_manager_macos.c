/**
 * @file watchers_manager_macos.c
 * @brief macOS-specific implementation for Watchers Manager Plugin
 */

#include "../../include/plugin_interface_common.h"
#include "../../include/semantic_types.h"
#include "../../include/watchers_manager_platform.h"
#include <CoreServices/CoreServices.h>
#include <sys/stat.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

// macOS-specific data structure
typedef struct {
    FSEventStreamRef event_stream;
    CFRunLoopRef run_loop;
    char watch_path[512];
    bool is_active;
} macos_watcher_data_t;

// Callback function for file system events
void macos_event_callback(ConstFSEventStreamRef streamRef,
                        void *clientCallBackInfo,
                        size_t numEvents,
                        void *eventPaths,
                        const FSEventStreamEventFlags eventFlags[],
                        const FSEventStreamEventId eventIds[]) {
    
    char **paths = (char **)eventPaths;
    
    for (size_t i = 0; i < numEvents; i++) {
        printf("macOS Event: %s (flags: %u)\n", paths[i], (unsigned int)eventFlags[i]);
        
        // Convert macOS events to unified events
        if (eventFlags[i] & kFSEventStreamEventFlagItemCreated) {
            printf("  -> File created\n");
        }
        if (eventFlags[i] & kFSEventStreamEventFlagItemRemoved) {
            printf("  -> File deleted\n");
        }
        if (eventFlags[i] & kFSEventStreamEventFlagItemModified) {
            printf("  -> File modified\n");
        }
        if (eventFlags[i] & kFSEventStreamEventFlagItemRenamed) {
            printf("  -> File renamed\n");
        }
    }
}

plugin_result_t macos_init_watcher(watcher_context_t* context) {
    if (!context) return PLUGIN_RESULT_INVALID_PARAM;
    
    macos_watcher_data_t* data = malloc(sizeof(macos_watcher_data_t));
    if (!data) return PLUGIN_RESULT_ERROR;
    
    memset(data, 0, sizeof(macos_watcher_data_t));
    strncpy(data->watch_path, context->watch_path, sizeof(data->watch_path) - 1);
    
    context->platform_handle = data;
    context->is_active = false;
    
    return PLUGIN_RESULT_SUCCESS;
}

plugin_result_t macos_start_watching(watcher_context_t* context) {
    if (!context || !context->platform_handle) return PLUGIN_RESULT_INVALID_PARAM;
    
    macos_watcher_data_t* data = (macos_watcher_data_t*)context->platform_handle;
    
    CFStringRef path = CFStringCreateWithCString(NULL, data->watch_path, kCFStringEncodingUTF8);
    CFArrayRef paths = CFArrayCreate(NULL, (const void **)&path, 1, NULL);
    
    FSEventStreamContext context_info = {0, data, NULL, NULL, NULL};
    
    data->event_stream = FSEventStreamCreate(NULL,
                                          &macos_event_callback,
                                          &context_info,
                                          paths,
                                          kFSEventStreamEventIdSinceNow,
                                          1.0, // latency in seconds
                                          kFSEventStreamCreateFlagFileEvents,
                                          NULL);
    
    if (!data->event_stream) {
        CFRelease(path);
        CFRelease(paths);
        return PLUGIN_RESULT_ERROR;
    }
    
    data->run_loop = CFRunLoopGetCurrent();
    FSEventStreamScheduleWithRunLoop(data->event_stream, data->run_loop, kCFRunLoopDefaultMode);
    FSEventStreamStart(data->event_stream);
    
    data->is_active = true;
    context->is_active = true;
    
    CFRelease(path);
    CFRelease(paths);
    
    return PLUGIN_RESULT_SUCCESS;
}

plugin_result_t macos_process_events(watcher_context_t* context, watcher_event_t* events, size_t* event_count) {
    if (!context || !events || !event_count) return PLUGIN_RESULT_INVALID_PARAM;
    
    // macOS uses callback-based events, so this is a placeholder
    // Events are processed asynchronously in the callback
    *event_count = 0;
    
    return PLUGIN_RESULT_SUCCESS;
}

plugin_result_t macos_stop_watching(watcher_context_t* context) {
    if (!context || !context->platform_handle) return PLUGIN_RESULT_INVALID_PARAM;
    
    macos_watcher_data_t* data = (macos_watcher_data_t*)context->platform_handle;
    
    if (data->event_stream && data->is_active) {
        FSEventStreamStop(data->event_stream);
        FSEventStreamInvalidate(data->event_stream);
        data->is_active = false;
        context->is_active = false;
    }
    
    return PLUGIN_RESULT_SUCCESS;
}

plugin_result_t macos_cleanup_watcher(watcher_context_t* context) {
    if (!context) return PLUGIN_RESULT_INVALID_PARAM;
    
    if (context->platform_handle) {
        macos_watcher_data_t* data = (macos_watcher_data_t*)context->platform_handle;
        
        if (data->event_stream) {
            FSEventStreamRelease(data->event_stream);
        }
        
        free(data);
        context->platform_handle = NULL;
    }
    
    return PLUGIN_RESULT_SUCCESS;
}
