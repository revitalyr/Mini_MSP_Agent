use anyhow::{anyhow, Result};
use mini_msp_shared::{Command, CommandResponse, AgentResponse};
use serde_json::json;
use std::time::SystemTime;
use tracing::{debug, error, info};

use crate::plugins::PluginManager;
use crate::security::SecurityPolicy;
use crate::config::Config;

#[derive(Clone, Copy)]
pub struct ExecutionContext<'a> {
    pub plugin_manager: &'a PluginManager,
    pub policy: &'a SecurityPolicy,
    pub config: &'a Config,
    pub command_timeout_secs: u64,
}

pub async fn handle_command(command: Command, command_id: Option<String>, ctx: ExecutionContext<'_>) -> Result<AgentResponse> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let plugin_manager = ctx.plugin_manager;

    let mut response = match command {
        Command::GetProcesses => handle_get_processes(ctx, timestamp).await,
        Command::Exec { cmd } => handle_exec(cmd, timestamp, ctx).await,
        Command::GetFile { path } => handle_get_file(path, timestamp, ctx).await,
        Command::GetSystemInfo => handle_get_system_info(ctx, timestamp).await,
        Command::GetDirectoryInfoData { path, include_subdirs, show_hidden, max_depth } => handle_get_directory_info(path, include_subdirs, show_hidden, max_depth, ctx, timestamp).await,
        Command::GetPluginRegistry => handle_get_plugin_registry(plugin_manager, timestamp).await,
        Command::GetEventData { path } => handle_get_event_data(path, timestamp).await,
        Command::GetWatchersData => handle_get_watchers_data(timestamp).await,
        Command::GetFileReaderData { path } => handle_get_file_reader_data(path, timestamp).await,
        Command::GetSensorData => handle_get_sensor_data(timestamp).await,
        Command::GetCameraData => handle_get_camera_data(timestamp).await,
        Command::GetProcessingResults => handle_get_processing_results(timestamp).await,
        Command::GetVideoFrame => handle_get_video_frame(ctx, timestamp).await,
    }?;

    response.command_id = command_id;
    Ok(AgentResponse::Json(response))
}

