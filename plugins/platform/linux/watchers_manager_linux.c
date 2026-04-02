/**
 * @file watchers_manager_linux.c
 * @brief Linux-specific implementation for Watchers Manager Plugin
 */

#include "../../include/plugin_interface_common.h"
#include "../../include/semantic_types.h"
#include "../../include/watchers_manager_platform.h"
#include <sys/inotify.h>
#include <unistd.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

plugin_result_t linux_init_watcher(watcher_context_t* context) {
    if (!context) return PLUGIN_RESULT_INVALID_PARAM;
    
    int fd = inotify_init();
    if (fd == -1) return PLUGIN_RESULT_ERROR;
    
    int wd = inotify_add_watch(fd, context->watch_path, IN_ALL_EVENTS);
    if (wd == -1) {
        close(fd);
        return PLUGIN_RESULT_ERROR;
    }
    
    linux_watcher_data_t* data = malloc(sizeof(linux_watcher_data_t));
    if (!data) {
        close(fd);
        return PLUGIN_RESULT_ERROR;
    }
    
    data->inotify_fd = fd;
    data->watch_descriptor = wd;
    context->platform_data = data;
    context->is_active = true;
    
    return PLUGIN_RESULT_SUCCESS;
}

plugin_result_t linux_start_watching(watcher_context_t* context) {
    // Linux inotify starts watching immediately after init
    return PLUGIN_RESULT_SUCCESS;
}

plugin_result_t linux_process_events(watcher_context_t* context, watcher_event_t* events, size_t* event_count) {
    if (!context || !events || !event_count) return PLUGIN_RESULT_INVALID_PARAM;
    
    linux_watcher_data_t* data = (linux_watcher_data_t*)context->platform_data;
    
    char buffer[4096];
    ssize_t length = read(data->inotify_fd, buffer, sizeof(buffer));
    
    if (length == -1) return PLUGIN_RESULT_ERROR;
    
    size_t count = 0;
    size_t i = 0;
    
    while (i < (size_t)length && count < *event_count) {
        struct inotify_event* event = (struct inotify_event*)&buffer[i];
        
        watcher_event_t* watcher_event = &events[count];
        
        switch (event->mask) {
            case IN_CREATE:
                watcher_event->type = WATCHER_EVENT_FILE_CREATED;
                break;
            case IN_DELETE:
                watcher_event->type = WATCHER_EVENT_FILE_DELETED;
                break;
            case IN_MODIFY:
                watcher_event->type = WATCHER_EVENT_FILE_MODIFIED;
                break;
            case IN_MOVED_FROM:
            case IN_MOVED_TO:
                watcher_event->type = WATCHER_EVENT_FILE_RENAMED;
                break;
            default:
                watcher_event->type = WATCHER_EVENT_UNKNOWN;
                break;
        }
        
        size_t name_len = event->len;
        if (name_len > sizeof(watcher_event->filename) - 1) {
            name_len = sizeof(watcher_event->filename) - 1;
        }
        
        strncpy(watcher_event->filename, event->name, name_len);
        watcher_event->filename[name_len] = '\0';
        
        watcher_event->timestamp = time(NULL) * 1000;
        
        i += sizeof(struct inotify_event) + event->len;
        count++;
    }
    
    *event_count = count;
    return PLUGIN_RESULT_SUCCESS;
}

plugin_result_t linux_stop_watching(watcher_context_t* context) {
    if (!context || !context->platform_data) return PLUGIN_RESULT_INVALID_PARAM;
    
    linux_watcher_data_t* data = (linux_watcher_data_t*)context->platform_data;
    
    if (data->inotify_fd != -1) {
        close(data->inotify_fd);
        data->inotify_fd = -1;
    }
    
    return PLUGIN_RESULT_SUCCESS;
}

plugin_result_t linux_cleanup_watcher(watcher_context_t* context) {
    if (!context) return PLUGIN_RESULT_INVALID_PARAM;
    
    if (context->platform_data) {
        free(context->platform_data);
        context->platform_data = NULL;
    }
    
    context->is_active = false;
    return PLUGIN_RESULT_SUCCESS;
}
