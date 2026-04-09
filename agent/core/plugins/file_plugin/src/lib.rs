use anyhow::Result;
use chrono::{DateTime, Utc};
use core_shared::{Plugin, PluginInfo, PluginStatus, FileInfo};
use nix::sys::stat::{stat, lstat, SFlag};
use nix::unistd::{getpwuid, getgrgid};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use tracing::{info, warn, error};
use uuid::Uuid;

pub struct FilePlugin {
    info: PluginInfo,
    base_path: PathBuf,
}

impl FilePlugin {
    pub fn new() -> Self {
        Self {
            info: PluginInfo {
                name: "file_plugin".to_string(),
                version: "1.0.0".to_string(),
                description: "File system operations and monitoring plugin".to_string(),
                author: "MSP Agent Team".to_string(),
                status: PluginStatus::Unloaded,
                loaded_at: None,
                last_error: None,
            },
            base_path: PathBuf::from("/"),
        }
    }

    fn get_file_info(&self, path: &Path) -> Result<FileInfo> {
        let metadata = fs::metadata(path)?;
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        
        let is_directory = metadata.is_dir();
        let is_file = metadata.is_file();
        let is_symlink = metadata.file_type().is_symlink();
        
        let modified = DateTime::from_timestamp(metadata.modified()?.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64, 0)
            .unwrap_or_else(Utc::now);
        
        let accessed = metadata.accessed()
            .ok()
            .and_then(|t| DateTime::from_timestamp(t.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64, 0));
        
        let created = metadata.created()
            .ok()
            .and_then(|t| DateTime::from_timestamp(t.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64, 0));
        
        let permissions = format!("{:o}", metadata.permissions().mode());
        
        let (owner, group) = if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
            let owner_name = getpwuid(metadata.uid())
                .map(|pw| pw.name.to_string_lossy().to_string())
                .unwrap_or_else(|_| metadata.uid().to_string());
            
            let group_name = getgrgid(metadata.gid())
                .map(|gr| gr.name.to_string_lossy().to_string())
                .unwrap_or_else(|_| metadata.gid().to_string());
            
            (Some(owner_name), Some(group_name))
        } else {
            (None, None)
        };
        
        Ok(FileInfo {
            path: path.to_string_lossy().to_string(),
            name,
            size: metadata.len(),
            is_directory,
            is_file,
            is_symlink,
            permissions,
            modified,
            accessed,
            created,
            owner,
            group,
        })
    }

    fn list_directory(&self, path: &Path, include_hidden: bool, max_entries: usize) -> Result<Vec<FileInfo>> {
        let mut entries = Vec::new();
        let mut count = 0;
        
        for entry in fs::read_dir(path)? {
            if count >= max_entries {
                break;
            }
            
            let entry = entry?;
            let file_name = entry.file_name();
            
            // Skip hidden files unless requested
            if !include_hidden && file_name.to_string_lossy().starts_with('.') {
                continue;
            }
            
            match self.get_file_info(&entry.path()) {
                Ok(info) => {
                    entries.push(info);
                    count += 1;
                }
                Err(e) => {
                    warn!("Failed to get info for {}: {}", entry.path().display(), e);
                }
            }
        }
        
        // Sort entries: directories first, then files, both alphabetically
        entries.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
        
        Ok(entries)
    }
}

impl Plugin for FilePlugin {
    fn name(&self) -> &str {
        &self.info.name
    }

    fn version(&self) -> &str {
        &self.info.version
    }

    fn description(&self) -> &str {
        &self.info.description
    }

    async fn initialize(&mut self) -> Result<()> {
        info!("Initializing file plugin");
        
        // Set base path to current directory or root
        self.base_path = PathBuf::from("/");
        
        self.info.status = PluginStatus::Loaded;
        self.info.loaded_at = Some(Utc::now());
        
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down file plugin");
        self.info.status = PluginStatus::Unloaded;
        self.info.loaded_at = None;
        Ok(())
    }