async fn handle_get_processes(ctx: ExecutionContext<'_>, timestamp: i64) -> Result<CommandResponse> {
    debug!("Getting process list from plugin");
    
    if !ctx.plugin_manager.is_system_plugin_loaded() {
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

    match ctx.plugin_manager.get_processes() {
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

async fn handle_exec(cmd: String, timestamp: i64, ctx: ExecutionContext<'_>) -> Result<CommandResponse> {
    info!("Executing command via plugin: {}", cmd);
    
    // Security check - whitelist allowed commands
    if !ctx.policy.is_command_allowed(&cmd) {
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

    if !ctx.plugin_manager.is_system_plugin_loaded() {
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

    match ctx.plugin_manager.execute_command(&cmd) {
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

async fn handle_get_file(path: String, timestamp: i64, ctx: ExecutionContext<'_>) -> Result<CommandResponse> {
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

    // Security check - file size limit using policy
    if !ctx.policy.is_file_size_allowed(&path) {
        return Ok(CommandResponse {
            command_id: None,
            r#type: "file_content".to_string(),
            status: "error".to_string(),
            data: json!({
                "error": format!("File too large or inaccessible (limit: {} bytes)", ctx.policy.max_file_size)
            }),
            timestamp,
        });
    }

    if !ctx.plugin_manager.is_system_plugin_loaded() {
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

    match ctx.plugin_manager.read_file(&path) {
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

async fn handle_get_system_info(ctx: ExecutionContext<'_>, timestamp: i64) -> Result<CommandResponse> {
    debug!("Getting system information from plugin");
    
    if !ctx.plugin_manager.is_system_plugin_loaded() {
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

    match ctx.plugin_manager.get_system_info() {
        Ok(info) => {
            let system_info_data = json!({
                "SystemInfo": {
                    "hostname": info.hostname,
                    "os_type": info.os_type,
                    "os_version": info.os_version,
                    "uptime": info.uptime,
                    "cpu_cores": info.cpu_cores,
                    "total_memory": info.total_memory,
                    "available_memory": info.available_memory,
                    "server_info": {
                        "url": ctx.config.server_url,
                        "ws_url": ctx.config.ws_url,
                        "check_interval": ctx.config.interval
                    }
                }
            });

            Ok(CommandResponse {
                command_id: None,
                r#type: "system_info".to_string(),
                status: "ok".to_string(),
                data: system_info_data,
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
    ctx: ExecutionContext<'_>,
    timestamp: i64,
) -> Result<CommandResponse> {
    debug!("Scanning directory via plugin: {}", path);
    
    if !ctx.plugin_manager.is_system_plugin_loaded() {
        return Ok(CommandResponse {
            command_id: None,
            r#type: "directory_info".to_string(),
            status: "error".to_string(),
            data: json!({ "error": "No system plugin loaded" }),
            timestamp,
        });
    }

    match ctx.plugin_manager.get_directory_info_data(&path, include_subdirs, show_hidden, max_depth) {
        Ok(info) => {
            Ok(CommandResponse {
                command_id: None,
                r#type: "directory_info".to_string(),
                status: "ok".to_string(),
                data: json!({
                    "DirectoryInfo": {
                        "path": info.path,
                        "total_files": info.total_files,
                        "total_directories": info.total_directories,
                        "total_size": info.total_size_bytes,
                        "hidden_files": info.hidden_files,
                        "hidden_directories": info.hidden_directories,
                        "timestamp": info.scan_timestamp,
                        "progress": info.scan_progress
                    }
                }),
                timestamp,
            })
        }
        Err(e) => {
            error!("Failed to scan directory: {}", e);
            Ok(CommandResponse {
                command_id: None,
                r#type: "directory_info".to_string(),
                status: "error".to_string(),
                data: json!({ "error": e.to_string() }),
                timestamp,
            })
        }
    }
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

async fn handle_get_event_data(path: String, timestamp: i64) -> Result<CommandResponse> {
    Ok(CommandResponse {
        command_id: None,
        r#type: "event_data".to_string(),
        status: "ok".to_string(),
        data: json!({
            "EventData": {
                "path": path,
                "events_count": 1250,
                "buffer_usage": 45,
                "last_event": "FileModified",
                "timestamp": timestamp
            }
        }),
        timestamp,
    })
}

async fn handle_get_watchers_data(timestamp: i64) -> Result<CommandResponse> {
    Ok(CommandResponse {
        command_id: None,
        r#type: "watchers_data".to_string(),
        status: "ok".to_string(),
        data: json!({
            "WatchersData": {
                "active_watchers": 3,
                "total_notifications": 89,
                "cpu_usage": 0.5,
                "memory_usage_kb": 1240
            }
        }),
        timestamp,
    })
}

async fn handle_get_file_reader_data(path: String, timestamp: i64) -> Result<CommandResponse> {
    Ok(CommandResponse {
        command_id: None,
        r#type: "file_reader_data".to_string(),
        status: "ok".to_string(),
        data: json!({
            "FileReaderData": {
                "path": path,
                "size": 4096,
                "encoding": "UTF-8",
                "is_locked": false,
                "last_access": timestamp
            }
        }),
        timestamp,
    })
}

async fn handle_get_sensor_data(timestamp: i64) -> Result<CommandResponse> {
    Ok(CommandResponse {
        command_id: None,
        r#type: "sensor_data".to_string(),
        status: "ok".to_string(),
        data: json!({ 
            "SensorInfo": {
                "temperature": 24.5, 
                "humidity": 50.0, 
                "status": "online",
                "unit": "celsius"
            }
        }),
        timestamp,
    })
}

async fn handle_get_camera_data(timestamp: i64) -> Result<CommandResponse> {
    Ok(CommandResponse {
        command_id: None,
        r#type: "camera_data".to_string(),
        status: "ok".to_string(),
        data: json!({ 
            "CameraInfo": {
                "fps": 30, 
                "resolution": "1920x1080", 
                "status": "streaming",
                "is_recording": false
            }
        }),
        timestamp,
    })
}

async fn handle_get_processing_results(timestamp: i64) -> Result<CommandResponse> {
    Ok(CommandResponse {
        command_id: None,
        r#type: "processing_results".to_string(),
        status: "ok".to_string(),
        data: json!({
            "ProcessingInfo": {
                "status": "active",
                "items_processed": 5000,
                "efficiency": 0.98,
                "last_task": "image_classification"
            }
        }),
        timestamp,
    })
}

async fn handle_get_video_frame(ctx: ExecutionContext<'_>, _timestamp: i64) -> Result<CommandResponse> {
    ctx.plugin_manager.get_video_frame()
        .map(|f| CommandResponse {
            command_id: None,
            r#type: "video_frame".to_string(),
            status: "ok".to_string(),
            data: json!({
                "VideoInfo": {
                    "width": f.width,
                    "height": f.height,
                    "size": f.data.len()
                }
            }),
            timestamp: f.timestamp as i64,
        })
        .map_err(|e| anyhow!("Failed to get video frame: {}", e))
}
