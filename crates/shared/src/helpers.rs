//! Shared helper functions for Mini MSP.
//!
//! These utilities support FFI-safe string conversion, timestamp formatting,
//! and semantic validation routines used across the Rust server and shared crate.

use std::ffi::{c_char, CStr};

use crate::types::{FileSize, Percentage, ProcessId, Timestamp};
use crate::constants::{INVALID_PID, INVALID_TIMESTAMP, MAX_PERCENTAGE, MIN_PERCENTAGE};

/// Convert a C string pointer to a Rust string slice.
///
/// Returns `None` for null pointers or invalid UTF-8.
pub unsafe fn c_str_to_str(ptr: *const c_char) -> Option<&'static str> {
    if ptr.is_null() {
        return None;
    }

    CStr::from_ptr(ptr).to_str().ok()
}

/// Convert a C string pointer to a Rust `String`.
///
/// Returns an empty string for null pointers.
pub unsafe fn c_str_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }

    CStr::from_ptr(ptr).to_string_lossy().into_owned()
}

/// Check whether a timestamp is valid.
pub fn is_valid_timestamp(ts: Timestamp) -> bool {
    ts != INVALID_TIMESTAMP
}

/// Check whether a process identifier is valid.
pub fn is_valid_pid(pid: ProcessId) -> bool {
    pid != INVALID_PID
}

/// Clamp a percentage value to the valid 0..=100 range.
pub fn clamp_percentage(p: Percentage) -> Percentage {
    p.clamp(MIN_PERCENTAGE, MAX_PERCENTAGE)
}

/// Format a file size using unit suffixes.
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

/// Format a timestamp in a simple readable form.
pub fn format_timestamp(ts: Timestamp) -> String {
    format!("{} ms since epoch", ts)
}