    async fn handle_command(&self, command: &str, params: HashMap<String, Value>) -> Result<Value> {
        match command {
            "list_directory" => self.handle_list_directory(params).await,
            "get_file_info" => self.handle_get_file_info(params).await,
            "read_file" => self.handle_read_file(params).await,
            "write_file" => self.handle_write_file(params).await,
            "create_directory" => self.handle_create_directory(params).await,
            "delete_file" => self.handle_delete_file(params).await,
            "move_file" => self.handle_move_file(params).await,
            "copy_file" => self.handle_copy_file(params).await,
            "get_disk_usage" => self.handle_get_disk_usage(params).await,
            _ => Err(anyhow::anyhow!("Unknown command: {}", command)),
        }
    }

    async fn get_metrics(&self) -> Result<core_shared::SystemMetrics> {
        // This plugin doesn't provide system metrics
        Err(anyhow::anyhow!("File plugin doesn't provide system metrics"))
    }
}

impl FilePlugin {
    async fn handle_list_directory(&self, params: HashMap<String, Value>) -> Result<Value> {
        let path = params.get("path")
            .and_then(|p| p.as_str())
            .unwrap_or("/");
        
        let include_hidden = params.get("include_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let max_entries = params.get("max_entries")
            .and_then(|v| v.as_u64())
            .unwrap_or(1000) as usize;
        
        let path = Path::new(path);
        if !path.exists() {
            return Err(anyhow::anyhow!("Path does not exist: {}", path.display()));
        }
        
        if !path.is_dir() {
            return Err(anyhow::anyhow!("Path is not a directory: {}", path.display()));
        }
        
        let entries = self.list_directory(path, include_hidden, max_entries)?;
        
        Ok(json!({
            "path": path.to_string_lossy(),
            "entries": entries,
            "count": entries.len(),
        }))
    }

    async fn handle_get_file_info(&self, params: HashMap<String, Value>) -> Result<Value> {
        let path = params.get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing path parameter"))?;
        
        let path = Path::new(path);
        if !path.exists() {
            return Err(anyhow::anyhow!("File does not exist: {}", path.display()));
        }
        
        let info = self.get_file_info(path)?;
        Ok(json!(info))
    }

    async fn handle_read_file(&self, params: HashMap<String, Value>) -> Result<Value> {
        let path = params.get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing path parameter"))?;
        
        let max_size = params.get("max_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(1024 * 1024) as usize; // 1MB default
        
        let path = Path::new(path);
        if !path.is_file() {
            return Err(anyhow::anyhow!("Path is not a file: {}", path.display()));
        }
        
        let metadata = fs::metadata(path)?;
        if metadata.len() > max_size as u64 {
            return Err(anyhow::anyhow!("File too large: {} bytes", metadata.len()));
        }
        
        let content = fs::read_to_string(path)?;
        
        Ok(json!({
            "path": path.to_string_lossy(),
            "content": content,
            "size": content.len(),
            "encoding": "utf-8",
        }))
    }

    async fn handle_write_file(&self, params: HashMap<String, Value>) -> Result<Value> {
        let path = params.get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing path parameter"))?;
        
        let content = params.get("content")
            .and_then(|c| c.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing content parameter"))?;
        
        let create_dirs = params.get("create_dirs")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let path = Path::new(path);
        
        if create_dirs {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
        }
        
        fs::write(path, content)?;
        
        Ok(json!({
            "path": path.to_string_lossy(),
            "bytes_written": content.len(),
            "success": true,
        }))
    }

    async fn handle_create_directory(&self, params: HashMap<String, Value>) -> Result<Value> {
        let path = params.get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing path parameter"))?;
        
        let recursive = params.get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let path = Path::new(path);
        
        if recursive {
            fs::create_dir_all(path)?;
        } else {
            fs::create_dir(path)?;
        }
        
        Ok(json!({
            "path": path.to_string_lossy(),
            "created": true,
        }))
    }

    async fn handle_delete_file(&self, params: HashMap<String, Value>) -> Result<Value> {
        let path = params.get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing path parameter"))?;
        
        let recursive = params.get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let path = Path::new(path);
        
        if path.is_dir() {
            if recursive {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_dir(path)?;
            }
        } else {
            fs::remove_file(path)?;
        }
        
        Ok(json!({
            "path": path.to_string_lossy(),
            "deleted": true,
        }))
    }

    async fn handle_move_file(&self, params: HashMap<String, Value>) -> Result<Value> {
        let source = params.get("source")
            .and_then(|p| p.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing source parameter"))?;
        
        let destination = params.get("destination")
            .and_then(|p| p.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing destination parameter"))?;
        
        let source_path = Path::new(source);
        let dest_path = Path::new(destination);
        
        fs::rename(source_path, dest_path)?;
        
        Ok(json!({
            "source": source,
            "destination": destination,
            "moved": true,
        }))
    }

    async fn handle_copy_file(&self, params: HashMap<String, Value>) -> Result<Value> {
        let source = params.get("source")
            .and_then(|p| p.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing source parameter"))?;
        
        let destination = params.get("destination")
            .and_then(|p| p.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing destination parameter"))?;
        
        let source_path = Path::new(source);
        let dest_path = Path::new(destination);
        
        if source_path.is_dir() {
            copy_dir(source_path, dest_path)?;
        } else {
            fs::copy(source_path, dest_path)?;
        }
        
        Ok(json!({
            "source": source,
            "destination": destination,
            "copied": true,
        }))
    }

    async fn handle_get_disk_usage(&self, params: HashMap<String, Value>) -> Result<Value> {
        let path = params.get("path")
            .and_then(|p| p.as_str())
            .unwrap_or("/");
        
        let path = Path::new(path);
        let (total_size, file_count, dir_count) = self.calculate_disk_usage(path)?;
        
        Ok(json!({
            "path": path.to_string_lossy(),
            "total_size_bytes": total_size,
            "file_count": file_count,
            "directory_count": dir_count,
            "total_size_human": format_bytes(total_size),
        }))
    }

    fn calculate_disk_usage(&self, path: &Path) -> Result<(u64, u64, u64)> {
        let mut total_size = 0u64;
        let mut file_count = 0u64;
        let mut dir_count = 0u64;
        
        if path.is_file() {
            total_size = fs::metadata(path)?.len();
            file_count = 1;
        } else if path.is_dir() {
            dir_count = 1;
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let entry_path = entry.path();
                
                if entry_path.is_file() {
                    total_size += fs::metadata(&entry_path)?.len();
                    file_count += 1;
                } else if entry_path.is_dir() {
                    let (sub_size, sub_files, sub_dirs) = self.calculate_disk_usage(&entry_path)?;
                    total_size += sub_size;
                    file_count += sub_files;
                    dir_count += sub_dirs + 1;
                }
            }
        }
        
        Ok((total_size, file_count, dir_count))
    }
}

// Helper function to copy directories recursively
fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if src_path.is_dir() {
            copy_dir(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

// Helper function to format bytes
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: f64 = 1024.0;
    
    if bytes == 0 {
        return "0 B".to_string();
    }
    
    let bytes_f = bytes as f64;
    let unit_index = (bytes_f.log10() / THRESHOLD.log10()).floor() as usize;
    let unit_index = unit_index.min(UNITS.len() - 1);
    
    let size = bytes_f / THRESHOLD.powi(unit_index as i32);
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

// Factory function for plugin loading
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut Box<dyn Plugin> {
    let plugin = Box::new(FilePlugin::new());
    Box::into_raw(Box::new(plugin))
}

// Required for dynamic loading
#[no_mangle]
pub extern "C" fn get_plugin_info() -> PluginInfo {
    PluginInfo {
        name: "file_plugin".to_string(),
        version: "1.0.0".to_string(),
        description: "File system operations and monitoring plugin".to_string(),
        author: "MSP Agent Team".to_string(),
        status: PluginStatus::Unloaded,
        loaded_at: None,
        last_error: None,
    }
}
