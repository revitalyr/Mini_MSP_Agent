//! System information endpoints
//! 
//! Предоставление информации о системе

use axum::{response::Json, http::StatusCode};
use serde_json::{json, Value};
use std::process::Command;

/// Get system information
pub async fn get_system_info() -> Result<Json<Value>, StatusCode> {
    let os_info = get_os_info();
    let hostname = get_hostname();
    
    Ok(Json(json!({
        "platform": os_info.platform,
        "platform_name": os_info.name,
        "icon": os_info.icon,
        "hostname": hostname,
        "architecture": os_info.architecture,
        "version": os_info.version
    })))
}

struct OSInfo {
    platform: String,
    name: String,
    icon: String,
    architecture: String,
    version: String,
}

fn get_os_info() -> OSInfo {
    #[cfg(target_os = "windows")]
    {
        OSInfo {
            platform: "windows".to_string(),
            name: "Windows".to_string(),
            icon: "🪟".to_string(),
            architecture: "x64".to_string(),
            version: get_windows_version(),
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        OSInfo {
            platform: "linux".to_string(),
            name: "Linux".to_string(),
            icon: "🐧".to_string(),
            architecture: "x64".to_string(),
            version: get_linux_version(),
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        OSInfo {
            platform: "macos".to_string(),
            name: "macOS".to_string(),
            icon: "🍎".to_string(),
            architecture: "x64".to_string(),
            version: get_macos_version(),
        }
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        OSInfo {
            platform: "unknown".to_string(),
            name: "Unknown".to_string(),
            icon: "❓".to_string(),
            architecture: "unknown".to_string(),
            version: "unknown".to_string(),
        }
    }
}

#[cfg(target_os = "windows")]
fn get_windows_version() -> String {
    match Command::new("cmd").args(&["/C", "ver"]).output() {
        Ok(output) => {
            let version_str = String::from_utf8_lossy(&output.stdout);
            version_str.trim().to_string()
        }
        Err(_) => "Unknown Windows".to_string(),
    }
}

#[cfg(target_os = "linux")]
fn get_linux_version() -> String {
    match Command::new("uname").args(&["-r"]).output() {
        Ok(output) => {
            let version_str = String::from_utf8_lossy(&output.stdout);
            version_str.trim().to_string()
        }
        Err(_) => "Unknown Linux".to_string(),
    }
}

#[cfg(target_os = "macos")]
fn get_macos_version() -> String {
    match Command::new("sw_vers").args(&["-productVersion"]).output() {
        Ok(output) => {
            let version_str = String::from_utf8_lossy(&output.stdout);
            version_str.trim().to_string()
        }
        Err(_) => "Unknown macOS".to_string(),
    }
}

fn get_hostname() -> String {
    match Command::new("hostname").output() {
        Ok(output) => {
            let hostname_str = String::from_utf8_lossy(&output.stdout);
            hostname_str.trim().to_string()
        }
        Err(_) => "unknown".to_string(),
    }
}
