#ifndef FILE_READER_PLATFORM_H
#define FILE_READER_PLATFORM_H

#include "../include/plugin_interface_common.h"
#include "../include/semantic_types.h"

// File metadata structure for platform-specific operations
typedef struct {
    char m_path[512];
    char m_name[256];
    file_size_t m_size_bytes;
    timestamp_t m_modification_time;
    timestamp_t m_creation_time;
    char m_permissions[16];
    bool m_is_readable;
    bool m_is_writable;
    bool m_is_executable;
    bool m_is_hidden;
    bool m_is_directory;
} file_metadata_t;

// Platform-specific function declarations
#ifdef _WIN32
plugin_result_t windows_read_file_content(const char* path, char** content, file_size_t* size);
plugin_result_t windows_get_file_metadata(const char* path, file_metadata_t* metadata);
bool windows_file_exists(const char* path);
bool windows_is_directory(const char* path);
#define platform_read_file_content windows_read_file_content
#define platform_get_file_metadata windows_get_file_metadata
#define platform_file_exists windows_file_exists
#define platform_is_directory windows_is_directory
#else
plugin_result_t linux_read_file_content(const char* path, char** content, file_size_t* size);
plugin_result_t linux_get_file_metadata(const char* path, file_metadata_t* metadata);
bool linux_file_exists(const char* path);
bool linux_is_directory(const char* path);
#define platform_read_file_content linux_read_file_content
#define platform_get_file_metadata linux_get_file_metadata
#define platform_file_exists linux_file_exists
#define platform_is_directory linux_is_directory
#endif

#endif // FILE_READER_PLATFORM_H
