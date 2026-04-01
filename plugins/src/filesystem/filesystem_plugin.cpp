#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>
#include <dirent.h>
#include <fcntl.h>
#include <time.h>
#include <errno.h>
#include <pthread.h>

#ifdef __linux__
#include <sys/inotify.h>
#include <sys/sendfile.h>
#endif

#ifdef _WIN32
#include <windows.h>
#include <fileapi.h>
#include <handleapi.h>
#endif

// Plugin structure
typedef struct {
    char* name;
    char* version;
    char* platform;
} PluginInfo;

// Response structures
typedef struct {
    char* error;
    char* data;
    int success;
} PluginResponse;

typedef struct {
    char* path;
    char* type;
    long size;
    time_t modified;
    char* permissions;
} FileInfo;

typedef struct {
    char* path;
    int file_count;
    int dir_count;
    long total_size;
} DirectoryInfo;

typedef struct {
    char* event_type;
    char* path;
    time_t timestamp;
} EventData;

// Global variables
static PluginInfo plugin_info = {
    .name = "Filesystem Plugin",
    .version = "1.0.0",
    .platform = "Unix"
};

#ifdef __linux__
static int inotify_fd = -1;
static pthread_mutex_t watch_mutex = PTHREAD_MUTEX_INITIALIZER;
#endif

// Helper functions
static char* create_string(const char* src) {
    if (!src) return NULL;
    char* dest = malloc(strlen(src) + 1);
    if (dest) strcpy(dest, src);
    return dest;
}

static PluginResponse* create_response(int success, const char* error, const char* data) {
    PluginResponse* response = malloc(sizeof(PluginResponse));
    if (response) {
        response->success = success;
        response->error = error ? create_string(error) : NULL;
        response->data = data ? create_string(data) : NULL;
    }
    return response;
}

// Directory Information
static PluginResponse* get_directory_info(const char* path) {
    struct stat st;
    DIR* dir;
    struct dirent* entry;
    DirectoryInfo info = {0};
    
    if (stat(path, &st) != 0) {
        return create_response(0, "Directory not accessible", NULL);
    }
    
    if (!S_ISDIR(st.st_mode)) {
        return create_response(0, "Path is not a directory", NULL);
    }
    
    info.path = create_string(path);
    
    dir = opendir(path);
    if (!dir) {
        return create_response(0, "Cannot open directory", NULL);
    }
    
    while ((entry = readdir(dir)) != NULL) {
        if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) {
            continue;
        }
        
        char full_path[1024];
        snprintf(full_path, sizeof(full_path), "%s/%s", path, entry->d_name);
        
        struct stat entry_stat;
        if (stat(full_path, &entry_stat) == 0) {
            if (S_ISDIR(entry_stat.st_mode)) {
                info.dir_count++;
            } else {
                info.file_count++;
                info.total_size += entry_stat.st_size;
            }
        }
    }
    
    closedir(dir);
    
    char* json_data = malloc(1024);
    snprintf(json_data, 1024,
        "{\"path\":\"%s\",\"file_count\":%d,\"dir_count\":%d,\"total_size\":%ld}",
        info.path, info.file_count, info.dir_count, info.total_size);
    
    free(info.path);
    return create_response(1, NULL, json_data);
}

// File Information
static PluginResponse* get_file_info(const char* path) {
    struct stat st;
    
    if (stat(path, &st) != 0) {
        return create_response(0, "File not accessible", NULL);
    }
    
    char permissions[11];
    snprintf(permissions, 11, "%o", st.st_mode & 0777);
    
    char* json_data = malloc(1024);
    snprintf(json_data, 1024,
        "{\"path\":\"%s\",\"size\":%ld,\"modified\":%ld,\"permissions\":\"%s\",\"type\":\"%s\"}",
        path, st.st_size, st.st_mtime, permissions,
        S_ISDIR(st.st_mode) ? "directory" : "file");
    
    return create_response(1, NULL, json_data);
}

