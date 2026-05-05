//! Semantic Type Aliases for FFI Safety and Domain-Driven Design
//!
//! This module defines semantic type aliases that encode the meaning of values,
//! not just their storage type. This improves code readability, maintainability,
//! and FFI safety by making domain concepts explicit in the type system.
//!
//! These types mirror the C type_aliases.h header for consistent FFI boundaries.

use std::ffi::{c_char, c_ulonglong, c_void, CStr};

// =============================================================================
// SIZE AND CAPACITY TYPES
// =============================================================================

/// File size in bytes (semantic alias for u64)
pub type FileSize = u64;

/// Buffer capacity in bytes (semantic alias for u64)
pub type BufferSize = u64;

/// Memory usage in bytes (semantic alias for u64)
pub type MemorySize = u64;

/// Disk usage in bytes (semantic alias for u64)
pub type DiskSize = u64;

// =============================================================================
// COUNT AND QUANTITY TYPES
// =============================================================================

/// Number of files (semantic alias for u32)
pub type FileCount = u32;

/// Number of directories (semantic alias for u32)
pub type DirectoryCount = u32;

/// Number of function calls (semantic alias for u32)
pub type CallCount = u32;

/// Number of errors (semantic alias for u32)
pub type ErrorCount = u32;

/// Number of processes (semantic alias for u32)
pub type ProcessCount = u32;

/// Number of file watchers (semantic alias for u32)
pub type WatcherCount = u32;

/// Number of notifications (semantic alias for u32)
pub type NotificationCount = u32;

/// Generic item count (semantic alias for u32)
pub type ItemCount = u32;

/// Directory depth level (semantic alias for u32)
pub type DepthLevel = u32;

/// Object count for collections (semantic alias for usize)
pub type ObjectCount = usize;

// =============================================================================
// TIME AND TIMESTAMP TYPES
// =============================================================================

/// Unix timestamp in milliseconds (semantic alias for u64)
pub type Timestamp = u64;

/// System uptime in seconds (semantic alias for u64)
pub type Uptime = u64;

/// Time duration in milliseconds (semantic alias for u64)
pub type Duration = u64;

// =============================================================================
// IDENTIFIER TYPES
// =============================================================================

/// Process identifier (semantic alias for u32)
pub type ProcessId = u32;

/// User identifier (semantic alias for u64)
pub type UserId = u64;

/// Sequence number for ordering (semantic alias for u64)
pub type SequenceNumber = u64;

/// Pixel width (semantic alias for u32)
pub type Width = u32;

/// Pixel height (semantic alias for u32)
pub type Height = u32;

/// Frames per second (semantic alias for u32)
pub type FrameRate = u32;

// =============================================================================
// PERCENTAGE AND RATIO TYPES
// =============================================================================

/// Percentage value 0-100 (semantic alias for u8)
pub type Percentage = u8;

/// CPU usage as percentage 0.0-100.0 (semantic alias for f32)
pub type CpuUsage = f32;

/// RAM usage as percentage 0.0-100.0 (semantic alias for f32)
pub type RamUsage = f32;

/// Disk usage as percentage 0.0-100.0 (semantic alias for f32)
pub type DiskUsage = f32;

/// System load index (semantic alias for f32)
pub type LoadIndex = f32;

// =============================================================================
// LENGTH AND STRING TYPES
// =============================================================================

/// Path string length (semantic alias for u16)
pub type PathLength = u16;

/// Generic string length (semantic alias for u16)
pub type StringLength = u16;

/// UTF-8 string slice (readonly, semantic alias for C string)
pub type Utf8Slice = *const c_char;

/// Mutable string buffer (semantic alias for mutable C string)
pub type StringBuffer = *mut c_char;

/// File path string (readonly, semantic alias)
pub type FilePath = *const c_char;

/// Error message string (readonly, semantic alias)
pub type ErrorMessage = *const c_char;

/// Plugin name string (readonly, semantic alias)
pub type PluginName = *const c_char;

/// Plugin version string (readonly, semantic alias)
pub type PluginVersion = *const c_char;

/// Command name string (readonly, semantic alias)
pub type CommandName = *const c_char;

/// Hostname string (readonly, semantic alias)
pub type Hostname = *const c_char;

/// OS type string (readonly, semantic alias)
pub type OsType = *const c_char;

/// OS version string (readonly, semantic alias)
pub type OsVersion = *const c_char;

/// Configuration path string (readonly, semantic alias)
pub type ConfigPath = *const c_char;

/// Encoding name string (readonly, semantic alias)
pub type Encoding = *const c_char;

/// Codec name string (readonly, semantic alias)
pub type CodecName = *const c_char;

/// Status message string (readonly, semantic alias)
pub type Status = *const c_char;

// =============================================================================
// BOOLEAN AND FLAG TYPES
// =============================================================================

/// Hidden file flag (semantic alias for bool)
pub type IsHidden = bool;

/// Directory flag (semantic alias for bool)
pub type IsDirectory = bool;

