//! Simple System Information Application
//! 
//! Loads plugins based on the running OS and logs system information to out/<system name>/info.json

use mini_msp_shared::{Plugin, SystemMetrics, Command, CommandResponse};
use std::path::Path;
use std::fs;
use std::env;
use serde_json::json;
use std::process::Command as ProcessCommand;

// Include the system_info_plugin module
#[path = "system_info_plugin.rs"]
mod system_info_plugin;
use system_info_plugin::SystemInfoPlugin;

// System information structure
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SystemInfo {
    system_name: String,
    os_type: String,
    os_version: String,
    architecture: String,
    hostname: String,
    timestamp: u64,
    plugins_loaded: Vec<PluginInfo>,
    system_metrics: Option<SystemMetrics>,
    additional_info: serde_json::Value,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PluginInfo {
    name: String,
    version: String,
    file_path: String,
    status: String,
}

// Real plugin wrapper for OS-specific plugins
#[derive(Debug, Clone)]
struct OsPlugin {
    name: String,
    version: String,
    file_path: String,
    os_type: String,
}

impl OsPlugin {
    fn new(file_name: String, file_path: String, os_type: String) -> Self {
        let name = file_name.replace(".cpp", "").replace("_", " ");
        Self {
            name: name.clone(),
            version: "1.0.0".to_string(),
            file_path,
            os_type,
        }
    }
}

#[async_trait::async_trait]
impl Plugin for OsPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        &self.version
    }
    
    async fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Initializing OS plugin: {} for {}", self.name, self.os_type);
        Ok(())
    }
    
    async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Shutting down OS plugin: {}", self.name);
        Ok(())
    }
    
    async fn get_metrics(&self) -> Option<SystemMetrics> {
        Some(SystemMetrics {
            cpu_usage: get_cpu_usage() as f32,
            memory_usage: get_memory_usage(),
            disk_usage: get_disk_usage() as f32,
            uptime: get_uptime(),
        })
    }
    
    async fn handle_command(&self, cmd: &Command) -> Result<CommandResponse, Box<dyn std::error::Error>> {
        match cmd {
            Command::GetSystemInfo => {
                let system_info = get_system_info_json();
                Ok(CommandResponse {
                    command_id: Some("system_info".to_string()),
                    r#type: "system_info_response".to_string(),
                    status: "success".to_string(),
                    data: system_info,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                })
            },
            Command::GetProcesses => {
                let processes = get_running_processes();
                Ok(CommandResponse {
                    command_id: Some("processes".to_string()),
                    r#type: "processes_response".to_string(),
                    status: "success".to_string(),
                    data: json!({"processes": processes}),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                })
            },
            _ => Ok(CommandResponse {
                command_id: Some("unknown".to_string()),
                r#type: "error".to_string(),
                status: "error".to_string(),
                data: json!({"error": "Unsupported command"}),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            })
        }
    }
    
    fn clone_box(&self) -> Box<dyn Plugin> {
        Box::new(self.clone())
    }
    
    fn eq_box(&self, other: &Box<dyn Plugin>) -> bool {
        self.name == other.name() && self.version == other.version()
    }
    
    fn serialize_box(&self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        Ok(json!({
            "name": self.name,
            "version": self.version,
            "type": "OsPlugin",
            "file_path": self.file_path,
            "os_type": self.os_type
        }))
    }
    
    fn deserialize_box(&self, data: &serde_json::Value) -> Result<Box<dyn Plugin>, Box<dyn std::error::Error>> {
        let name = data.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
        let version = data.get("version").and_then(|v| v.as_str()).unwrap_or("1.0.0");
        let file_path = data.get("file_path").and_then(|v| v.as_str()).unwrap_or("unknown");
        let os_type = data.get("os_type").and_then(|v| v.as_str()).unwrap_or("unknown");
        Ok(Box::new(OsPlugin {
            name: name.to_string(),
            version: version.to_string(),
            file_path: file_path.to_string(),
            os_type: os_type.to_string(),
        }))
    }
}

// OS detection functions
fn get_os_type() -> String {
    env::consts::OS.to_string()
}

fn get_system_name() -> String {
    match get_os_type().as_str() {
        "macos" => {
            let output = ProcessCommand::new("scutil")
                .arg("--get")
                .arg("ComputerName")
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_else(|_| "Mac".to_string());
            output
        },
        "linux" => {
            let output = ProcessCommand::new("hostname")
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_else(|_| "Linux".to_string());
            output
        },
        "windows" => {
            let output = ProcessCommand::new("hostname")
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_else(|_| "Windows".to_string());
            output
        },
        _ => "Unknown".to_string(),
    }
}

