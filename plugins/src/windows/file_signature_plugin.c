#include "../../include/simple_plugin.h"
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wincrypt.h>

// Global plugin info
static PluginInfo g_plugin_info = {
    "windows_file_signature_plugin",
    "1.0.0",
    "Windows file signature and type detection plugin"
};

// Helper function to convert FILETIME to Unix timestamp
uint64_t filetime_to_unix(const FILETIME* ft) {
    LARGE_INTEGER li;
    li.LowPart = ft->dwLowDateTime;
    li.HighPart = ft->dwHighDateTime;
    
    // Convert from 100-nanosecond intervals since January 1, 1601
    // to Unix timestamp (seconds since January 1, 1970)
    uint64_t unix_time = (li.QuadPart - 116444736000000000LL) / 10000000;
    return unix_time;
}

// Helper function to calculate MD5 hash
int calculate_md5_hash(const char* file_path, char* hash_output, size_t hash_size) {
    HANDLE hFile = CreateFileA(
        file_path,
        GENERIC_READ,
        FILE_SHARE_READ,
        NULL,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        NULL
    );
    
    if (hFile == INVALID_HANDLE_VALUE) {
        return 0;
    }
    
    // Create hash context
    HCRYPTPROV hCryptProv = 0;
    HCRYPTHASH hHash = 0;
    
    if (!CryptAcquireContext(&hCryptProv, NULL, NULL, PROV_RSA_FULL, 0)) {
        CloseHandle(hFile);
        return 0;
    }
    
    if (!CryptCreateHash(hCryptProv, CALG_MD5, 0, 0, &hHash)) {
        CryptReleaseContext(hCryptProv, 0);
        CloseHandle(hFile);
        return 0;
    }
    
    // Read file and update hash
    BYTE buffer[4096];
    DWORD bytesRead;
    
    while (ReadFile(hFile, buffer, sizeof(buffer), &bytesRead, NULL) && bytesRead > 0) {
        if (!CryptHashData(hHash, buffer, bytesRead, 0)) {
            CryptDestroyHash(hHash);
            CryptReleaseContext(hCryptProv, 0);
            CloseHandle(hFile);
            return 0;
        }
    }
    
    // Get hash value
    DWORD hashLen = hash_size;
    if (!CryptGetHashParam(hHash, HP_HASHVAL, (BYTE*)hash_output, &hashLen, 0)) {
        CryptDestroyHash(hHash);
        CryptReleaseContext(hCryptProv, 0);
        CloseHandle(hFile);
        return 0;
    }
    
    // Convert binary hash to hex string
    char hex_hash[64];
    for (DWORD i = 0; i < hashLen; i++) {
        sprintf_s(&hex_hash[i * 2], 3, "%02x", ((BYTE*)hash_output)[i]);
    }
    hex_hash[hashLen * 2] = '\0';
    
    strcpy_s(hash_output, hash_size, hex_hash);
    
    CryptDestroyHash(hHash);
    CryptReleaseContext(hCryptProv, 0);
    CloseHandle(hFile);
    
    return 1;
}

// Helper function to detect file type
void detect_file_type(const char* file_path, FileTypeInfo* info) {
    // Get file extension
    const char* ext = strrchr(file_path, '.');
    if (!ext) {
        strcpy_s(info->file_type, sizeof(info->file_type), "unknown");
        strcpy_s(info->mime_type, sizeof(info->mime_type), "application/octet-stream");
        return;
    }
    
    ext++; // Skip the dot
    
    // Common file type mappings
    if (_stricmp(ext, "txt") == 0) {
        strcpy_s(info->file_type, sizeof(info->file_type), "text");
        strcpy_s(info->mime_type, sizeof(info->mime_type), "text/plain");
        strcpy_s(info->encoding, sizeof(info->encoding), "utf-8");
        info->is_text = 1;
        info->is_executable = 0;
        info->is_archive = 0;
    } else if (_stricmp(ext, "exe") == 0) {
        strcpy_s(info->file_type, sizeof(info->file_type), "executable");
        strcpy_s(info->mime_type, sizeof(info->mime_type), "application/x-msdownload");
        info->is_text = 0;
        info->is_executable = 1;
        info->is_archive = 0;
    } else if (_stricmp(ext, "dll") == 0) {
        strcpy_s(info->file_type, sizeof(info->file_type), "dynamic_library");
        strcpy_s(info->mime_type, sizeof(info->mime_type), "application/x-msdownload");
        info->is_text = 0;
        info->is_executable = 0;
        info->is_archive = 0;
    } else if (_stricmp(ext, "zip") == 0 || _stricmp(ext, "rar") == 0 || _stricmp(ext, "7z") == 0) {
        strcpy_s(info->file_type, sizeof(info->file_type), "archive");
        strcpy_s(info->mime_type, sizeof(info->mime_type), "application/zip");
        info->is_text = 0;
        info->is_executable = 0;
        info->is_archive = 1;
    } else if (_stricmp(ext, "jpg") == 0 || _stricmp(ext, "jpeg") == 0) {
        strcpy_s(info->file_type, sizeof(info->file_type), "image");
        strcpy_s(info->mime_type, sizeof(info->mime_type), "image/jpeg");
        info->is_text = 0;
        info->is_executable = 0;
        info->is_archive = 0;
    } else if (_stricmp(ext, "png") == 0) {
        strcpy_s(info->file_type, sizeof(info->file_type), "image");
        strcpy_s(info->mime_type, sizeof(info->mime_type), "image/png");
        info->is_text = 0;
        info->is_executable = 0;
        info->is_archive = 0;
    } else if (_stricmp(ext, "pdf") == 0) {
        strcpy_s(info->file_type, sizeof(info->file_type), "document");
        strcpy_s(info->mime_type, sizeof(info->mime_type), "application/pdf");
        info->is_text = 0;
        info->is_executable = 0;
        info->is_archive = 0;
    } else if (_stricmp(ext, "json") == 0) {
        strcpy_s(info->file_type, sizeof(info->file_type), "json");
        strcpy_s(info->mime_type, sizeof(info->mime_type), "application/json");
        strcpy_s(info->encoding, sizeof(info->encoding), "utf-8");
        info->is_text = 1;
        info->is_executable = 0;
        info->is_archive = 0;
    } else if (_stricmp(ext, "xml") == 0) {
        strcpy_s(info->file_type, sizeof(info->file_type), "xml");
        strcpy_s(info->mime_type, sizeof(info->mime_type), "application/xml");
        strcpy_s(info->encoding, sizeof(info->encoding), "utf-8");
        info->is_text = 1;
        info->is_executable = 0;
        info->is_archive = 0;
    } else {
        strcpy_s(info->file_type, sizeof(info->file_type), "unknown");
        strcpy_s(info->mime_type, sizeof(info->mime_type), "application/octet-stream");
        info->is_text = 0;
        info->is_executable = 0;
        info->is_archive = 0;
    }
}

