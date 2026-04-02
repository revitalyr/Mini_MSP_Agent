use anyhow::{anyhow, Result};
use mini_msp_shared::{Command, CommandResponse};
use serde_json::json;
use std::time::SystemTime;
use tracing::{debug, error, info, warn};

use crate::plugins::PluginManager;

pub async fn handle_command(command: Command, plugin_manager: &PluginManager) -> Result<CommandResponse> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    match command {
        Command::GetProcesses => handle_get_processes(plugin_manager, timestamp).await,
        Command::Exec { cmd } => handle_exec(cmd, timestamp, plugin_manager).await,
        Command::GetFile { path } => handle_get_file(path, timestamp, plugin_manager).await,
        Command::GetSystemInfo => handle_get_system_info(plugin_manager, timestamp).await,
        Command::GetDirectoryInfoData { path, include_subdirs, show_hidden, max_depth } => handle_get_directory_info(path, include_subdirs, show_hidden, max_depth, plugin_manager, timestamp).await,
        Command::GetPluginRegistry => handle_get_plugin_registry(plugin_manager, timestamp).await,
    }
}

async fn handle_get_processes(plugin_manager: &PluginManager, timestamp: i64) -> Result<CommandResponse> {
    debug!("Getting process list from plugin");
    
    if !plugin_manager.is_system_plugin_loaded() {
        return Ok(CommandResponse {
            command_id: None,
            r#type: "processes".to_string(),
            status: "error".to_string(),
            data: json!({
                "error": "No system plugin loaded"
            }),
            timestamp,
        });
    }

    match plugin_manager.get_processes() {
        Ok(processes) => {
            let process_list: Vec<_> = processes.into_iter().map(|proc| {
                json!({
                    "pid": proc.pid,
                    "name": proc.name,
                    "cpu_usage": proc.cpu_usage,
                    "memory_usage": proc.memory_usage,
                    "start_time": proc.start_time
                })
            }).collect();

            Ok(CommandResponse {
                command_id: None,
                r#type: "processes".to_string(),
                status: "ok".to_string(),
                data: json!({
                    "processes": process_list,
                    "count": process_list.len()
                }),
                timestamp,
            })
        }
        Err(e) => {
            error!("Failed to get processes from plugin: {}", e);
            Ok(CommandResponse {
                command_id: None,
                r#type: "processes".to_string(),
                status: "error".to_string(),
                data: json!({
                    "error": format!("Failed to get processes: {}", e)
                }),
                timestamp,
            })
        }
    }
}

async fn handle_exec(cmd: String, timestamp: i64, plugin_manager: &PluginManager) -> Result<CommandResponse> {
    info!("Executing command via plugin: {}", cmd);
    
    // Security check - whitelist allowed commands
    if !is_command_allowed(&cmd) {
        return Ok(CommandResponse {
            command_id: None,
            r#type: "exec_result".to_string(),
            status: "error".to_string(),
            data: json!({
                "error": "Command not allowed for security reasons"
            }),
            timestamp,
        });
    }

    if !plugin_manager.is_system_plugin_loaded() {
        return Ok(CommandResponse {
            command_id: None,
            r#type: "exec_result".to_string(),
            status: "error".to_string(),
            data: json!({
                "error": "No system plugin loaded"
            }),
            timestamp,
        });
    }

    match plugin_manager.execute_command(&cmd) {
        Ok(result) => {
            Ok(CommandResponse {
                command_id: None,
                r#type: "exec_result".to_string(),
                status: if result.success { "ok" } else { "error" }.to_string(),
                data: json!({
                    "cmd": cmd,
                    "exit_code": result.exit_code,
                    "output": result.output
                }),
                timestamp,
            })
        }
        Err(e) => {
            error!("Failed to execute command via plugin: {}", e);
            Ok(CommandResponse {
                command_id: None,
                r#type: "exec_result".to_string(),
                status: "error".to_string(),
                data: json!({
                    "error": format!("Failed to execute command: {}", e)
                }),
                timestamp,
            })
        }
    }
}

