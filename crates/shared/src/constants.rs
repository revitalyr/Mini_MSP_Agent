//! # Constants and String Literals
//!
//! This module defines all hardcoded constants, string lengths, and
//! configuration values used across the plugin system.
//!
//! Constants match the C definitions in `plugins/include/constants.h`
//! for consistent FFI boundary usage.

// -----------------------------------------------------------------------------
// String Length Constants
// -----------------------------------------------------------------------------

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

/// Maximum length for file pattern
pub const MAX_PATTERN_LEN: usize = 256;

/// Maximum length for hostname
pub const MAX_HOSTNAME_LEN: usize = 256;

/// Maximum length for OS type
pub const MAX_OS_TYPE_LEN: usize = 64;

/// Maximum length for OS version
pub const MAX_OS_VERSION_LEN: usize = 128;

/// Maximum length for error message
pub const MAX_ERROR_MSG_LEN: usize = 512;

/// Maximum length for command output
pub const MAX_COMMAND_OUTPUT_LEN: usize = 512;

// -----------------------------------------------------------------------------
// Numeric Constants
// -----------------------------------------------------------------------------

/// Default maximum directory scan depth
pub const DEFAULT_MAX_DEPTH: u32 = 10;

/// Default scan interval in seconds
pub const DEFAULT_SCAN_INTERVAL: u64 = 5;

/// Maximum memory usage in MB
pub const MAX_MEMORY_USAGE_MB: u64 = 512;

/// Default heartbeat interval in seconds
pub const DEFAULT_HEARTBEAT_SEC: u64 = 30;

// -----------------------------------------------------------------------------
// String Literals
// -----------------------------------------------------------------------------

/// Default text encoding
pub const DEFAULT_ENCODING: &str = "utf-8";

/// Default video codec
pub const DEFAULT_CODEC: &str = "h264";

/// Default status message
pub const DEFAULT_STATUS: &str = "ok";

/// Error status
pub const ERROR_STATUS: &str = "error";

/// Success status
pub const SUCCESS_STATUS: &str = "success";

// -----------------------------------------------------------------------------
// Plugin Configuration Constants
// -----------------------------------------------------------------------------

/// Plugin API version
pub const PLUGIN_API_VERSION: &str = "2.0.0";

/// Plugin interface version
pub const PLUGIN_INTERFACE_VERSION: u32 = 1;