// File Reader
static PluginResponse* read_file_content(const char* path, int max_size) {
    FILE* file = fopen(path, "r");
    if (!file) {
        return create_response(0, "Cannot open file", NULL);
    }
    
    // Get file size
    fseek(file, 0, SEEK_END);
    long file_size = ftell(file);
    fseek(file, 0, SEEK_SET);
    
    if (file_size > max_size) {
        fclose(file);
        return create_response(0, "File too large", NULL);
    }
    
    char* content = malloc(file_size + 1);
    if (!content) {
        fclose(file);
        return create_response(0, "Memory allocation failed", NULL);
    }
    
    size_t bytes_read = fread(content, 1, file_size, file);
    content[bytes_read] = '\0';
    
    fclose(file);
    
    // Escape JSON string
    char* escaped_content = malloc(file_size * 2 + 100);
    char* ptr = escaped_content;
    *ptr++ = '"';
    
    for (size_t i = 0; i < bytes_read; i++) {
        if (content[i] == '"' || content[i] == '\\') {
            *ptr++ = '\\';
        }
        if (content[i] == '\n') {
            *ptr++ = '\\';
            *ptr++ = 'n';
        } else if (content[i] == '\r') {
            *ptr++ = '\\';
            *ptr++ = 'r';
        } else if (content[i] == '\t') {
            *ptr++ = '\\';
            *ptr++ = 't';
        } else {
            *ptr++ = content[i];
        }
    }
    
    *ptr++ = '"';
    *ptr = '\0';
    
    char* json_data = malloc(strlen(escaped_content) + 100);
    snprintf(json_data, strlen(escaped_content) + 100,
        "{\"path\":\"%s\",\"content\":%s,\"size\":%ld}",
        path, escaped_content, file_size);
    
    free(content);
    free(escaped_content);
    
    return create_response(1, NULL, json_data);
}

// File Signature (MD5-like checksum)
static PluginResponse* get_file_signature(const char* path) {
    FILE* file = fopen(path, "rb");
    if (!file) {
        return create_response(0, "Cannot open file", NULL);
    }
    
    unsigned int checksum = 0;
    int byte;
    
    while ((byte = fgetc(file)) != EOF) {
        checksum = ((checksum << 1) | (checksum >> 31)) ^ byte;
    }
    
    fclose(file);
    
    char* json_data = malloc(256);
    snprintf(json_data, 256,
        "{\"path\":\"%s\",\"signature\":\"%08x\",\"algorithm\":\"simple_checksum\"}",
        path, checksum);
    
    return create_response(1, NULL, json_data);
}

// Root Directory Information
static PluginResponse* get_root_directory_info() {
    return get_directory_info("/");
}

// Scanner (recursive directory scan)
static PluginResponse* scan_directory(const char* path, int max_depth) {
    // Simplified scanner - just returns immediate directory contents
    return get_directory_info(path);
}

#ifdef __linux__
// Folder Watcher (using inotify)
static PluginResponse* start_folder_watcher(const char* path) {
    pthread_mutex_lock(&watch_mutex);
    
    if (inotify_fd == -1) {
        inotify_fd = inotify_init();
        if (inotify_fd == -1) {
            pthread_mutex_unlock(&watch_mutex);
            return create_response(0, "Cannot initialize inotify", NULL);
        }
    }
    
    int wd = inotify_add_watch(inotify_fd, path, IN_CREATE | IN_DELETE | IN_MODIFY | IN_MOVED);
    if (wd == -1) {
        pthread_mutex_unlock(&watch_mutex);
        return create_response(0, "Cannot add watch", NULL);
    }
    
    pthread_mutex_unlock(&watch_mutex);
    
    char* json_data = malloc(256);
    snprintf(json_data, 256,
        "{\"path\":\"%s\",\"watch_id\":%d,\"status\":\"watching\"}",
        path, wd);
    
    return create_response(1, NULL, json_data);
}