/// Locked file flag (semantic alias for bool)
pub type IsLocked = bool;

/// Dynamic allocation flag (semantic alias for bool)
pub type IsDynamic = bool;

/// Auto-reload configuration flag (semantic alias for bool)
pub type AutoReload = bool;

/// Recursive operation flag (semantic alias for bool)
pub type Recursive = bool;

// =============================================================================
// RESULT AND STATUS TYPES
// =============================================================================

/// Error code (semantic alias for i32)
pub type ErrorCode = i32;

/// Exit code (semantic alias for i32)
pub type ExitCode = i32;

/// Plugin initialization result
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitResult {
    Success = 0,
    Failure = 1,
    AlreadyInitialized = 2,
}

/// Command execution result
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandResult {
    Success = 0,
    Failure = 1,
    NotSupported = 2,
    InvalidParams = 3,
}

// =============================================================================
// CONSTANTS
// =============================================================================

/// Maximum length for permission string
pub const MAX_PERMISSION_LEN: usize = 16;

/// Maximum length for event message
pub const MAX_EVENT_MSG_LEN: usize = 64;

/// Maximum length for codec name
pub const MAX_CODEC_NAME_LEN: usize = 16;

/// Maximum length for encoding name
pub const MAX_ENCODING_LEN: usize = 32;

/// Maximum length for status message
pub const MAX_STATUS_LEN: usize = 64;

/// Maximum length for category name
pub const MAX_CATEGORY_LEN: usize = 64;

/// Maximum length for artifact type name
pub const MAX_ARTIFACT_TYPE_LEN: usize = 64;

/// Maximum length for file pattern
pub const MAX_PATTERN_LEN: usize = 256;

/// Maximum length for file path
pub const MAX_PATH_LEN: usize = 512;

/// Maximum length for hostname
pub const MAX_HOSTNAME_LEN: usize = 256;

/// Maximum length for OS type
pub const MAX_OS_TYPE_LEN: usize = 64;

/// Maximum length for OS version
pub const MAX_OS_VERSION_LEN: usize = 128;

/// Maximum length for error message
pub const MAX_ERROR_MSG_LEN: usize = 512;

/// Maximum length for forensic finding details
pub const MAX_DETAILS_LEN: usize = 1024;

/// Maximum length for command output
pub const MAX_COMMAND_OUTPUT_LEN: usize = 512;

/// Default maximum directory scan depth
pub const DEFAULT_MAX_DEPTH: u32 = 10;

/// Default scan interval in seconds
pub const DEFAULT_SCAN_INTERVAL_SEC: u32 = 5;

/// Maximum memory usage in MB
pub const MAX_MEMORY_USAGE_MB: u32 = 512;

/// Default heartbeat interval in seconds
pub const DEFAULT_HEARTBEAT_SEC: u32 = 30;

/// Default text encoding
pub const DEFAULT_ENCODING: &str = "utf-8";

/// Default video codec
pub const DEFAULT_CODEC: &str = "h264";

/// Default status message
pub const DEFAULT_STATUS: &str = "ok";

/// Error status string
pub const ERROR_STATUS: &str = "error";

/// Success status string
pub const SUCCESS_STATUS: &str = "success";

/// Plugin API version
pub const PLUGIN_API_VERSION: &str = "2.0.0";

/// Plugin interface version
pub const PLUGIN_INTERFACE_VERSION: u32 = 1;

/// Invalid process ID marker
pub const INVALID_PID: ProcessId = 0;

/// Invalid timestamp marker
pub const INVALID_TIMESTAMP: Timestamp = 0;

/// Maximum percentage value
pub const MAX_PERCENTAGE: Percentage = 100;

/// Minimum percentage value
pub const MIN_PERCENTAGE: Percentage = 0;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Convert a C string pointer to a Rust string slice safely
pub unsafe fn c_str_to_str(ptr: *const c_char) -> Option<&'static str> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok()
}

/// Convert a C string pointer to a lossy Rust String
pub unsafe fn c_str_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    CStr::from_ptr(ptr).to_string_lossy().into_owned()
}

/// Check if a timestamp is valid (non-zero)
pub fn is_valid_timestamp(ts: Timestamp) -> bool {
    ts != INVALID_TIMESTAMP
}

/// Check if a process ID is valid (non-zero)
pub fn is_valid_pid(pid: ProcessId) -> bool {
    pid != INVALID_PID
}

/// Clamp a percentage value to valid range 0-100
pub fn clamp_percentage(p: Percentage) -> Percentage {
    p.clamp(MIN_PERCENTAGE, MAX_PERCENTAGE)
}

/// Format a file size in human-readable format
pub fn format_file_size(size: FileSize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.2} {}", size, UNITS[unit_index])
}

/// Format a timestamp as ISO 8601 string (if chrono feature available)
pub fn format_timestamp(ts: Timestamp) -> String {
    // Simple formatting without chrono dependency
    format!("{} ms since epoch", ts)
}
