#ifndef TYPE_ALIASES_H
#define TYPE_ALIASES_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// =============================================================================
// SEMANTIC TYPE ALIASES
// =============================================================================
//
// This header defines semantic type aliases for domain-driven design.
// Each alias encodes the meaning of a value, not just its storage type.
// This improves code readability, maintainability, and FFI safety.
//
// Naming convention: <domain>_<meaning>_t
// Example: file_size_t, timestamp_t, error_count_t
// =============================================================================

// -----------------------------------------------------------------------------
// Size and Capacity Types
// -----------------------------------------------------------------------------

typedef uint64_t FileSize;           // Size in bytes (semantic alias)
typedef uint64_t BufferSize;         // Buffer capacity in bytes (semantic alias)
typedef uint64_t MemorySize;         // Memory usage in bytes (semantic alias)
typedef uint64_t DiskSize;           // Disk usage in bytes (semantic alias)

// -----------------------------------------------------------------------------
// Count and Quantity Types
// -----------------------------------------------------------------------------

typedef uint32_t FileCount;          // Number of files (semantic alias)
typedef uint32_t DirectoryCount;     // Number of directories (semantic alias)
typedef uint32_t CallCount;          // Number of function calls (semantic alias)
typedef uint32_t ErrorCount;        // Number of errors (semantic alias)
typedef uint32_t ProcessCount;       // Number of processes (semantic alias)
typedef uint32_t WatcherCount;       // Number of file watchers (semantic alias)
typedef uint32_t NotificationCount;  // Number of notifications (semantic alias)
typedef uint32_t ItemCount;          // Generic item count (semantic alias)
typedef uint32_t DepthLevel;         // Directory depth level (semantic alias)

// -----------------------------------------------------------------------------
// Time and Timestamp Types
// -----------------------------------------------------------------------------

typedef uint64_t Timestamp;          // Unix timestamp in milliseconds (semantic alias)
typedef uint64_t Uptime;             // System uptime in seconds (semantic alias)
typedef uint64_t Duration;          // Time duration in milliseconds (semantic alias)

// -----------------------------------------------------------------------------
// Identifier Types
// -----------------------------------------------------------------------------

typedef uint32_t ProcessId;          // Process identifier (semantic alias)
typedef uint32_t Width;              // Pixel width (semantic alias)
typedef uint32_t Height;             // Pixel height (semantic alias)
typedef uint32_t FrameRate;          // Frames per second (semantic alias)

// -----------------------------------------------------------------------------
// Percentage and Ratio Types
// -----------------------------------------------------------------------------

typedef uint8_t Percentage;         // Percentage value 0-100 (semantic alias)
typedef float CpuUsage;             // CPU usage as percentage (semantic alias)
typedef float RamUsage;             // RAM usage as percentage (semantic alias)
typedef float DiskUsage;            // Disk usage as percentage (semantic alias)
typedef float LoadIndex;            // System load index (semantic alias)

// -----------------------------------------------------------------------------
// Length and String Types
// -----------------------------------------------------------------------------

typedef uint16_t PathLength;         // Path string length (semantic alias)
typedef uint16_t StringLength;       // Generic string length (semantic alias)

// -----------------------------------------------------------------------------
// String Pointer Types (const char* with semantic meaning)
// -----------------------------------------------------------------------------

typedef const char* PathString;      // File/directory path (readonly, semantic alias)
typedef const char* ErrorMessage;    // Error message (readonly, semantic alias)
typedef const char* PluginName;      // Plugin name (readonly, semantic alias)
typedef const char* PluginVersion;   // Plugin version (readonly, semantic alias)
typedef const char* PluginDescription; // Plugin description (readonly, semantic alias)
typedef const char* CommandName;     // Command name (readonly, semantic alias)
typedef const char* ConfigPath;      // Configuration file path (readonly, semantic alias)
typedef const char* Hostname;        // System hostname (readonly, semantic alias)
typedef const char* OsType;          // Operating system type (readonly, semantic alias)
typedef const char* OsVersion;       // Operating system version (readonly, semantic alias)
typedef const char* Encoding;        // Text encoding (readonly, semantic alias)
typedef const char* CodecName;       // Video codec name (readonly, semantic alias)
typedef const char* Status;          // Status message (readonly, semantic alias)

// -----------------------------------------------------------------------------
// Mutable String Pointer Types (char* with semantic meaning)
// -----------------------------------------------------------------------------

typedef char* MutableString;         // Mutable string buffer (semantic alias)

// -----------------------------------------------------------------------------
// Boolean and Flag Types
// -----------------------------------------------------------------------------

typedef bool IsHidden;              // Hidden file flag (semantic alias)
typedef bool IsDirectory;           // Directory flag (semantic alias)
typedef bool IsLocked;              // Locked file flag (semantic alias)
typedef bool IsDynamic;             // Dynamic allocation flag (semantic alias)
typedef bool AutoReload;            // Auto-reload configuration flag (semantic alias)
typedef bool Recursive;             // Recursive operation flag (semantic alias)

// -----------------------------------------------------------------------------
// Result and Status Types
// -----------------------------------------------------------------------------

typedef int32_t ErrorCode;           // Error code (semantic alias)
typedef int32_t ExitCode;            // Process exit code (semantic alias)

#ifdef __cplusplus
}
#endif

#endif // TYPE_ALIASES_H