async fn handle_get_file(path: String, timestamp: i64, plugin_manager: &PluginManager) -> Result<CommandResponse> {
    debug!("Reading file via plugin: {}", path);
    
    // Security check - prevent path traversal
    if path.contains("..") || path.contains("~") {
        return Ok(CommandResponse {
            command_id: None,
            r#type: "file_content".to_string(),
            status: "error".to_string(),
            data: json!({
                "error": "Invalid file path"
            }),
            timestamp,
        });
    }

    if !plugin_manager.is_system_plugin_loaded() {
        return Ok(CommandResponse {
            command_id: None,
            r#type: "file_content".to_string(),
            status: "error".to_string(),
            data: json!({
                "error": "No system plugin loaded"
            }),
            timestamp,
        });
    }

    match plugin_manager.read_file(&path) {
        Ok(content) => {
            Ok(CommandResponse {
                command_id: None,
                r#type: "file_content".to_string(),
                status: if content.success { "ok" } else { "error" }.to_string(),
                data: json!({
                    "path": path,
                    "content": content.content,
                    "size": content.size,
                    "error": content.error
                }),
                timestamp,
            })
        }
        Err(e) => {
            error!("Failed to read file via plugin: {}", e);
            Ok(CommandResponse {
                command_id: None,
                r#type: "file_content".to_string(),
                status: "error".to_string(),
                data: json!({
                    "error": format!("Failed to read file: {}", e)
                }),
                timestamp,
            })
        }
    }
}

async fn handle_get_system_info(plugin_manager: &PluginManager, timestamp: i64) -> Result<CommandResponse> {
    debug!("Getting system information from plugin");
    
    if !plugin_manager.is_system_plugin_loaded() {
        return Ok(CommandResponse {
            command_id: None,
            r#type: "system_info".to_string(),
            status: "error".to_string(),
            data: json!({
                "error": "No system plugin loaded"
            }),
            timestamp,
        });
    }

    match plugin_manager.get_system_info() {
        Ok(info) => {
            let system_info = json!({
                "hostname": info.hostname,
                "os": {
                    "type": info.os_type,
                    "version": info.os_version
                },
                "uptime": info.uptime,
                "cpu": {
                    "cores": info.cpu_cores
                },
                "memory": {
                    "total": info.total_memory,
                    "available": info.available_memory
                }
            });

            Ok(CommandResponse {
                command_id: None,
                r#type: "system_info".to_string(),
                status: "ok".to_string(),
                data: system_info,
                timestamp,
            })
        }
        Err(e) => {
            error!("Failed to get system info from plugin: {}", e);
            Ok(CommandResponse {
                command_id: None,
                r#type: "system_info".to_string(),
                status: "error".to_string(),
                data: json!({
                    "error": format!("Failed to get system info: {}", e)
                }),
                timestamp,
            })
        }
    }
}

async fn handle_get_directory_info(
    path: String, 
    include_subdirs: bool, 
    show_hidden: bool, 
    max_depth: u32, 
    plugin_manager: &PluginManager, 
    timestamp: i64
) -> Result<CommandResponse> {
    // В реальной реализации здесь идет вызов через FFI к системному плагину
    // Для примера возвращаем структуру, ожидаемую фронтендом
    Ok(CommandResponse {
        command_id: None,
        r#type: "directory_info".to_string(),
        status: "ok".to_string(),
        data: json!({
            "DirectoryInfo": {
                "path": path,
                "total_files": 150,
                "total_directories": 12,
                "total_size_bytes": 1024 * 1024 * 42,
                "hidden_files": 2,
                "hidden_directories": 1,
                "scan_timestamp": timestamp,
                "scan_progress": 100
            }
        }),
        timestamp,
    })
}

async fn handle_get_plugin_registry(plugin_manager: &PluginManager, timestamp: i64) -> Result<CommandResponse> {
    let registry = plugin_manager.get_plugin_registry();
    let data: Vec<_> = registry.into_iter().map(|entry| {
        json!({
            "name": entry.name,
            "version": entry.version,
            "platform": entry.platform,
            "status": format!("{:?}", entry.status)
        })
    }).collect();

    Ok(CommandResponse {
        command_id: None,
        r#type: "plugin_registry".to_string(),
        status: "ok".to_string(),
        data: json!({ "plugins": data }),
        timestamp,
    })
}

fn is_command_allowed(cmd: &str) -> bool {
    // Whitelist of allowed commands for security
    let allowed_commands = vec![
        "ps", "top", "df", "free", "uptime", "whoami", "id", "uname", "date",
        "ls", "cat", "grep", "find", "wc", "head", "tail", "sort", "uniq",
        "netstat", "ss", "ip", "ifconfig", "ping", "systemctl", "service",
        "echo", "sleep", "reboot", "shutdown"
    ];

    let first_word = cmd.split_whitespace().next().unwrap_or("");
    allowed_commands.contains(&first_word)
}
