#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dirent.h>
#include <sys/stat.h>
#include "../../include/plugin_interface.h"

static int g_initialized = 0;

int initialize() {
    g_initialized = 1;
    return 1;
}

void cleanup() {
    g_initialized = 0;
}

int get_directory_contents(const char* path, FileInfo* files, int max_count) {
    if (!g_initialized || !path || !files) return 0;
    
    DIR* dir = opendir(path);
    if (!dir) return 0;
    
    int count = 0;
    struct dirent* entry;
    
    while ((entry = readdir(dir)) != NULL && count < max_count) {
        strncpy(files[count].path, entry->d_name, sizeof(files[count].path) - 1);
        
        struct stat st;
        char full_path[1024];
        snprintf(full_path, sizeof(full_path), "%s/%s", path, entry->d_name);
        
        if (stat(full_path, &st) == 0) {
            files[count].size = st.st_size;
            files[count].modified = st.st_mtime;
            files[count].is_directory = S_ISDIR(st.st_mode);
        }
        
        count++;
    }
    
    closedir(dir);
    return count;
}

const char* get_plugin_name() {
    return "Directory Info Plugin";
}

const char* get_plugin_version() {
    return "1.0.0";
}

const char* get_plugin_description() {
    return "Plugin for reading directory contents on Unix/Linux";
}
