//! Example Rust Plugin
//!
//! Demonstrates how to create a custom plugin in Rust
//! that can be loaded by the server via FFI/C ABI
//!
//! This plugin provides:
//! - Custom command execution
//! - Metrics reporting
//! - Plugin lifecycle management
//!
//! Build as cdylib:
//! ```bash
//! cargo build --release --manifest-path crates/plugins/Cargo.toml
//! ```

use std::ffi::{c_char, CStr, CString};
use std::os::raw::c_int;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

// Plugin metadata
const PLUGIN_NAME: &str = "example_rust_plugin";
const PLUGIN_VERSION: &str = "1.0.0";
const PLUGIN_DESCRIPTION: &str = "Example Rust plugin demonstrating custom commands";

// Maximum sizes matching C++ FFI constants
const MAX_COMMAND_LEN: usize = 1024;
const MAX_PATH_LEN: usize = 4096;
const MAX_OUTPUT_LEN: usize = 1024;
const MAX_METRICS_LEN: usize = 512;

// Plugin state (thread-safe)
static COMMANDS_EXECUTED: AtomicI32 = AtomicI32::new(0);
static ERRORS_ENCOUNTERED: AtomicI32 = AtomicI32::new(0);
static START_TIME: AtomicU64 = AtomicU64::new(0);

/// Get plugin information
/// Format: "name:version:description"
#[no_mangle]
pub extern "C" fn get_plugin_info() -> *const c_char {
    let info = format!("{}:{}:{}", PLUGIN_NAME, PLUGIN_VERSION, PLUGIN_DESCRIPTION);
    let c_info = CString::new(info).unwrap_or_default();
    // Leak the string so it stays valid (called once at load)
    c_info.into_raw()
}

/// Initialize plugin
#[no_mangle]
pub extern "C" fn plugin_initialize() -> bool {
    COMMANDS_EXECUTED.store(0, Ordering::SeqCst);
    ERRORS_ENCOUNTERED.store(0, Ordering::SeqCst);
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    START_TIME.store(now, Ordering::SeqCst);
    
    true
}

/// Execute custom command
/// 
/// # Arguments
/// * `command` - Null-terminated command string
/// * `output` - Output buffer to write result
/// * `output_len` - Length of output buffer
/// 
/// # Returns
/// * `true` - Command executed successfully
/// * `false` - Command failed
#[no_mangle]
pub extern "C" fn plugin_execute_command(
    command: *const c_char,
    output: *mut c_char,
    output_len: usize,
) -> bool {
    // Validate inputs
    if command.is_null() || output.is_null() || output_len == 0 {
        ERRORS_ENCOUNTERED.fetch_add(1, Ordering::SeqCst);
        return false;
    }
    
    // Parse command
    let cmd_str = unsafe {
        match CStr::from_ptr(command).to_str() {
            Ok(s) => s,
            Err(_) => {
                ERRORS_ENCOUNTERED.fetch_add(1, Ordering::SeqCst);
                return false;
            }
        }
    };
    
    // Execute command and capture result
    let result = execute_rust_command(cmd_str);
    
    // Copy result to output buffer
    let output_slice = unsafe {
        std::slice::from_raw_parts_mut(output as *mut u8, output_len)
    };
    
    let result_bytes = result.as_bytes();
    let copy_len = std::cmp::min(result_bytes.len(), output_len - 1);
    output_slice[..copy_len].copy_from_slice(&result_bytes[..copy_len]);
    output_slice[copy_len] = 0; // Null terminator
    
    // Update metrics
    COMMANDS_EXECUTED.fetch_add(1, Ordering::SeqCst);
    
    // Simple success check - commands starting with "echo" succeed
    let success = cmd_str.starts_with("echo") || cmd_str.starts_with("status");
    
    if !success {
        ERRORS_ENCOUNTERED.fetch_add(1, Ordering::SeqCst);
    }
    
    success
}

/// Get plugin metrics as JSON
#[no_mangle]
pub extern "C" fn plugin_get_metrics(metrics_buffer: *mut c_char, buffer_len: usize) -> bool {
    if metrics_buffer.is_null() || buffer_len == 0 {
        return false;
    }
    
    let start = START_TIME.load(Ordering::SeqCst);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let uptime = (now - start) as f64;
    
    let metrics = format!(
        r#"{{"commands_executed":{},"errors_encountered":{},"uptime_seconds":{},"status":"{}"}}"#,
        COMMANDS_EXECUTED.load(Ordering::SeqCst),
        ERRORS_ENCOUNTERED.load(Ordering::SeqCst),
        uptime,
        if ERRORS_ENCOUNTERED.load(Ordering::SeqCst) > 0 {
            "degraded"
        } else {
            "healthy"
        }
    );
    
    let metrics_bytes = metrics.as_bytes();
    let copy_len = std::cmp::min(metrics_bytes.len(), buffer_len - 1);
    
    let buffer_slice = unsafe {
        std::slice::from_raw_parts_mut(metrics_buffer as *mut u8, buffer_len)
    };
    buffer_slice[..copy_len].copy_from_slice(&metrics_bytes[..copy_len]);
    buffer_slice[copy_len] = 0;
    
    true
}

/// Cleanup plugin resources
#[no_mangle]
pub extern "C" fn plugin_cleanup() {
    COMMANDS_EXECUTED.store(0, Ordering::SeqCst);
    ERRORS_ENCOUNTERED.store(0, Ordering::SeqCst);
    START_TIME.store(0, Ordering::SeqCst);
}

// Internal command execution
fn execute_rust_command(command: &str) -> String {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return "Error: Empty command".to_string();
    }
    
    match parts[0] {
        "echo" => {
            // Echo command - return arguments
            parts[1..].join(" ")
        }
        "status" => {
            // Status command - return plugin status
            format!(
                "Plugin: {} v{} | Commands: {} | Errors: {} | Uptime: {}s",
                PLUGIN_NAME,
                PLUGIN_VERSION,
                COMMANDS_EXECUTED.load(Ordering::SeqCst),
                ERRORS_ENCOUNTERED.load(Ordering::SeqCst),
                START_TIME.load(Ordering::SeqCst)
            )
        }
        "help" => {
            // Help command - list available commands
            "Available commands:\n\
             - echo <message> : Echo the message back\n\
             - status : Show plugin status\n\
             - help : Show this help message"
                .to_string()
        }
        _ => {
            // Unknown command
            format!("Unknown command: {}. Type 'help' for available commands.", parts[0])
        }
    }
}

// Ensure C ABI compatibility
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_plugin_lifecycle() {
        assert!(plugin_initialize());
        
        let mut output = vec![0u8; 256];
        let cmd = CString::new("echo hello").unwrap();
        let result = plugin_execute_command(
            cmd.as_ptr(),
            output.as_mut_ptr() as *mut c_char,
            output.len()
        );
        assert!(result);
        
        let output_str = CStr::from_bytes_with_nul(&output)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(output_str.contains("hello"));
        
        plugin_cleanup();
    }
    
    #[test]
    fn test_metrics() {
        plugin_initialize();
        
        let mut metrics = vec![0u8; 512];
        let result = plugin_get_metrics(
            metrics.as_mut_ptr() as *mut c_char,
            metrics.len()
        );
        assert!(result);
        
        let metrics_str = CStr::from_bytes_with_nul(&metrics)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(metrics_str.contains("commands_executed"));
        
        plugin_cleanup();
    }
}
