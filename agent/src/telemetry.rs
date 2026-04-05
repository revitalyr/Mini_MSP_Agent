use anyhow::Result;
use mini_msp_shared::Metrics;
use tracing::{debug, error};

use crate::plugins::PluginManager;

#[derive(Clone)]
pub struct TelemetryCollector {
    plugin_manager: PluginManager,
}

impl TelemetryCollector {
    pub fn new(plugin_manager: PluginManager) -> Self {
        Self {
            plugin_manager,
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
        self.plugin_manager.get_system_metrics()
            .map(|m| m.hostname)
            .unwrap_or_else(|_| "unknown".to_string())
    }

    pub fn get_uptime(&self) -> u64 {
        self.plugin_manager.get_system_metrics()
            .map(|m| m.uptime)
            .unwrap_or(0)
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

        debug!("Got {} processes from plugin", plugin_processes.len());

        let mut processes = Vec::new();
        for proc in plugin_processes {
            // Log top processes for debugging
            if processes.len() <= 5 {
                let proc_info: &ProcessInfo = &processes[processes.len() - 1];
                debug!("Process: {} (PID: {}) - CPU: {:.1}%, Memory: {:.1} MB, Duration: {}", 
                       proc_info.name, proc_info.pid, proc_info.cpu_usage, proc_info.get_memory_mb(), proc_info.get_duration());
            }
            
            processes.push(ProcessInfo {
                pid: proc.pid,
                name: proc.name.clone(),
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

        let info = self.plugin_manager.get_system_info()?;
        
        debug!("System info: {} {} on {} ({} cores, {:.1} GB RAM, {:.1}% used, uptime {:.1}h)", 
               info.os_type, info.os_version, info.hostname, 
               info.cpu_cores, info.total_memory as f64 / 1024.0 / 1024.0 / 1024.0, 
               ((info.total_memory - info.available_memory) as f64 / info.total_memory as f64) * 100.0,
               info.uptime as f64 / 3600.0);

        let system_info = SystemInfo {
            os_type: info.os_type,
            os_version: info.os_version,
            hostname: info.hostname,
            uptime: info.uptime,
            cpu_cores: info.cpu_cores,
            total_memory: info.total_memory,
            available_memory: info.available_memory,
        };
        
        // Use system info fields
        debug!("Memory usage: {:.1}/{:.1} GB ({:.1}%)", 
               system_info.get_available_memory_gb(), 
               system_info.get_total_memory_gb(),
               system_info.get_memory_usage_percent());

        Ok(system_info)
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

impl ProcessInfo {
    pub fn get_memory_mb(&self) -> f64 {
        self.memory_usage as f64 / 1024.0 / 1024.0
    }
    
    pub fn get_duration(&self) -> String {
        format!("{}s", self.start_time)
    }
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

impl SystemInfo {
    pub fn get_total_memory_gb(&self) -> f64 {
        self.total_memory as f64 / 1024.0 / 1024.0 / 1024.0
    }
    
    pub fn get_available_memory_gb(&self) -> f64 {
        self.available_memory as f64 / 1024.0 / 1024.0 / 1024.0
    }
    
    pub fn get_memory_usage_percent(&self) -> f64 {
        if self.total_memory > 0 {
            ((self.total_memory - self.available_memory) as f64 / self.total_memory as f64) * 100.0
        } else {
            0.0
        }
    }
    
    pub fn get_uptime_hours(&self) -> f64 {
        self.uptime as f64 / 3600.0
    }
}
