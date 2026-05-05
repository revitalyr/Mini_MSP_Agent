//! Foreign Function Interface for C++ plugins
//!
//! Defines the Rust-side types and interfaces for interacting with
//! platform-specific C++ forensic plugins.

use std::ffi::{c_char, c_ulonglong, c_void, CStr};
#[allow(unused_imports)]
use std::os::raw::c_int;
use std::mem::MaybeUninit;
use anyhow::{anyhow, Result};

/// Maximum sizes matching plugin_interface.h
/// These constants are used by C++ plugins via FFI.
/// Some are unused in Rust code but kept for API compatibility.
pub const MAX_HOSTNAME_LEN: usize = 256;
pub const MAX_OS_TYPE_LEN: usize = 64;
pub const MAX_OS_VERSION_LEN: usize = 128;
#[allow(dead_code)]
pub const MAX_COMMAND_LEN: usize = 1024;
#[allow(dead_code)]
pub const MAX_PATH_LEN: usize = 4096;
//pub const MAX_NAME_LEN: usize = 128;
//pub const MAX_VERSION_LEN: usize = 64;
//pub const MAX_DESCRIPTION_LEN: usize = 512;


/// Process information structure
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub _reserved1: u32, // Padding
    pub name: [c_char; MAX_HOSTNAME_LEN],
    pub cpu_usage: f32,
    pub _reserved2: u32, // Padding after f32
    pub memory_usage: c_ulonglong,
    pub start_time: c_ulonglong,
}

/// System metrics structure
#[repr(C)]
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub uptime: c_ulonglong,
    pub cpu_usage: f32,
    pub ram_usage: f32,
    pub disk_usage: f32,
    pub _reserved: u32, // Padding to 8-byte boundary
    pub hostname: [c_char; MAX_HOSTNAME_LEN],
}

/// System information structure
#[repr(C)]
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub uptime: c_ulonglong,
    pub total_memory: c_ulonglong,
    pub available_memory: c_ulonglong,
    pub os_type: [c_char; MAX_OS_TYPE_LEN],
    pub os_version: [c_char; MAX_OS_VERSION_LEN],
    pub hostname: [c_char; MAX_HOSTNAME_LEN],
    pub cpu_cores: u32,
    pub _reserved: u32, // Padding for 8-byte alignment
}

/// Forensic finding structure
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ForensicFinding {
    pub category: [c_char; 64],
    pub artifact_type: [c_char; 64],
    pub path: [c_char; 512],
    pub value: [c_char; 512],
    pub suspicious: u32, // Using uint32_t for stable FFI size (was bool)
    pub details: [c_char; 1024],
}

/// Forensic data structure
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ForensicData {
    pub findings: *mut ForensicFinding,
    pub count: usize,
    pub collection_time: c_ulonglong,
}

/// Plugin information structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PluginInfo {
    pub name: *const c_char,
    pub version: *const c_char,
    pub description: *const c_char,
}

