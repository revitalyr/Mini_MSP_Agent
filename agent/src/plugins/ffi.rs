use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::slice;
use anyhow::{anyhow, Result};

// --- Semantic Type Aliases ---
pub type FileSize = u64;
pub type Timestamp = u64;
pub type FileCount = u32;
pub type CallCount = u32;
pub type Percentage = u8;
// -----------------------------

/// Макрос для автоматической генерации конвертации из C-структур в Rust
macro_rules! impl_ffi_conv {
    // Основной вход: C-тип => Safe-тип { поле_C [: тип_обработки] => поле_Rust, ... }
    ($c_type:ty => $safe_type:ident { $( $c_field:ident $( : $kind:ident )? => $safe_field:ident ),* $(,)? }) => {
        impl $safe_type {
            pub unsafe fn from_c_struct(info: $c_type) -> Self {
                Self {
                    $(
                        $safe_field: impl_ffi_conv!(@convert info.$c_field $( : $kind )?),
                    )*
                }
            }
        }
    };

    // Вспомогательные правила конвертации
    (@convert $expr:expr) => { $expr }; // Прямое копирование
    (@convert $expr:expr : string) => { c_string_to_string_lossy($expr) }; // Указатель в String
    (@convert $expr:expr : string_array) => { c_string_to_string_lossy($expr.as_ptr()) }; // [c_char] в String
    (@convert $expr:expr : percentage) => { 
        // Проверка границ: гарантируем, что процент не превысит 100
        std::cmp::min($expr, 100) 
    };
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PluginInfo {
    pub name: *const c_char,
    pub version: *const c_char,
    pub description: *const c_char,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub cpu_usage: Percentage,
    pub ram_usage: Percentage,
    pub disk_usage: Percentage,
    pub uptime: Timestamp,
    pub hostname: [c_char; 256],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: [c_char; 256],
    pub cpu_usage: Percentage,
    pub memory_usage: FileSize,
    pub start_time: Timestamp,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FileContent {
    pub content: *mut c_char,
    pub size: usize,
    pub success: bool,
    pub error: [c_char; 512],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CommandResult {
    pub output: *mut c_char,
    pub exit_code: i32,
    pub success: bool,
    pub error: [c_char; 256],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SystemInfo {
    pub os_type: [c_char; 64],
    pub os_version: [c_char; 128],
    pub hostname: [c_char; 256],
    pub uptime: u64,
    pub cpu_cores: u32,
    pub total_memory: u64,
    pub available_memory: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CDirectoryInfoData {
    pub m_path: *mut c_char,
    pub m_total_files: FileCount,
    pub m_total_directories: FileCount,
    pub m_total_size_bytes: FileSize,
    pub m_hidden_files: FileCount,
    pub m_hidden_directories: FileCount,
    pub m_scan_timestamp: Timestamp,
    pub m_scan_progress: Percentage,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CEventData {
    pub m_path: *mut c_char,
    pub m_events_count: CallCount,
    pub m_buffer_usage: Percentage,
    pub m_last_event: [c_char; 64],
    pub m_timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CWatchersData {
    pub m_active_watchers: FileCount,
    pub m_total_notifications: CallCount,
    pub m_cpu_usage: f32,
    pub m_memory_usage_kb: FileSize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CFileReaderData {
    pub m_path: *mut c_char,
    pub m_size: u64,
    pub m_encoding: [c_char; 32],
    pub m_is_locked: bool,
    pub m_last_access: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CSensorData {
    pub m_temperature: f32,
    pub m_humidity: f32,
    pub m_pressure: f32,
    pub m_timestamp: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CCameraData {
    pub m_width: u32,
    pub m_height: u32,
    pub m_fps: u32,
    pub m_codec: [c_char; 16],
    pub m_timestamp: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CProcessingResults {
    pub m_status: [c_char; 64],
    pub m_load_index: f32,
    pub m_processed_items: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CVideoFrame {
    pub m_data: *mut u8,
    pub m_size: u64,
    pub m_width: u32,
    pub m_height: u32,
    pub m_timestamp: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PluginInterface {
    pub get_plugin_info: Option<unsafe extern "C" fn() -> *mut PluginInfo>,
    pub init: Option<unsafe extern "C" fn() -> bool>,
    pub cleanup: Option<unsafe extern "C" fn()>,
    pub get_system_metrics: Option<unsafe extern "C" fn(*mut SystemMetrics) -> bool>,
    pub get_processes: Option<unsafe extern "C" fn(*mut *mut ProcessInfo, *mut usize) -> bool>,
    pub execute_command: Option<unsafe extern "C" fn(*const c_char, *mut CommandResult) -> bool>,
    pub read_file: Option<unsafe extern "C" fn(*const c_char, *mut FileContent) -> bool>,
    pub get_system_info: Option<unsafe extern "C" fn(*mut SystemInfo) -> bool>,
    pub get_directory_info_data: Option<unsafe extern "C" fn(
        *const c_char, 
        bool, 
        bool, 
        u32
    ) -> *mut CDirectoryInfoData>,
    pub get_event_data: Option<unsafe extern "C" fn(*const c_char) -> *mut CEventData>,
    pub get_watchers_data: Option<unsafe extern "C" fn() -> *mut CWatchersData>,
    pub get_file_reader_data: Option<unsafe extern "C" fn(*const c_char) -> *mut CFileReaderData>,
    pub get_sensor_data: Option<unsafe extern "C" fn() -> *mut CSensorData>,
    pub get_camera_data: Option<unsafe extern "C" fn() -> *mut CCameraData>,
    pub get_processing_results: Option<unsafe extern "C" fn() -> *mut CProcessingResults>,
    pub get_video_frame: Option<unsafe extern "C" fn() -> *mut CVideoFrame>,
    pub free_memory: Option<unsafe extern "C" fn(*mut c_void)>,
}

// Safe Rust wrappers for C structures
impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            cpu_usage: 0u8,
            ram_usage: 0u8,
            disk_usage: 0u8,
            uptime: 0,
            hostname: [0; 256],
        }
    }
}

impl Default for ProcessInfo {
    fn default() -> Self {
        Self {
            pid: 0,
            name: [0; 256],
            cpu_usage: 0u8,
            memory_usage: 0,
            start_time: 0,
        }
    }
}

impl Default for FileContent {
    fn default() -> Self {
        Self {
            content: ptr::null_mut(),
            size: 0,
            success: false,
            error: [0; 512],
        }
    }
}

impl Default for CommandResult {
    fn default() -> Self {
        Self {
            output: ptr::null_mut(),
            exit_code: 0,
            success: false,
            error: [0; 256],
        }
    }
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self {
            os_type: [0; 64],
            os_version: [0; 128],
            hostname: [0; 256],
            uptime: 0,
            cpu_cores: 0,
            total_memory: 0,
            available_memory: 0,
        }
    }
}

// Helper functions for safe string conversion
pub unsafe fn c_string_to_string(ptr: *const c_char) -> Result<String> {
    if ptr.is_null() {
        return Err(anyhow!("Null pointer"));
    }
    
    let c_str = CStr::from_ptr(ptr);
    c_str.to_str()
        .map(|s| s.to_string())
        .map_err(|e| anyhow!("Invalid UTF-8: {}", e))
}

pub unsafe fn c_string_to_string_lossy(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    
    let c_str = CStr::from_ptr(ptr);
    c_str.to_string_lossy().into_owned()
}

pub fn string_to_c_string(s: &str) -> Result<CString> {
    CString::new(s).map_err(|e| anyhow!("Invalid string: {}", e))
}

// Safe wrappers for FFI functions
pub struct SafePluginInterface {
    interface: PluginInterface,
    free_memory: Option<unsafe extern "C" fn(*mut c_void)>,
}

impl SafePluginInterface {
    pub unsafe fn new(interface: PluginInterface) -> Self {
        Self {
            interface,
            free_memory: interface.free_memory,
        }
    }
    
    pub fn get_plugin_info(&self) -> Result<PluginInfoData> {
        unsafe {
            let get_info = self.interface.get_plugin_info
                .ok_or_else(|| anyhow!("get_plugin_info function not available"))?;
            
            let info = get_info();
            if info.is_null() {
                return Err(anyhow!("Plugin info is null"));
            }
            
            PluginInfoData::from_c_struct(*info)
        }
    }
    
    pub fn init(&self) -> Result<bool> {
        unsafe {
            let init = self.interface.init
                .ok_or_else(|| anyhow!("init function not available"))?;
            
            Ok(init())
        }
    }
    
    pub fn cleanup(&self) {
        unsafe {
            if let Some(cleanup) = self.interface.cleanup {
                cleanup();
            }
        }
    }
    
    pub fn get_system_metrics(&self) -> Result<SystemMetricsData> {
        unsafe {
            let get_metrics = self.interface.get_system_metrics
                .ok_or_else(|| anyhow!("get_system_metrics function not available"))?;
            
            let mut metrics = SystemMetrics::default();
            if get_metrics(&mut metrics) {
                Ok(SystemMetricsData::from_c_struct(metrics))
            } else {
                Err(anyhow!("Failed to get system metrics"))
            }
        }
    }
    
    pub fn get_processes(&self) -> Result<Vec<ProcessInfoData>> {
        unsafe {
            let get_processes = self.interface.get_processes
                .ok_or_else(|| anyhow!("get_processes function not available"))?;
            
            let mut processes: *mut ProcessInfo = ptr::null_mut();
            let mut count: usize = 0;
            
            if get_processes(&mut processes, &mut count) {
                let mut result = Vec::with_capacity(count);
                
                for i in 0..count {
                    let process = *processes.add(i);
                    result.push(ProcessInfoData::from_c_struct(process));
                }
                
                // Free allocated memory
                if let Some(free_fn) = self.free_memory {
                    free_fn(processes as *mut c_void);
                }
                
                Ok(result)
            } else {
                Err(anyhow!("Failed to get processes"))
            }
        }
    }
    
    pub fn execute_command(&self, command: &str) -> Result<CommandResultData> {
        unsafe {
            let exec_cmd = self.interface.execute_command
                .ok_or_else(|| anyhow!("execute_command function not available"))?;
            
            let cmd_cstr = string_to_c_string(command)?;
            let mut result = CommandResult::default();
            
            if exec_cmd(cmd_cstr.as_ptr(), &mut result) {
                let data = CommandResultData::from_c_struct(&result);
                
                // Free allocated memory
                if !result.output.is_null() {
                    if let Some(free_fn) = self.free_memory {
                        free_fn(result.output as *mut c_void);
                    }
                }
                
                Ok(data)
            } else {
                Err(anyhow!("Failed to execute command"))
            }
        }
    }
    
    pub fn read_file(&self, path: &str) -> Result<FileContentData> {
        unsafe {
            let read_file = self.interface.read_file
                .ok_or_else(|| anyhow!("read_file function not available"))?;
            
            let path_cstr = string_to_c_string(path)?;
            let mut content = FileContent::default();
            
            if read_file(path_cstr.as_ptr(), &mut content) {
                let data = FileContentData::from_c_struct(&content);
                
                // Free allocated memory
                if !content.content.is_null() {
                    if let Some(free_fn) = self.free_memory {
                        free_fn(content.content as *mut c_void);
                    }
                }
                
                Ok(data)
            } else {
                Err(anyhow!("Failed to read file"))
            }
        }
    }
    
    pub fn get_system_info(&self) -> Result<SystemInfoData> {
        unsafe {
            let get_info = self.interface.get_system_info
                .ok_or_else(|| anyhow!("get_system_info function not available"))?;
            
            let mut info = SystemInfo::default();
            if get_info(&mut info) {
                let data = SystemInfoData::from_c_struct(&info);
                Ok(data)
            } else {
                Err(anyhow!("Failed to get system info"))
            }
        }
    }

    pub fn get_directory_info_data(&self, path: &str, recursive: bool, show_hidden: bool, max_depth: u32) -> Result<DirectoryInfoData> {
        unsafe {
            let get_dir = self.interface.get_directory_info_data
                .ok_or_else(|| anyhow!("get_directory_info_data function not available"))?;
            
            let path_cstr = string_to_c_string(path)?;
            let info_ptr = get_dir(path_cstr.as_ptr(), recursive, show_hidden, max_depth);
            
            if info_ptr.is_null() {
                return Err(anyhow!("Failed to get directory info data from plugin"));
            }
            
            let data = DirectoryInfoData::from_c_struct(*info_ptr);
            
            // Memory management: assuming plugin provides a way to free the struct
            if let Some(free_fn) = self.free_memory {
                free_fn(info_ptr as *mut c_void);
            }
            
            Ok(data)
        }
    }

    pub fn get_event_data(&self, path: &str) -> Result<EventData> {
        unsafe {
            let func = self.interface.get_event_data.ok_or_else(|| anyhow!("get_event_data not available"))?;
            let path_cstr = string_to_c_string(path)?;
            let ptr = func(path_cstr.as_ptr());
            if ptr.is_null() { return Err(anyhow!("Plugin returned null")); }
            let data = EventData::from_c_struct(*ptr);
            if let Some(free_fn) = self.free_memory { free_fn(ptr as *mut c_void); }
            Ok(data)
        }
    }

    pub fn get_watchers_data(&self) -> Result<WatchersData> {
        unsafe {
            let func = self.interface.get_watchers_data.ok_or_else(|| anyhow!("get_watchers_data not available"))?;
            let ptr = func();
            if ptr.is_null() { return Err(anyhow!("Plugin returned null")); }
            let data = WatchersData::from_c_struct(*ptr);
            if let Some(free_fn) = self.free_memory { free_fn(ptr as *mut c_void); }
            Ok(data)
        }
    }

    pub fn get_file_reader_data(&self, path: &str) -> Result<FileReaderData> {
        unsafe {
            let func = self.interface.get_file_reader_data.ok_or_else(|| anyhow!("get_file_reader_data not available"))?;
            let path_cstr = string_to_c_string(path)?;
            let ptr = func(path_cstr.as_ptr());
            if ptr.is_null() { return Err(anyhow!("Plugin returned null")); }
            let data = FileReaderData::from_c_struct(*ptr);
            if let Some(free_fn) = self.free_memory { free_fn(ptr as *mut c_void); }
            Ok(data)
        }
    }

    pub fn get_sensor_data(&self) -> Result<SensorData> {
        unsafe {
            let func = self.interface.get_sensor_data.ok_or_else(|| anyhow!("get_sensor_data not available"))?;
            let ptr = func();
            if ptr.is_null() { return Err(anyhow!("Plugin returned null")); }
            let data = SensorData::from_c_struct(*ptr);
            if let Some(free_fn) = self.free_memory { free_fn(ptr as *mut c_void); }
            Ok(data)
        }
    }

    pub fn get_camera_data(&self) -> Result<CameraData> {
        unsafe {
            let func = self.interface.get_camera_data.ok_or_else(|| anyhow!("get_camera_data not available"))?;
            let ptr = func();
            if ptr.is_null() { return Err(anyhow!("Plugin returned null")); }
            let data = CameraData::from_c_struct(*ptr);
            if let Some(free_fn) = self.free_memory { free_fn(ptr as *mut c_void); }
            Ok(data)
        }
    }

    pub fn get_processing_results(&self) -> Result<ProcessingResults> {
        unsafe {
            let func = self.interface.get_processing_results.ok_or_else(|| anyhow!("get_processing_results not available"))?;
            let ptr = func();
            if ptr.is_null() { return Err(anyhow!("Plugin returned null")); }
            let data = ProcessingResults::from_c_struct(*ptr);
            if let Some(free_fn) = self.free_memory { free_fn(ptr as *mut c_void); }
            Ok(data)
        }
    }

    pub fn get_video_frame(&self) -> Result<VideoFrameData> {
        unsafe {
            let func = self.interface.get_video_frame.ok_or_else(|| anyhow!("get_video_frame not available"))?;
            let ptr = func();
            if ptr.is_null() { return Err(anyhow!("Plugin returned null")); }
            
            let c_frame = *ptr;
            let data_slice = std::slice::from_raw_parts(c_frame.m_data, c_frame.m_size as usize);
            let frame_data = VideoFrameData {
                data: data_slice.to_vec(),
                width: c_frame.m_width,
                height: c_frame.m_height,
                timestamp: c_frame.m_timestamp,
            };

            if let Some(free_fn) = self.free_memory {
                // We need to free both the internal buffer and the struct
                // This assumes the C++ side provided a structure where m_data needs explicit freeing
                // if it wasn't part of the same allocation. Adjust based on C++ impl.
                // For now, we free the struct.
                free_fn(ptr as *mut c_void);
            }

            Ok(frame_data)
        }
    }
}

// Safe Rust data structures
#[derive(Debug, Clone)]
pub struct DirectoryInfoData {
    pub path: String,
    pub total_files: u64,
    pub total_directories: u64,
    pub total_size_bytes: u64,
    pub hidden_files: u64,
    pub hidden_directories: u64,
    pub scan_timestamp: u64,
    pub scan_progress: u8,
}

impl_ffi_conv!(CDirectoryInfoData => DirectoryInfoData {
    m_path : string => path,
    m_total_files => total_files,
    m_total_directories => total_directories,
    m_total_size_bytes => total_size_bytes,
    m_hidden_files => hidden_files,
    m_hidden_directories => hidden_directories,
    m_scan_timestamp => scan_timestamp,
    m_scan_progress : percentage => scan_progress,
});

#[derive(Debug, Clone)]
pub struct EventData {
    pub path: String,
    pub events_count: u64,
    pub buffer_usage: u8,
    pub last_event: String,
    pub timestamp: u64,
}

impl_ffi_conv!(CEventData => EventData {
    m_path : string => path,
    m_events_count => events_count,
    m_buffer_usage : percentage => buffer_usage,
    m_last_event : string_array => last_event,
    m_timestamp => timestamp,
});

impl EventData {
    unsafe fn from_c_struct(event: CEventData) -> Result<Self> {
        Ok(Self {
            path: c_string_to_string_lossy(event.m_path),
            events_count: event.m_events_count,
            buffer_usage: event.m_buffer_usage,
            last_event: c_string_to_string_lossy(event.m_last_event),
            timestamp: event.m_timestamp,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WatchersData {
    pub active_watchers: u32,
    pub total_notifications: u64,
    pub cpu_usage: f32,
    pub memory_usage_kb: u64,
}

impl_ffi_conv!(CWatchersData => WatchersData {
    m_active_watchers => active_watchers,
    m_total_notifications => total_notifications,
    m_cpu_usage => cpu_usage,
    m_memory_usage_kb => memory_usage_kb,
});

#[derive(Debug, Clone)]
pub struct FileReaderData {
    pub path: String,
    pub size: u64,
    pub encoding: String,
    pub is_locked: bool,
    pub last_access: u64,
}

impl_ffi_conv!(CFileReaderData => FileReaderData {
    m_path : string => path,
    m_size => size,
    m_encoding : string_array => encoding,
    m_is_locked => is_locked,
    m_last_access => last_access,
});

#[derive(Debug, Clone)]
pub struct SensorData {
    pub temperature: f32,
    pub humidity: f32,
    pub pressure: f32,
    pub timestamp: u64,
}

impl_ffi_conv!(CSensorData => SensorData {
    m_temperature => temperature,
    m_humidity => humidity,
    m_pressure => pressure,
    m_timestamp => timestamp,
});

#[derive(Debug, Clone)]
pub struct CameraData {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub codec: String,
    pub timestamp: u64,
}

impl_ffi_conv!(CCameraData => CameraData {
    m_width => width,
    m_height => height,
    m_fps => fps,
    m_codec : string_array => codec,
    m_timestamp => timestamp,
});

#[derive(Debug, Clone)]
pub struct ProcessingResults {
    pub status: String,
    pub load_index: f32,
    pub processed_items: u32,
}

impl_ffi_conv!(CProcessingResults => ProcessingResults {
    m_status : string_array => status,
    m_load_index => load_index,
    m_processed_items => processed_items,
});

#[derive(Debug, Clone)]
pub struct VideoFrameData {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct PluginInfoData {
    pub name: String,
    pub version: String,
    pub description: String,
}

impl PluginInfoData {
    unsafe fn from_c_struct(info: PluginInfo) -> Result<Self> {
        Ok(Self {
            name: c_string_to_string_lossy(info.name),
            version: c_string_to_string_lossy(info.version),
            description: c_string_to_string_lossy(info.description),
        })
    }
}

#[derive(Debug, Clone)]
pub struct SystemMetricsData {
    pub cpu_usage: f32,
    pub ram_usage: f32,
    pub disk_usage: f32,
    pub uptime: u64,
    pub hostname: String,
}

impl SystemMetricsData {
    unsafe fn from_c_struct(metrics: SystemMetrics) -> Self {
        Self {
            cpu_usage: metrics.cpu_usage as f32,
            ram_usage: metrics.ram_usage as f32,
            disk_usage: metrics.disk_usage as f32,
            uptime: metrics.uptime,
            hostname: c_string_to_string_lossy(metrics.hostname.as_ptr()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessInfoData {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub start_time: u64,
}

impl ProcessInfoData {
    unsafe fn from_c_struct(process: ProcessInfo) -> Self {
        Self {
            pid: process.pid,
            name: c_string_to_string_lossy(process.name.as_ptr()),
            cpu_usage: process.cpu_usage as f32,
            memory_usage: process.memory_usage,
            start_time: process.start_time,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileContentData {
    pub content: String,
    pub size: usize,
    pub success: bool,
    pub error: String,
}

impl FileContentData {
    unsafe fn from_c_struct(content: &FileContent) -> Self {
        Self {
            content: if !content.content.is_null() {
                c_string_to_string_lossy(content.content)
            } else {
                String::new()
            },
            size: content.size,
            success: content.success,
            error: c_string_to_string_lossy(content.error.as_ptr()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandResultData {
    pub output: String,
    pub exit_code: i32,
    pub success: bool,
    pub error: String,
}

impl CommandResultData {
    unsafe fn from_c_struct(result: &CommandResult) -> Self {
        Self {
            output: if !result.output.is_null() {
                c_string_to_string_lossy(result.output)
            } else {
                String::new()
            },
            exit_code: result.exit_code,
            success: result.success,
            error: c_string_to_string_lossy(result.error.as_ptr()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SystemInfoData {
    pub os_type: String,
    pub os_version: String,
    pub hostname: String,
    pub uptime: u64,
    pub cpu_cores: u32,
    pub total_memory: u64,
    pub available_memory: u64,
}

impl SystemInfoData {
    unsafe fn from_c_struct(info: &SystemInfo) -> Self {
        Self {
            os_type: c_string_to_string_lossy(info.os_type.as_ptr()),
            os_version: c_string_to_string_lossy(info.os_version.as_ptr()),
            hostname: c_string_to_string_lossy(info.hostname.as_ptr()),
            uptime: info.uptime,
            cpu_cores: info.cpu_cores,
            total_memory: info.total_memory,
            available_memory: info.available_memory,
        }
    }
}

impl Drop for SafePluginInterface {
    fn drop(&mut self) {
        self.cleanup();
    }
}