static PluginResponse* get_file_events() {
    if (inotify_fd == -1) {
        return create_response(0, "No active watchers", NULL);
    }
    
    char buffer[4096];
    int length = read(inotify_fd, buffer, sizeof(buffer));
    
    if (length <= 0) {
        return create_response(0, "No events", NULL);
    }
    
    // Parse inotify events (simplified)
    char* json_data = malloc(1024);
    strcpy(json_data, "{\"events\":[");
    
    int event_count = 0;
    int i = 0;
    while (i < length) {
        struct inotify_event* event = (struct inotify_event*)&buffer[i];
        
        if (event_count > 0) {
            strcat(json_data, ",");
        }
        
        char event_info[256];
        snprintf(event_info, 256,
            "{\"type\":\"%s\",\"path\":\"%s\",\"wd\":%d}",
            (event->mask & IN_CREATE) ? "create" :
            (event->mask & IN_DELETE) ? "delete" :
            (event->mask & IN_MODIFY) ? "modify" : "unknown",
            event->name, event->wd);
        
        strcat(json_data, event_info);
        event_count++;
        
        i += sizeof(struct inotify_event) + event->len;
    }
    
    strcat(json_data, "]}");
    
    return create_response(1, NULL, json_data);
}
#else
// Placeholder for non-Linux systems
static PluginResponse* start_folder_watcher(const char* path) {
    return create_response(0, "Folder watching not supported on this platform", NULL);
}

static PluginResponse* get_file_events() {
    return create_response(0, "File events not supported on this platform", NULL);
}
#endif

// Watchers Manager
static PluginResponse* list_active_watchers() {
    char* json_data = malloc(256);
    strcpy(json_data, "{\"watchers\":[],\"platform\":\"not_supported\"}");
    return create_response(1, NULL, json_data);
}

// Event Data (mock implementation)
static PluginResponse* get_event_data(const char* event_type) {
    char* json_data = malloc(256);
    snprintf(json_data, 256,
        "{\"event_type\":\"%s\",\"timestamp\":%ld,\"data\":\"mock_event_data\"}",
        event_type, time(NULL));
    
    return create_response(1, NULL, json_data);
}

// File Listener (mock implementation)
static PluginResponse* start_file_listener(const char* path) {
    char* json_data = malloc(256);
    snprintf(json_data, 256,
        "{\"path\":\"%s\",\"status\":\"listening\",\"type\":\"file_listener\"}",
        path);
    
    return create_response(1, NULL, json_data);
}

// Plugin API functions
extern "C" {
    PluginResponse* execute_function(const char* function_name, const char* params) {
        if (!function_name) {
            return create_response(0, "No function name provided", NULL);
        }
        
        if (strcmp(function_name, "directory_information") == 0) {
            return get_directory_info(params);
        } else if (strcmp(function_name, "file_reader") == 0) {
            return read_file_content(params, 1024 * 1024); // 1MB limit
        } else if (strcmp(function_name, "file_signature") == 0) {
            return get_file_signature(params);
        } else if (strcmp(function_name, "root_directory_information") == 0) {
            return get_root_directory_info();
        } else if (strcmp(function_name, "scanner") == 0) {
            return scan_directory(params, 3); // 3 levels deep
        } else if (strcmp(function_name, "folder_watcher") == 0) {
            return start_folder_watcher(params);
        } else if (strcmp(function_name, "file_listener") == 0) {
            return start_file_listener(params);
        } else if (strcmp(function_name, "watchers_manager") == 0) {
            return list_active_watchers();
        } else if (strcmp(function_name, "event_data") == 0) {
            return get_event_data(params);
        } else if (strcmp(function_name, "get_file_events") == 0) {
            return get_file_events();
        } else {
            return create_response(0, "Unknown function", NULL);
        }
    }
    
    PluginInfo* get_plugin_info() {
        return &plugin_info;
    }
    
    int initialize() {
        printf("Filesystem Plugin initialized\n");
        return 1;
    }
    
    void cleanup() {
#ifdef __linux__
        if (inotify_fd != -1) {
            close(inotify_fd);
            inotify_fd = -1;
        }
#endif
        printf("Filesystem Plugin cleaned up\n");
    }
}