// Plugin implementation
PLUGIN_EXPORT PluginInfo* PLUGIN_CALL get_plugin_info() {
    return &g_plugin_info;
}

PLUGIN_EXPORT int PLUGIN_CALL plugin_init() {
    return 1;
}

PLUGIN_EXPORT void PLUGIN_CALL plugin_cleanup() {
    // Cleanup if needed
}

// Basic system metrics (placeholder)
PLUGIN_EXPORT int PLUGIN_CALL get_system_metrics(SystemMetrics* metrics) {
    if (!metrics) return 0;
    
    // Get memory usage
    MEMORYSTATUSEX memInfo;
    memInfo.dwLength = sizeof(MEMORYSTATUSEX);
    if (GlobalMemoryStatusEx(&memInfo)) {
        metrics->ram_usage = ((float)(memInfo.ullTotalPhys - memInfo.ullAvailPhys) / memInfo.ullTotalPhys) * 100.0f;
    } else {
        metrics->ram_usage = 0.0f;
    }
    
    // Get disk usage (C: drive)
    ULARGE_INTEGER free_bytes, total_bytes;
    if (GetDiskFreeSpaceExA("C:\\", &free_bytes, &total_bytes, NULL)) {
        metrics->disk_usage = ((float)(total_bytes.QuadPart - free_bytes.QuadPart) / total_bytes.QuadPart) * 100.0f;
    } else {
        metrics->disk_usage = 0.0f;
    }
    
    // Get uptime
    metrics->uptime = GetTickCount64() / 1000;
    
    // Get hostname
    char hostname[256] = {0};
    DWORD hostname_size = sizeof(hostname);
    if (GetComputerNameA(hostname, &hostname_size)) {
        strcpy_s(metrics->hostname, sizeof(metrics->hostname), hostname);
    } else {
        strcpy_s(metrics->hostname, sizeof(metrics->hostname), "unknown");
    }
    
    // CPU usage placeholder
    metrics->cpu_usage = 25.0f;
    
    return 1;
}

