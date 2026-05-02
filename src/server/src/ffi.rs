//! Foreign Function Interface for C++ plugins
//!
//! Defines the Rust-side types and interfaces for interacting with
//! platform-specific C++ forensic plugins.

use std::ffi::{c_char, c_int, c_ulonglong, c_void, CStr};
use std::mem::MaybeUninit;
use anyhow::{anyhow, Result};

/// Maximum sizes matching plugin_interface.h
/// These constants are used by C++ plugins via FFI.
/// Some are unused in Rust code but kept for API compatibility.
pub const MAX_HOSTNAME_LEN: usize = 256;
pub const MAX_OS_TYPE_LEN: usize = 64;
pub const MAX_OS_VERSION_LEN: usize = 64;
#[allow(dead_code)]
pub const MAX_COMMAND_LEN: usize = 1024;
#[allow(dead_code)]
pub const MAX_PATH_LEN: usize = 4096;
pub const MAX_NAME_LEN: usize = 128;
pub const MAX_VERSION_LEN: usize = 64;
pub const MAX_DESCRIPTION_LEN: usize = 512;

/// Percentage type (0-100)
pub type Percentage = u8;

/// Process information structure
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: c_int,
    pub name: [c_char; MAX_HOSTNAME_LEN],
    pub memory_usage: c_ulonglong,
    pub cpu_usage: Percentage,
    pub start_time: c_ulonglong,
}

/// System metrics structure
#[repr(C)]
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub hostname: [c_char; MAX_HOSTNAME_LEN],
    pub ram_usage: Percentage,
    pub cpu_usage: Percentage,
    pub uptime: c_ulonglong,
}

/// System information structure
#[repr(C)]
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub os_type: [c_char; MAX_OS_TYPE_LEN],
    pub os_version: [c_char; MAX_OS_VERSION_LEN],
    pub hostname: [c_char; MAX_HOSTNAME_LEN],
    pub cpu_cores: c_int,
    pub total_memory: c_ulonglong,
    pub available_memory: c_ulonglong,
    pub uptime: c_ulonglong,
}

/// Plugin information structure
#[repr(C)]
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: [c_char; MAX_NAME_LEN],
    pub version: [c_char; MAX_VERSION_LEN],
    pub description: [c_char; MAX_DESCRIPTION_LEN],
}

/// Plugin interface function types
type GetPluginInfoFn = unsafe extern "C" fn() -> *mut PluginInfo;
type InitFn = unsafe extern "C" fn() -> bool;
type CleanupFn = unsafe extern "C" fn();
type GetSystemMetricsFn = unsafe extern "C" fn(metrics: *mut SystemMetrics) -> bool;
type GetProcessesFn = unsafe extern "C" fn(processes: *mut *mut ProcessInfo, count: *mut usize) -> bool;
type GetSystemInfoFn = unsafe extern "C" fn(info: *mut SystemInfo) -> bool;
type FreeMemoryFn = unsafe extern "C" fn(ptr: *mut c_void);

/// C plugin interface structure
#[repr(C)]
#[derive(Clone)]
pub struct PluginInterface {
    pub get_plugin_info: GetPluginInfoFn,
    pub init: InitFn,
    pub cleanup: CleanupFn,
    pub get_system_metrics: GetSystemMetricsFn,
    pub get_processes: GetProcessesFn,
    pub execute_command: *const c_void,  // Placeholder
    pub read_file: *const c_void,        // Placeholder
    pub get_system_info: GetSystemInfoFn,
    pub get_directory_info_data: *const c_void,  // Placeholder
    pub get_event_data: *const c_void,   // Placeholder
    pub get_watchers_data: *const c_void,  // Placeholder
    pub get_file_reader_data: *const c_void,  // Placeholder
    pub get_sensor_data: *const c_void,  // Placeholder
    pub get_camera_data: *const c_void,  // Placeholder
    pub get_processing_results: *const c_void,  // Placeholder
    pub get_video_frame: *const c_void,  // Placeholder
    pub free_memory: FreeMemoryFn,
}