fn get_os_version() -> String {
    match get_os_type().as_str() {
        "macos" => {
            let output = ProcessCommand::new("sw_vers")
                .arg("-productVersion")
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_else(|_| "Unknown".to_string());
            output
        },
        "linux" => {
            if let Ok(content) = fs::read_to_string("/etc/os-release") {
                for line in content.lines() {
                    if line.starts_with("PRETTY_NAME=") {
                        return line.split('=').nth(1)
                            .unwrap_or("Unknown")
                            .trim_matches('"')
                            .to_string();
                    }
                }
            }
            "Unknown Linux".to_string()
        },
        "windows" => {
            let output = ProcessCommand::new("cmd")
                .args(&["/c", "ver"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_else(|_| "Unknown Windows".to_string());
            output
        },
        _ => "Unknown".to_string(),
    }
}

fn get_cpu_usage() -> f64 {
    // Simple CPU usage simulation - in real implementation would use system APIs
    25.5
}

fn get_memory_usage() -> u64 {
    // Simple memory usage simulation - in real implementation would use system APIs
    8192
}

fn get_disk_usage() -> f64 {
    // Simple disk usage simulation - in real implementation would use system APIs
    65.2
}

fn get_uptime() -> u64 {
    match get_os_type().as_str() {
        "macos" => {
            let _output = ProcessCommand::new("uptime")
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_else(|_| "0".to_string());
            // Parse uptime from output (simplified)
            3600
        },
        "linux" => {
            if let Ok(content) = fs::read_to_string("/proc/uptime") {
                content.split_whitespace().next()
                    .and_then(|s| s.parse::<f64>().ok())
                    .map(|u| u as u64)
                    .unwrap_or(0)
            } else {
                0
            }
        },
        "windows" => {
            // Windows uptime would be retrieved via system calls
            3600
        },
        _ => 0,
    }
}

fn get_system_info_json() -> serde_json::Value {
    json!({
        "os_type": get_os_type(),
        "os_version": get_os_version(),
        "architecture": env::consts::ARCH,
        "hostname": get_system_name(),
        "cpu_cores": num_cpus::get(),
        "total_memory": get_memory_usage(),
        "uptime": get_uptime()
    })
}

fn get_running_processes() -> Vec<serde_json::Value> {
    match get_os_type().as_str() {
        "macos" => {
            let output = ProcessCommand::new("ps")
                .args(&["-eo", "pid,comm"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_else(|_| String::new());
            
            let mut processes = Vec::new();
            for line in output.lines().skip(1) {
                if let Some(pid) = line.split_whitespace().next() {
                    let command_str = line.replace(pid, "").trim().to_string();
                    if !command_str.is_empty() {
                        processes.push(json!({
                            "pid": pid,
                            "command": command_str
                        }));
                    }
                }
            }
            processes.truncate(10); // Limit to first 10 processes
            processes
        },
        "linux" => {
            let output = ProcessCommand::new("ps")
                .args(&["-eo", "pid,comm"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_else(|_| String::new());
            
            let mut processes = Vec::new();
            for line in output.lines().skip(1) {
                if let Some(pid) = line.split_whitespace().next() {
                    let command_str = line.replace(pid, "").trim().to_string();
                    if !command_str.is_empty() {
                        processes.push(json!({
                            "pid": pid,
                            "command": command_str
                        }));
                    }
                }
            }
            processes.truncate(10); // Limit to first 10 processes
            processes
        },
        "windows" => {
            let output = ProcessCommand::new("tasklist")
                .args(&["/fo", "csv", "/nh"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_else(|_| String::new());
            
            let mut processes = Vec::new();
            for line in output.lines() {
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() >= 2 {
                    let image_name = parts[0].trim_matches('"');
                    let pid = parts[1].trim_matches('"');
                    processes.push(json!({
                        "pid": pid,
                        "command": image_name
                    }));
                }
            }
            processes.truncate(10); // Limit to first 10 processes
            processes
        },
        _ => Vec::new(),
    }
}

fn load_os_plugins() -> Vec<Box<dyn Plugin>> {
    let mut loaded_plugins = Vec::new();
    let _plugins_dir = Path::new("src/plugins/src");
    let os_type = get_os_type();
    
    println!("Loading plugins for OS: {}", os_type);
    
    // First load C++ plugins, then add SystemInfoPlugin with them
    let mut cpp_plugins = Vec::new();
    let plugins_dir = Path::new("src/plugins/src");
    
    if plugins_dir.exists() {
        // Load C++ plugins first
        if let Ok(entries) = fs::read_dir(plugins_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("cpp") {
                    // Skip OS-specific directories
                    if path.parent().and_then(|p| p.file_name()) == Some(std::ffi::OsStr::new(os_type.as_str())) {
                        continue;
                    }
                    
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        println!("Found C++ plugin: {}", file_name);
                        let plugin = OsPlugin::new(file_name.to_string(), path.to_string_lossy().to_string(), os_type.clone());
                        cpp_plugins.push(Box::new(plugin) as Box<dyn Plugin>);
                    }
                }
            }
        }
    }
    
    // Add SystemInfoPlugin for sys_info.md parsing with C++ plugins
    let sys_info_path = "sys_info.md".to_string();
    if Path::new(&sys_info_path).exists() {
        println!("Adding SystemInfoPlugin for sys_info.md parsing with {} C++ plugins", cpp_plugins.len());
        let system_info_plugin = SystemInfoPlugin::new_with_plugins(sys_info_path, os_type.clone(), cpp_plugins);
        loaded_plugins.push(Box::new(system_info_plugin) as Box<dyn Plugin>);
    } else {
        println!("Warning: sys_info.md not found at {}", sys_info_path);
    }
    
        println!("Loaded {} plugins", loaded_plugins.len());
    loaded_plugins
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== System Information Application ===");
    
    // Detect OS and system information
    let os_type = get_os_type();
    let system_name = get_system_name();
    let os_version = get_os_version();
    
    println!("Detected OS: {} {}", os_type, os_version);
    println!("System name: {}", system_name);
    
    // Load OS-specific plugins
    let plugins = load_os_plugins();
    
    // Initialize all plugins
    let mut initialized_plugins = Vec::new();
    for mut plugin in plugins {
        if let Err(e) = plugin.init().await {
            println!("Failed to initialize plugin {}: {}", plugin.name(), e);
        } else {
            println!("Successfully initialized plugin: {}", plugin.name());
            initialized_plugins.push(plugin);
        }
    }
    
    // Collect system information from plugins
    let mut system_metrics = None;
    let mut plugin_infos = Vec::new();
    
    for plugin in &initialized_plugins {
        // Get plugin info
        plugin_infos.push(PluginInfo {
            name: plugin.name().to_string(),
            version: plugin.version().to_string(),
            file_path: if let Ok(serialized) = plugin.serialize_box() {
                serialized.get("file_path").and_then(|v| v.as_str()).unwrap_or("unknown").to_string()
            } else {
                "unknown".to_string()
            },
            status: "initialized".to_string(),
        });
        
        // Get metrics
        if system_metrics.is_none() {
            system_metrics = plugin.get_metrics().await;
        }
    }
    
    // Get additional system information
    let additional_info = get_system_info_json();
    
    // Create system info structure
    let system_info = SystemInfo {
        system_name: system_name.clone(),
        os_type: os_type.clone(),
        os_version,
        architecture: env::consts::ARCH.to_string(),
        hostname: system_name.clone(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        plugins_loaded: plugin_infos,
        system_metrics,
        additional_info,
    };
    
    // Create output directory
    let output_dir = Path::new("out").join(&system_name);
    fs::create_dir_all(&output_dir)?;
    
    // Write system information to JSON file
    let output_file = output_dir.join("info.json");
    let json_output = serde_json::to_string_pretty(&system_info)?;
    fs::write(&output_file, json_output)?;
    
    println!("System information written to: {}", output_file.display());
    
    // Test plugin commands
    for plugin in &initialized_plugins {
        println!("\n--- Testing plugin: {} ---", plugin.name());
        
        // Test system info command
        if let Ok(response) = plugin.handle_command(&Command::GetSystemInfo).await {
            println!("System info response: {}", response.status);
        }
        
        // Test processes command
        if let Ok(response) = plugin.handle_command(&Command::GetProcesses).await {
            println!("Processes response: {}", response.status);
        }
    }
    
    // Shutdown all plugins
    for mut plugin in initialized_plugins {
        if let Err(e) = plugin.shutdown().await {
            println!("Failed to shutdown plugin {}: {}", plugin.name(), e);
        } else {
            println!("Successfully shutdown plugin: {}", plugin.name());
        }
    }
    
    println!("\n=== Application Complete ===");
    Ok(())
}
