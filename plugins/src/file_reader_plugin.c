/**
 * @file file_reader_plugin.c
 * @brief File Reader Plugin for Mini MSP Agent
 * 
 * Provides comprehensive file reading capabilities including text files,
 * binary files, and various encoding support.
 * 
 * @author Mini MSP Agent Team
 * @version 1.0.0
 * @date 2026
 */

#include "../include/plugin_interface.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

#ifdef _WIN32
#include <windows.h>
#include <io.h>
#include <fcntl.h>
#else
#include <unistd.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <errno.h>
#endif

// Plugin information
static plugin_info_t file_reader_plugin_info = {
    .name = "file_reader",
    .version = "1.0.0",
    .description = "Reads files with support for various encodings and formats"
};

/**
 * @brief File metadata structure
 */
typedef struct {
    char path[512];
    char name[256];
    uint64_t size;
    uint64_t modified_time;
    uint64_t created_time;
    uint32_t permissions;
    bool is_readable;
    bool is_writable;
    bool is_executable;
    char encoding[32];
} file_metadata_t;

/**
 * @brief File read options structure
 */
typedef struct {
    uint64_t offset;
    uint64_t max_size;
    char encoding[32];
    bool binary_mode;
    bool include_metadata;
} file_read_options_t;

// Plugin implementation
static bool file_reader_init(void) {
    return true;
}

static void file_reader_cleanup(void) {
    // No cleanup needed
}

static plugin_info_t* file_reader_get_plugin_info(void) {
    return &file_reader_plugin_info;
}

static bool file_reader_get_system_metrics(system_metrics_t* metrics) {
    // Not applicable for file reader plugin
    return false;
}

static bool file_reader_get_processes(process_info_t** processes, size_t* count) {
    // Not applicable for file reader plugin
    return false;
}

static bool file_reader_execute_command(const char* command, command_result_t* result) {
    // Not applicable for file reader plugin
    return false;
}

static bool file_reader_get_system_info(system_info_t* info) {
    // Not applicable for file reader plugin
    return false;
}

/**
 * @brief Read file with options
 */