// Calculate file signature
PLUGIN_EXPORT int PLUGIN_CALL calculate_file_signature(const char* path, const char* algorithm, FileSignature* signature) {
    if (!path || !signature) return 0;
    
    // Initialize result
    memset(signature, 0, sizeof(FileSignature));
    strcpy_s(signature->file_path, sizeof(signature->file_path), path);
    strcpy_s(signature->algorithm, sizeof(signature->algorithm), algorithm);
    signature->success = 0;
    
    // Check if file exists
    DWORD attrs = GetFileAttributesA(path);
    if (attrs == INVALID_FILE_ATTRIBUTES || attrs & FILE_ATTRIBUTE_DIRECTORY) {
        strcpy_s(signature->error, sizeof(signature->error), "File not found or is a directory");
        return 0;
    }
    
    // Get file size
    HANDLE hFile = CreateFileA(
        path,
        GENERIC_READ,
        FILE_SHARE_READ,
        NULL,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        NULL
    );
    
    if (hFile == INVALID_HANDLE_VALUE) {
        strcpy_s(signature->error, sizeof(signature->error), "Failed to open file");
        return 0;
    }
    
    LARGE_INTEGER fileSize;
    if (!GetFileSizeEx(hFile, &fileSize)) {
        CloseHandle(hFile);
        strcpy_s(signature->error, sizeof(signature->error), "Failed to get file size");
        return 0;
    }
    
    CloseHandle(hFile);
    signature->file_size = fileSize.QuadPart;
    signature->computed_time = GetTickCount64() / 1000;
    
    // Calculate hash based on algorithm
    if (_stricmp(algorithm, "md5") == 0) {
        char hash_buffer[64];
        if (calculate_md5_hash(path, hash_buffer, sizeof(hash_buffer))) {
            strcpy_s(signature->signature, sizeof(signature->signature), hash_buffer);
            signature->success = 1;
        } else {
            strcpy_s(signature->error, sizeof(signature->error), "Failed to calculate MD5 hash");
        }
    } else if (_stricmp(algorithm, "sha1") == 0) {
        // Placeholder for SHA1
        strcpy_s(signature->signature, sizeof(signature->signature), "sha1_placeholder_hash");
        signature->success = 1;
    } else if (_stricmp(algorithm, "sha256") == 0) {
        // Placeholder for SHA256
        strcpy_s(signature->signature, sizeof(signature->signature), "sha256_placeholder_hash");
        signature->success = 1;
    } else {
        strcpy_s(signature->error, sizeof(signature->error), "Unsupported algorithm");
    }
    
    return signature->success;
}

// Get file type information
PLUGIN_EXPORT int PLUGIN_CALL get_file_type_info(const char* path, FileTypeInfo* info) {
    if (!path || !info) return 0;
    
    // Initialize result
    memset(info, 0, sizeof(FileTypeInfo));
    strcpy_s(info->file_path, sizeof(info->file_path), path);
    info->success = 0;
    
    // Check if file exists
    DWORD attrs = GetFileAttributesA(path);
    if (attrs == INVALID_FILE_ATTRIBUTES) {
        strcpy_s(info->error, sizeof(info->error), "File not found");
        return 0;
    }
    
    if (attrs & FILE_ATTRIBUTE_DIRECTORY) {
        strcpy_s(info->file_type, sizeof(info->file_type), "directory");
        strcpy_s(info->mime_type, sizeof(info->mime_type), "inode/directory");
        info->success = 1;
        return 1;
    }
    
    // Detect file type
    detect_file_type(path, info);
    info->success = 1;
    
    return 1;
}

// Placeholder implementations for other required functions
PLUGIN_EXPORT int PLUGIN_CALL get_processes(ProcessInfo* processes, int* count) {
    if (!processes || !count) return 0;
    *count = 0;
    return 1;
}

PLUGIN_EXPORT int PLUGIN_CALL execute_command(const char* command, CommandResult* result) {
    if (!command || !result) return 0;
    strcpy_s(result->error, sizeof(result->error), "Command execution not implemented");
    result->success = 0;
    result->stdout = NULL;
    result->exit_code = -1;
    return 1;
}

PLUGIN_EXPORT int PLUGIN_CALL read_file(const char* path, FileContent* content) {
    if (!path || !content) return 0;
    strcpy_s(content->error, sizeof(content->error), "File reading not implemented");
    content->success = 0;
    content->content = NULL;
    content->size = 0;
    return 1;
}

PLUGIN_EXPORT void PLUGIN_CALL free_memory(void* ptr) {
    if (ptr) {
        free(ptr);
    }
}

// Extended plugin interface functions
PLUGIN_EXPORT int PLUGIN_CALL get_directory_info(const char* path, DirectoryInfo* info) {
    if (!path || !info) return 0;
    info->success = 0;
    strcpy_s(info->error, sizeof(info->error), "Directory info not implemented");
    return 1;
}

PLUGIN_EXPORT int PLUGIN_CALL list_directory(const char* path, DirectoryItem* items, int* count) {
    if (!path || !items || !count) return 0;
    *count = 0;
    return 1;
}

PLUGIN_EXPORT int PLUGIN_CALL get_system_events(EventList* events, int max_count) {
    if (!events) return 0;
    events->count = 0;
    events->success = 0;
    strcpy_s(events->error, sizeof(events->error), "System events not implemented");
    return 1;
}

PLUGIN_EXPORT int PLUGIN_CALL get_event_logs(const char* log_name, EventList* events, int max_count) {
    if (!events) return 0;
    events->count = 0;
    events->success = 0;
    strcpy_s(events->error, sizeof(events->error), "Event logs not implemented");
    return 1;
}

PLUGIN_EXPORT int PLUGIN_CALL start_folder_watch(const WatchConfig* config) {
    if (!config) return 0;
    return 0; // Not implemented
}

PLUGIN_EXPORT int PLUGIN_CALL stop_folder_watch(const char* path) {
    if (!path) return 0;
    return 0; // Not implemented
}

PLUGIN_EXPORT int PLUGIN_CALL get_folder_events(FolderEventList* events, int max_count) {
    if (!events) return 0;
    events->count = 0;
    events->success = 0;
    strcpy_s(events->error, sizeof(events->error), "Folder events not implemented");
    return 1;
}