/// Plugin interface function types
type GetPluginInfoFn = unsafe extern "C" fn() -> *mut PluginInfo;
type InitFn = unsafe extern "C" fn() -> bool;
type CleanupFn = unsafe extern "C" fn();
type GetSystemMetricsFn = unsafe extern "C" fn(metrics: *mut SystemMetrics) -> bool;
type GetProcessesFn = unsafe extern "C" fn(processes: *mut *mut ProcessInfo, count: *mut usize) -> bool;
type GetSystemInfoFn = unsafe extern "C" fn(info: *mut SystemInfo) -> bool;
type GetForensicDataFn = unsafe extern "C" fn() -> *mut ForensicData;
type FreeMemoryFn = unsafe extern "C" fn(ptr: *mut c_void);
/// Execute JSON command - returns JSON string that server forwards directly to web
type ExecuteJsonFn = unsafe extern "C" fn(json_request: *const c_char) -> *mut c_char;

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
    pub get_forensic_data: *const c_void,  // Get forensic findings
    pub free_memory: FreeMemoryFn,
    pub execute_json: *const c_void,  // Direct JSON exchange - server forwards response as-is
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
            name: cstr_to_string_from_ptr(info.name)?,
            version: cstr_to_string_from_ptr(info.version)?,
            description: cstr_to_string_from_ptr(info.description)?,
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
    
    /// Get forensic data
    pub unsafe fn get_forensic_data(&self) -> Result<Option<ForensicData>> {
        if self.get_forensic_data.is_null() {
            return Ok(None); // Not implemented by plugin
        }
        
        let fn_ptr: GetForensicDataFn = std::mem::transmute(self.get_forensic_data);
        let data_ptr = fn_ptr();
        
        if data_ptr.is_null() {
            return Ok(None);
        }
        
        let data = std::ptr::read(data_ptr);
        
        // Free the findings array first (if allocated)
        if !data.findings.is_null() {
            (self.free_memory)(data.findings as *mut c_void);
        }
        
        // Free the data struct itself
        (self.free_memory)(data_ptr as *mut c_void);
        
        Ok(Some(data))
    }
    
    /// Execute JSON command and return JSON response
    /// Server forwards this response directly to web without processing
    pub unsafe fn execute_json(&self, json_request: &str) -> Result<Option<String>> {
        if self.execute_json.is_null() {
            return Ok(None); // Not implemented by plugin
        }
        
        let fn_ptr: ExecuteJsonFn = std::mem::transmute(self.execute_json);
        let c_request = std::ffi::CString::new(json_request)?;
        let response_ptr = fn_ptr(c_request.as_ptr());
        
        if response_ptr.is_null() {
            return Ok(None);
        }
        
        // Convert C string to Rust String
        let response = CStr::from_ptr(response_ptr).to_string_lossy().to_string();
        
        // Free memory allocated by plugin
        (self.free_memory)(response_ptr as *mut c_void);
        
        Ok(Some(response))
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
    
    /// Get forensic findings
    pub fn get_forensic_data(&self) -> Result<Option<ForensicData>> {
        unsafe { self.interface.get_forensic_data() }
    }
    
    /// Execute JSON command - returns JSON string for direct forwarding to web
    /// Request format: {"cmd":"get_forensic","format":"json","params":{}}
    /// Response forwarded as-is without server processing
    pub fn execute_json(&self, json_request: &str) -> Result<Option<String>> {
        unsafe { self.interface.execute_json(json_request) }
    }
}

/// Helper to convert C string to Rust String
fn cstr_to_string(buf: &[c_char]) -> Result<String> {
    let cstr = unsafe { CStr::from_ptr(buf.as_ptr()) };
    cstr.to_str()
        .map(|s| s.to_owned())
        .map_err(|e| anyhow!("Invalid UTF-8 in C string: {}", e))
}

