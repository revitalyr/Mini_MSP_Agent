use anyhow::Result;
use mini_msp_shared::Metrics;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error};

use crate::plugins::PluginManager;

pub struct TelemetryCollector {
    plugin_manager: PluginManager,
    start_time: SystemTime,
}

impl TelemetryCollector {
    pub fn new(plugin_manager: PluginManager) -> Self {
        Self {
            plugin_manager,
            start_time: SystemTime::now(),
        }
    }

    pub async fn collect_metrics(&self) -> Result<Metrics> {
        if !self.plugin_manager.is_system_plugin_loaded() {
            return Err(anyhow::anyhow!("No system plugin loaded"));
        }

        let system_metrics = self.plugin_manager.get_system_metrics()
            .map_err(|e| {
                error!("Failed to collect metrics from plugin: {}", e);
                e
            })?;

        debug!("Collected metrics - CPU: {:.1}%, RAM: {:.1}%, Disk: {:.1}%", 
               system_metrics.cpu_usage, system_metrics.ram_usage, system_metrics.disk_usage);

        Ok(Metrics {
            cpu: system_metrics.cpu_usage,
            ram: system_metrics.ram_usage,
            disk: system_metrics.disk_usage,
        })
    }

    pub fn get_hostname(&self) -> String {
        if let Ok(system_metrics) = self.plugin_manager.get_system_metrics() {
            system_metrics.hostname
        } else {
            // Fallback to environment
            std::env::var("COMPUTERNAME")
                .or_else(|_| std::env::var("HOSTNAME"))
                .unwrap_or_else(|_| {
                    std::process::Command::new("hostname")
                        .output()
                        .ok()
                        .and_then(|o| String::from_utf8(o.stdout).ok())
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                })
        }
    }

    pub fn get_uptime(&self) -> u64 {
        if let Ok(system_metrics) = self.plugin_manager.get_system_metrics() {
            system_metrics.uptime
        } else {
            self.start_time
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        }
    }

    pub fn get_processes(&self) -> Result<Vec<ProcessInfo>> {
        if !self.plugin_manager.is_system_plugin_loaded() {
            return Err(anyhow::anyhow!("No system plugin loaded"));
        }

        let plugin_processes = self.plugin_manager.get_processes()
            .map_err(|e| {
                error!("Failed to get processes from plugin: {}", e);
                e
            })?;

        let mut processes = Vec::new();
        for proc in plugin_processes {
            processes.push(ProcessInfo {
                pid: proc.pid,
                name: proc.name,
                cpu_usage: proc.cpu_usage,
                memory_usage: proc.memory_usage,
                start_time: proc.start_time,
            });
        }

        processes.sort_by(|a, b| a.cpu_usage.partial_cmp(&b.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));
        processes.reverse();

        Ok(processes)
    }

    pub fn get_system_info(&self) -> Result<SystemInfo> {
        if !self.plugin_manager.is_system_plugin_loaded() {
            return Err(anyhow::anyhow!("No system plugin loaded"));
        }

        let plugin_info = self.plugin_manager.get_system_info()
            .map_err(|e| {
                error!("Failed to get system info from plugin: {}", e);
                e
            })?;

        Ok(SystemInfo {
            os_type: plugin_info.os_type,
            os_version: plugin_info.os_version,
            hostname: plugin_info.hostname,
            uptime: plugin_info.uptime,
            cpu_cores: plugin_info.cpu_cores,
            total_memory: plugin_info.total_memory,
            available_memory: plugin_info.available_memory,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub start_time: u64,
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub os_type: String,
    pub os_version: String,
    pub hostname: String,
    pub uptime: u64,
    pub cpu_cores: u32,
    pub total_memory: u64,
    pub available_memory: u64,
}
