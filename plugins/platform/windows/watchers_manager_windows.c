/**
 * @file watchers_manager_windows.c
 * @brief Windows-specific implementation for Watchers Manager Plugin
 * 
 * Windows platform specific file system watching
 * 
 * @author Mini MSP Agent Team
 * @version 1.0.0
 * @date 2026
 */

#include "../../include/plugin_interface_common.h"
#include "../../include/semantic_types.h"
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// =============================================================================
// 👁️ WINDOWS WATCHER IMPLEMENTATION
// =============================================================================

/**
 * @brief Initialize Windows file watcher
 */
plugin_result_t windows_init_watcher(watcher_context_t* context) {
    if (!context) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    context->platform_data = NULL;
    context->is_active = false;
    
    // Create directory handle for watching
    HANDLE hDir = CreateFileA(
        context->watch_path,
        FILE_LIST_DIRECTORY,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        NULL,
        OPEN_EXISTING,
        FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
        NULL
    );
    
    if (hDir == INVALID_HANDLE_VALUE) {
        return PLUGIN_RESULT_PERMISSION_DENIED;
    }
    
    // Allocate platform-specific data
    windows_watcher_data_t* win_data = (windows_watcher_data_t*)malloc(sizeof(windows_watcher_data_t));
    if (!win_data) {
        CloseHandle(hDir);
        return PLUGIN_RESULT_ERROR;
    }
    
    win_data->directory_handle = hDir;
    win_data->overlap.hEvent = CreateEvent(NULL, TRUE, FALSE, NULL);
    win_data->buffer_size = 32 * 1024; // 32KB buffer
    win_data->buffer = (BYTE*)malloc(win_data->buffer_size);
    
    if (!win_data->buffer) {
        CloseHandle(hDir);
        CloseHandle(win_data->overlap.hEvent);
        free(win_data);
        return PLUGIN_RESULT_ERROR;
    }
    
    context->platform_data = win_data;
    context->is_active = true;
    
    return PLUGIN_RESULT_SUCCESS;
}

/**
 * @brief Start watching directory on Windows
 */
plugin_result_t windows_start_watching(watcher_context_t* context) {
    if (!context || !context->platform_data || !context->is_active) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    windows_watcher_data_t* win_data = (windows_watcher_data_t*)context->platform_data;
    
    // Read initial directory state
    DWORD bytesReturned;
    if (!ReadDirectoryChangesW(
        win_data->directory_handle,
        win_data->buffer,
        win_data->buffer_size,
        TRUE,
        FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_DIR_NAME | 
        FILE_NOTIFY_CHANGE_ATTRIBUTES | FILE_NOTIFY_CHANGE_SIZE | 
        FILE_NOTIFY_CHANGE_LAST_WRITE | FILE_NOTIFY_CHANGE_CREATION,
        &bytesReturned,
        &win_data->overlap,
        NULL)) {
        return PLUGIN_RESULT_ERROR;
    }
    
    return PLUGIN_RESULT_SUCCESS;
}

/**
 * @brief Process Windows watcher events
 */
plugin_result_t windows_process_events(watcher_context_t* context, watcher_event_t* events, size_t* event_count) {
    if (!context || !events || !event_count) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    windows_watcher_data_t* win_data = (windows_watcher_data_t*)context->platform_data;
    
    DWORD bytesReturned;
    if (!GetOverlappedResult(win_data->directory_handle, &win_data->overlap, &bytesReturned, FALSE)) {
        return PLUGIN_RESULT_ERROR;
    }
    
    if (bytesReturned == 0) {
        *event_count = 0;
        return PLUGIN_RESULT_SUCCESS;
    }
    
    // Parse notification buffer
    FILE_NOTIFY_INFORMATION* pNotify = (FILE_NOTIFY_INFORMATION*)win_data->buffer;
    size_t count = 0;
    
    while (count < *event_count && (BYTE*)pNotify < win_data->buffer + bytesReturned) {
        watcher_event_t* event = &events[count];
        
        // Convert Windows event to unified event
        switch (pNotify->Action) {
            case FILE_ACTION_ADDED:
                event->type = WATCHER_EVENT_FILE_CREATED;
                break;
            case FILE_ACTION_REMOVED:
                event->type = WATCHER_EVENT_FILE_DELETED;
                break;
            case FILE_ACTION_MODIFIED:
                event->type = WATCHER_EVENT_FILE_MODIFIED;
                break;
            case FILE_ACTION_RENAMED_OLD_NAME:
            case FILE_ACTION_RENAMED_NEW_NAME:
                event->type = WATCHER_EVENT_FILE_RENAMED;
                break;
            default:
                event->type = WATCHER_EVENT_UNKNOWN;
                break;
        }
        
        // Copy filename
        size_t nameLen = pNotify->FileNameLength / sizeof(WCHAR);
        if (nameLen > sizeof(event->filename) - 1) {
            nameLen = sizeof(event->filename) - 1;
        }
        
        // Convert WCHAR to char (simplified)
        for (size_t i = 0; i < nameLen; i++) {
            event->filename[i] = (char)pNotify->FileName[i];
        }
        event->filename[nameLen] = '\0';
        
        // Set timestamp
        event->timestamp = GetTickCount64() * 1000; // Convert to milliseconds
        
        count++;
        
        // Move to next entry
        if (pNotify->NextEntryOffset == 0) break;
        pNotify = (FILE_NOTIFY_INFORMATION*)((BYTE*)pNotify + pNotify->NextEntryOffset);
    }
    
    *event_count = count;
    
    // Restart watching
    ReadDirectoryChangesW(
        win_data->directory_handle,
        win_data->buffer,
        win_data->buffer_size,
        TRUE,
        FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_DIR_NAME | 
        FILE_NOTIFY_CHANGE_ATTRIBUTES | FILE_NOTIFY_CHANGE_SIZE | 
        FILE_NOTIFY_CHANGE_LAST_WRITE | FILE_NOTIFY_CHANGE_CREATION,
        &bytesReturned,
        &win_data->overlap,
        NULL);
    
    return PLUGIN_RESULT_SUCCESS;
}

/**
 * @brief Stop watching on Windows
 */
plugin_result_t windows_stop_watching(watcher_context_t* context) {
    if (!context || !context->platform_data) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    windows_watcher_data_t* win_data = (windows_watcher_data_t*)context->platform_data;
    
    // Cancel pending operations
    CancelIoEx(win_data->directory_handle, &win_data->overlap);
    
    return PLUGIN_RESULT_SUCCESS;
}

/**
 * @brief Cleanup Windows watcher
 */
plugin_result_t windows_cleanup_watcher(watcher_context_t* context) {
    if (!context) {
        return PLUGIN_RESULT_INVALID_PARAM;
    }
    
    if (context->platform_data) {
        windows_watcher_data_t* win_data = (windows_watcher_data_t*)context->platform_data;
        
        if (win_data->directory_handle) {
            CloseHandle(win_data->directory_handle);
        }
        
        if (win_data->overlap.hEvent) {
            CloseHandle(win_data->overlap.hEvent);
        }
        
        if (win_data->buffer) {
            free(win_data->buffer);
        }
        
        free(win_data);
        context->platform_data = NULL;
    }
    
    context->is_active = false;
    return PLUGIN_RESULT_SUCCESS;
}