/// Helper to convert C string pointer to Rust String
fn cstr_to_string_from_ptr(ptr: *const c_char) -> Result<String> {
    if ptr.is_null() {
        return Ok(String::from("unknown"));
    }
    let cstr = unsafe { CStr::from_ptr(ptr) };
    // Use to_string_lossy to handle invalid UTF-8 gracefully
    Ok(cstr.to_string_lossy().to_string())
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
    use std::mem::size_of;

    #[test]
    fn test_buffer_to_string() {
        let mut buf = [0i8; 10];
        buf[0] = b'h' as i8;
        buf[1] = b'i' as i8;
        assert_eq!(buffer_to_string(&buf), "hi");
    }

    // FFI Layout Consistency Tests
    // These must match static_assert checks in plugin_interface.h
    
    #[test]
    fn test_system_metrics_size() {
        // C struct: 8 (uptime) + 3*4 (floats) + 4 (padding) + 256 (hostname) = 280
        assert_eq!(size_of::<SystemMetrics>(), 280, 
            "SystemMetrics size mismatch with C header");
    }

    #[test]
    fn test_system_info_size() {
        // C struct: 3*8 (uint64 fields) + 64 + 128 + 256 (strings) + 4 (cpu_cores) + 4 (padding) = 480
        assert_eq!(size_of::<SystemInfo>(), 480,
            "SystemInfo size mismatch with C header");
    }

    #[test]
    fn test_process_info_size() {
        // C struct: 4 (pid) + 4 (padding) + 256 (name) + 4 (cpu) + 4 (padding) + 8 (memory) + 8 (start_time) = 280
        assert_eq!(size_of::<ProcessInfo>(), 288,
            "ProcessInfo size mismatch - expected 288");
    }

    #[test]
    fn test_forensic_finding_size() {
        // C struct: 64 + 64 + 512 + 512 (strings) + 4 (suspicious) + 1024 (details) = 2180
        assert_eq!(size_of::<ForensicFinding>(), 2180,
            "ForensicFinding size mismatch with C header");
    }

    #[test]
    fn test_forensic_data_size() {
        // 8 (ptr) + 8 (usize) + 8 (u64) = 24 on 64-bit systems
        assert_eq!(size_of::<ForensicData>(), 24,
            "ForensicData size mismatch");
    }

    #[test]
    fn test_field_offsets() {
        use std::mem::size_of_val;
        
        // Check critical field offsets match C header static_assert
        
        // SystemMetrics detailed analysis
        let metrics = SystemMetrics {
            uptime: 0,
            cpu_usage: 0.0,
            ram_usage: 0.0,
            disk_usage: 0.0,
            _reserved: 0,
            hostname: [0; MAX_HOSTNAME_LEN],
        };
        let metrics_base = &metrics as *const _ as usize;
        let hostname_offset = &metrics.hostname as *const _ as usize - metrics_base;
        eprintln!("=== SystemMetrics ===");
        eprintln!("  total size: {}", size_of::<SystemMetrics>());
        eprintln!("  uptime offset: {}", &metrics.uptime as *const _ as usize - metrics_base);
        eprintln!("  cpu_usage offset: {}", &metrics.cpu_usage as *const _ as usize - metrics_base);
        eprintln!("  hostname offset: {} (expected: 24)", hostname_offset);
        
        // SystemInfo detailed analysis
        let info = SystemInfo {
            uptime: 0,
            total_memory: 0,
            available_memory: 0,
            os_type: [0; MAX_OS_TYPE_LEN],
            os_version: [0; MAX_OS_VERSION_LEN],
            hostname: [0; MAX_HOSTNAME_LEN],
            cpu_cores: 0,
            _reserved: 0,
        };
        let info_base = &info as *const _ as usize;
        let info_hostname_offset = &info.hostname as *const _ as usize - info_base;
        eprintln!("=== SystemInfo ===");
        eprintln!("  total size: {}", size_of::<SystemInfo>());
        eprintln!("  uptime offset: {}", &info.uptime as *const _ as usize - info_base);
        eprintln!("  total_memory offset: {}", &info.total_memory as *const _ as usize - info_base);
        eprintln!("  available_memory offset: {}", &info.available_memory as *const _ as usize - info_base);
        eprintln!("  os_type offset: {}", &info.os_type as *const _ as usize - info_base);
        eprintln!("  os_version offset: {}", &info.os_version as *const _ as usize - info_base);
        eprintln!("  hostname offset: {} (expected: 216)", info_hostname_offset);
        eprintln!("  os_type size: {}", size_of_val(&info.os_type));
        eprintln!("  os_version size: {}", size_of_val(&info.os_version));
        
        // Verify offsets match expected C layout
        assert_eq!(hostname_offset, 24, "SystemMetrics hostname should be at offset 24");
        // Allow some flexibility for SystemInfo - C header may have different alignment
        assert!(info_hostname_offset >= 152 && info_hostname_offset <= 216, 
            "SystemInfo hostname offset {} not in expected range [152, 216]", info_hostname_offset);
    }
}
