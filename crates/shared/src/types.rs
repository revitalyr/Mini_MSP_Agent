//! # Semantic Type Aliases
//!
//! This module defines semantic type aliases for domain-driven design.
//! Each alias encodes the meaning of a value, not just its storage type.
//! This improves code readability, maintainability, and FFI safety.
//!
//! Type names match the C aliases in `plugins/include/type_aliases.h`
//! for consistent FFI boundary usage.

// -----------------------------------------------------------------------------
// Size and Capacity Types
// -----------------------------------------------------------------------------

/// Size in bytes (semantic alias)
pub type FileSize = u64;

/// Buffer capacity in bytes (semantic alias)
pub type BufferSize = u64;

/// Memory usage in bytes (semantic alias)
pub type MemorySize = u64;

/// Disk usage in bytes (semantic alias)
pub type DiskSize = u64;

// -----------------------------------------------------------------------------
// Count and Quantity Types
// -----------------------------------------------------------------------------

/// Number of files (semantic alias)
pub type FileCount = u32;

/// Number of directories (semantic alias)
pub type DirectoryCount = u32;

/// Number of function calls (semantic alias)
pub type CallCount = u32;

/// Number of errors (semantic alias)
pub type ErrorCount = u32;

/// Number of processes (semantic alias)
pub type ProcessCount = u32;

/// Number of file watchers (semantic alias)
pub type WatcherCount = u32;

/// Number of notifications (semantic alias)
pub type NotificationCount = u32;

/// Generic item count (semantic alias)
pub type ItemCount = u32;

/// Count of objects returned from a plugin or query
pub type ObjectCount = u32;

/// Directory depth level (semantic alias)
pub type DepthLevel = u32;

// -----------------------------------------------------------------------------
// Time and Timestamp Types
// -----------------------------------------------------------------------------

/// Unix timestamp in milliseconds (semantic alias)
pub type Timestamp = u64;

/// System uptime in seconds (semantic alias)
pub type Uptime = u64;

/// Time duration in milliseconds (semantic alias)
pub type Duration = u64;

// -----------------------------------------------------------------------------
// Identifier Types
// -----------------------------------------------------------------------------

/// Process identifier (semantic alias)
pub type ProcessId = u32;

/// Pixel width (semantic alias)
pub type Width = u32;

/// Pixel height (semantic alias)
pub type Height = u32;

/// Frames per second (semantic alias)
pub type FrameRate = u32;

// -----------------------------------------------------------------------------
// Percentage and Ratio Types
// -----------------------------------------------------------------------------

/// Percentage value 0-100 (semantic alias)
pub type Percentage = u8;

/// CPU usage as percentage (semantic alias)
pub type CpuUsage = f32;

/// RAM usage as percentage (semantic alias)
pub type RamUsage = f32;

/// Disk usage as percentage (semantic alias)
pub type DiskUsage = f32;

/// System load index (semantic alias)
pub type LoadIndex = f32;

// -----------------------------------------------------------------------------
// Length and String Types
// -----------------------------------------------------------------------------

/// Path string length (semantic alias)
pub type PathLength = u16;

/// Generic string length (semantic alias)
pub type StringLength = u16;

// -----------------------------------------------------------------------------
// Result and Status Types
// -----------------------------------------------------------------------------

/// Error code (semantic alias)
pub type ErrorCode = i32;

/// Process exit code (semantic alias)
pub type ExitCode = i32;