impl PluginInterface {
    /// Get safe plugin info
    pub unsafe fn get_plugin_info_safe(&self) -> Result<PluginInfoSafe> {
        let ptr = (self.get_plugin_info)();
        if ptr.is_null() {
            return Err(anyhow!("Plugin returned null info"));
        }
        
        let info = &*ptr;
        Ok(PluginInfoSafe {
            name: cstr_to_string(&info.name)?,
            version: cstr_to_string(&info.version)?,
            description: cstr_to_string(&info.description)?,
        })
    }
    
    /// Initialize plugin
    pub unsafe fn init(&self) -> Result<()> {
        if !(self.init)() {
            return Err(anyhow!("Plugin initialization failed"));
        }
        Ok(())
    }
    
    /// Cleanup plugin
    pub unsafe fn cleanup(&self) {
        (self.cleanup)();
    }
    
    /// Get system metrics
    pub unsafe fn get_system_metrics(&self) -> Result<SystemMetrics> {
        let mut metrics = MaybeUninit::<SystemMetrics>::uninit();
        if !(self.get_system_metrics)(metrics.as_mut_ptr()) {
            return Err(anyhow!("Failed to get system metrics"));
        }
        Ok(metrics.assume_init())
    }
    
    /// Get processes
    pub unsafe fn get_processes(&self) -> Result<Vec<ProcessInfo>> {
        let mut processes: *mut ProcessInfo = std::ptr::null_mut();
        let mut count: usize = 0;
        
        if !(self.get_processes)(&mut processes, &mut count) {
            return Err(anyhow!("Failed to get processes"));
        }
        
        if processes.is_null() || count == 0 {
            return Ok(Vec::new());
        }
        
        let slice = std::slice::from_raw_parts(processes, count);
        let result = slice.to_vec();
        
        // Free memory allocated by plugin
        (self.free_memory)(processes as *mut c_void);
        
        Ok(result)
    }
    
    /// Get system info
    pub unsafe fn get_system_info(&self) -> Result<SystemInfo> {
        let mut info = MaybeUninit::<SystemInfo>::uninit();
        if !(self.get_system_info)(info.as_mut_ptr()) {
            return Err(anyhow!("Failed to get system info"));
        }
        Ok(info.assume_init())
    }
}

/// Safe wrapper for plugin info
#[derive(Debug, Clone)]
pub struct PluginInfoSafe {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// Safe wrapper for plugin interface
pub struct SafePluginInterface {
    interface: PluginInterface,
}

impl SafePluginInterface {
    pub fn new(interface: PluginInterface) -> Self {
        Self { interface }
    }
    
    pub fn get_plugin_info(&self) -> Result<PluginInfoSafe> {
        unsafe { self.interface.get_plugin_info_safe() }
    }
    
    pub fn init(&self) -> Result<()> {
        unsafe { self.interface.init() }
    }
    
    pub fn cleanup(&self) {
        unsafe { self.interface.cleanup() }
    }
    
    pub fn get_system_metrics(&self) -> Result<SystemMetrics> {
        unsafe { self.interface.get_system_metrics() }
    }
    
    pub fn get_processes(&self) -> Result<Vec<ProcessInfo>> {
        unsafe { self.interface.get_processes() }
    }
    
    pub fn get_system_info(&self) -> Result<SystemInfo> {
        unsafe { self.interface.get_system_info() }
    }
}

/// Helper to convert C string to Rust String
fn cstr_to_string(buf: &[c_char]) -> Result<String> {
    let cstr = unsafe { CStr::from_ptr(buf.as_ptr()) };
    cstr.to_str()
        .map(|s| s.to_owned())
        .map_err(|e| anyhow!("Invalid UTF-8 in C string: {}", e))
}

/// Helper to get string from fixed-size buffer
/// 
/// This is a public API utility for external FFI usage.
/// Marked as allow(dead_code) since it's primarily used by
/// external C plugins through FFI, not internally.
#[allow(dead_code)]
pub fn buffer_to_string(buf: &[c_char]) -> String {
    cstr_to_string(buf).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_to_string() {
        let mut buf = [0i8; 10];
        buf[0] = b'h' as i8;
        buf[1] = b'i' as i8;
        assert_eq!(buffer_to_string(&buf), "hi");
    }
}