static bool read_file_with_options(const char* path, file_content_t* content, const file_read_options_t* options) {
    if (!path || !content) return false;
    
    memset(content, 0, sizeof(file_content_t));
    
#ifdef _WIN32
    HANDLE hFile = CreateFileA(path, GENERIC_READ, FILE_SHARE_READ, NULL, 
                             OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    if (hFile == INVALID_HANDLE_VALUE) {
        DWORD error = GetLastError();
        snprintf(content->error, sizeof(content->error), "Cannot open file: %lu", error);
        return false;
    }
    
    // Get file size
    LARGE_INTEGER fileSize;
    if (!GetFileSizeEx(hFile, &fileSize)) {
        CloseHandle(hFile);
        snprintf(content->error, sizeof(content->error), "Cannot get file size");
        return false;
    }
    
    // Apply size limits
    uint64_t readSize = fileSize.QuadPart;
    if (options) {
        if (options->max_size > 0 && readSize > options->max_size) {
            readSize = options->max_size;
        }
        if (options->offset > 0 && options->offset < fileSize.QuadPart) {
            LARGE_INTEGER li;
            li.QuadPart = options->offset;
            SetFilePointerEx(hFile, li, NULL, FILE_BEGIN);
            readSize = fileSize.QuadPart - options->offset;
            if (options->max_size > 0 && readSize > options->max_size) {
                readSize = options->max_size;
            }
        }
    }
    
    // Allocate buffer
    content->content = (char*)malloc(readSize + 1);
    if (!content->content) {
        CloseHandle(hFile);
        snprintf(content->error, sizeof(content->error), "Memory allocation failed");
        return false;
    }
    
    // Read file
    DWORD bytesRead;
    if (!ReadFile(hFile, content->content, (DWORD)readSize, &bytesRead, NULL)) {
        DWORD error = GetLastError();
        CloseHandle(hFile);
        free(content->content);
        content->content = NULL;
        snprintf(content->error, sizeof(content->error), "Read failed: %lu", error);
        return false;
    }
    
    CloseHandle(hFile);
    
    content->size = bytesRead;
    content->content[bytesRead] = '\0'; // Null terminate for text mode
    content->success = true;
    
#else
    int fd = open(path, O_RDONLY);
    if (fd == -1) {
        snprintf(content->error, sizeof(content->error), "Cannot open file: %s", strerror(errno));
        return false;
    }
    
    // Get file size
    struct stat st;
    if (fstat(fd, &st) == -1) {
        close(fd);
        snprintf(content->error, sizeof(content->error), "Cannot get file size: %s", strerror(errno));
        return false;
    }
    
    // Apply size limits
    uint64_t readSize = st.st_size;
    if (options) {
        if (options->max_size > 0 && readSize > options->max_size) {
            readSize = options->max_size;
        }
        if (options->offset > 0 && options->offset < st.st_size) {
            lseek(fd, options->offset, SEEK_SET);
            readSize = st.st_size - options->offset;
            if (options->max_size > 0 && readSize > options->max_size) {
                readSize = options->max_size;
            }
        }
    }
    
    // Allocate buffer
    content->content = (char*)malloc(readSize + 1);
    if (!content->content) {
        close(fd);
        snprintf(content->error, sizeof(content->error), "Memory allocation failed");
        return false;
    }
    
    // Read file
    ssize_t bytesRead = read(fd, content->content, readSize);
    if (bytesRead == -1) {
        close(fd);
        free(content->content);
        content->content = NULL;
        snprintf(content->error, sizeof(content->error), "Read failed: %s", strerror(errno));
        return false;
    }
    
    close(fd);
    
    content->size = bytesRead;
    content->content[bytesRead] = '\0'; // Null terminate for text mode
    content->success = true;
#endif
    
    return true;
}

/**
 * @brief Read file (simplified interface)
 */
static bool file_reader_read_file(const char* path, file_content_t* content) {
    return read_file_with_options(path, content, NULL);
}

/**
 * @brief Get file metadata
 */
static bool get_file_metadata(const char* path, file_metadata_t* metadata) {
    if (!path || !metadata) return false;
    
    memset(metadata, 0, sizeof(file_metadata_t));
    strncpy(metadata->path, path, sizeof(metadata->path) - 1);
    
    // Extract filename from path
    const char* filename = strrchr(path, '/');
    if (!filename) filename = strrchr(path, '\\');
    if (filename) {
        filename++;
    } else {
        filename = path;
    }
    strncpy(metadata->name, filename, sizeof(metadata->name) - 1);
    
#ifdef _WIN32
    WIN32_FILE_ATTRIBUTE_DATA fileData;
    if (!GetFileAttributesExA(path, GetFileExInfoStandard, &fileData)) {
        return false;
    }
    
    metadata->size = ((uint64_t)fileData.nFileSizeHigh << 32) | fileData.nFileSizeLow;
    metadata->is_readable = !(fileData.dwFileAttributes & FILE_ATTRIBUTE_READONLY);
    metadata->is_writable = !(fileData.dwFileAttributes & FILE_ATTRIBUTE_READONLY);
    metadata->is_executable = false; // Windows doesn't have simple executable flag
    
    // Convert FILETIME to Unix timestamp
    ULARGE_INTEGER ull;
    ull.LowPart = fileData.ftLastWriteTime.dwLowDateTime;
    ull.HighPart = fileData.ftLastWriteTime.dwHighDateTime;
    metadata->modified_time = (ull.QuadPart - 116444736000000000ULL) / 10000000ULL;
    
    ull.LowPart = fileData.ftCreationTime.dwLowDateTime;
    ull.HighPart = fileData.ftCreationTime.dwHighDateTime;
    metadata->created_time = (ull.QuadPart - 116444736000000000ULL) / 10000000ULL;
    
    metadata->permissions = fileData.dwFileAttributes;
    
#else
    struct stat st;
    if (stat(path, &st) == -1) {
        return false;
    }
    
    metadata->size = st.st_size;
    metadata->modified_time = st.st_mtime;
    metadata->created_time = st.st_ctime;
    metadata->permissions = st.st_mode;
    
    metadata->is_readable = (st.st_mode & S_IRUSR) != 0;
    metadata->is_writable = (st.st_mode & S_IWUSR) != 0;
    metadata->is_executable = (st.st_mode & S_IXUSR) != 0;
#endif
    
    // Detect encoding (simplified - just check for UTF-8 BOM)
    FILE* fp = fopen(path, "rb");
    if (fp) {
        unsigned char bom[3];
        size_t bytesRead = fread(bom, 1, 3, fp);
        fclose(fp);
        
        if (bytesRead >= 3 && bom[0] == 0xEF && bom[1] == 0xBB && bom[2] == 0xBF) {
            strcpy(metadata->encoding, "UTF-8");
        } else if (bytesRead >= 2 && bom[0] == 0xFF && bom[1] == 0xFE) {
            strcpy(metadata->encoding, "UTF-16LE");
        } else if (bytesRead >= 2 && bom[0] == 0xFE && bom[1] == 0xFF) {
            strcpy(metadata->encoding, "UTF-16BE");
        } else {
            strcpy(metadata->encoding, "ASCII/UTF-8");
        }
    } else {
        strcpy(metadata->encoding, "Unknown");
    }
    
    return true;
}

/**
 * @brief Read file lines
 */
static bool read_file_lines(const char* path, char*** lines, size_t* line_count, uint32_t max_lines) {
    if (!path || !lines || !line_count) return false;
    
    file_content_t content;
    if (!file_reader_read_file(path, &content)) {
        return false;
    }
    
    if (!content.success || !content.content) {
        return false;
    }
    
    // Count lines
    size_t count = 0;
    char* ptr = content.content;
    while (*ptr && count < max_lines) {
        if (*ptr == '\n') {
            count++;
        }
        ptr++;
    }
    
    // Allocate array for line pointers
    *lines = (char**)malloc(count * sizeof(char*));
    if (!*lines) {
        free(content.content);
        return false;
    }
    
    // Split into lines
    char* line_start = content.content;
    size_t index = 0;
    ptr = content.content;
    
    while (*ptr && index < count) {
        if (*ptr == '\n') {
            *ptr = '\0'; // Terminate line
            (*lines)[index] = line_start;
            line_start = ptr + 1;
            index++;
        }
        ptr++;
    }
    
    // Handle last line if no trailing newline
    if (*line_start && index < count) {
        (*lines)[index] = line_start;
        index++;
    }
    
    *line_count = index;
    return true;
}

static void file_reader_free_memory(void* ptr) {
    if (ptr) {
        free(ptr);
    }
}

// Plugin interface
static plugin_interface_t file_reader_interface = {
    .get_plugin_info = file_reader_get_plugin_info,
    .init = file_reader_init,
    .cleanup = file_reader_cleanup,
    .get_system_metrics = file_reader_get_system_metrics,
    .get_processes = file_reader_get_processes,
    .execute_command = file_reader_execute_command,
    .read_file = file_reader_read_file,
    .get_system_info = file_reader_get_system_info,
    .free_memory = file_reader_free_memory
};

// Plugin entry point
PLUGIN_EXPORT plugin_interface_t* PLUGIN_CALL get_plugin_interface(void) {
    return &file_reader_interface;
}
