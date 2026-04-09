use anyhow::Result;
use chrono::Utc;
use core_shared::{Plugin, PluginInfo, PluginStatus, SystemMetrics, FileInfo, NetworkInfo, EventMessage, EventType};
use serde_json::{json, Value};
use std::collections::HashMap;
use sysinfo::{System, SystemExt, ProcessExt, CpuExt, DiskExt, NetworkExt};
use tracing::{info, warn, error};
use uuid::Uuid;

pub struct SystemPlugin {
    info: PluginInfo,
    system: System,
    last_network_stats: HashMap<String, (u64, u64)>,
}

impl SystemPlugin {
    pub fn new() -> Self {
        Self {
            info: PluginInfo {
                name: "system_plugin".to_string(),
                version: "1.0.0".to_string(),
                description: "System metrics and information plugin".to_string(),
                author: "MSP Agent Team".to_string(),
                status: PluginStatus::Unloaded,
                loaded_at: None,
                last_error: None,
            },
            system: System::new_all(),
            last_network_stats: HashMap::new(),
        }
    }
}

impl Plugin for SystemPlugin {
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
        info!("Initializing system plugin");
        
        self.system.refresh_all();
        self.info.status = PluginStatus::Loaded;
        self.info.loaded_at = Some(Utc::now());
        
        // Initialize network stats
        for (interface_name, data) in self.system.networks() {
            self.last_network_stats.insert(
                interface_name.clone(),
                (data.total_received(), data.total_transmitted())
            );
        }
        
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down system plugin");
        self.info.status = PluginStatus::Unloaded;
        self.info.loaded_at = None;
        Ok(())
    }

    async fn handle_command(&self, command: &str, params: HashMap<String, Value>) -> Result<Value> {
        match command {
            "get_system_info" => self.get_system_info(),
            "get_processes" => self.get_processes(params),
            "get_disk_info" => self.get_disk_info(),
            "get_memory_info" => self.get_memory_info(),
            "get_cpu_info" => self.get_cpu_info(),
            "get_network_info" => self.get_network_info(),
            "get_uptime" => self.get_uptime(),
            "get_load_average" => self.get_load_average(),
            _ => Err(anyhow::anyhow!("Unknown command: {}", command)),
        }
    }

    async fn get_metrics(&self) -> Result<SystemMetrics> {
        self.system.refresh_all();
        
        let cpu_usage = self.system.global_cpu_info().cpu_usage();
        let total_memory = self.system.total_memory();
        let used_memory = self.system.used_memory();
        let memory_usage = (used_memory as f64 / total_memory as f64) * 100.0;
        
        let total_disk = self.system.disks().iter().map(|d| d.total_space()).sum();
        let used_disk = self.system.disks().iter().map(|d| d.available_space()).sum();
        let disk_usage = ((total_disk - used_disk) as f64 / total_disk as f64) * 100.0;
        
        let (network_rx, network_tx) = self.get_network_stats();
        
        let uptime = self.system.uptime();
        
        let load_average = if cfg!(target_os = "linux") {
            Some(self.system.load_average())
        } else {
            None
        };
        
        Ok(SystemMetrics {
            timestamp: Utc::now(),
            cpu_usage,
            memory_usage,
            disk_usage,
            network_rx,
            network_tx,
            uptime: uptime as u64,
            load_average,
        })
    }

    fn health_check(&self) -> Result<()> {
        if self.info.status != PluginStatus::Loaded {
            return Err(anyhow::anyhow!("Plugin not loaded"));
        }
        
        // Check if system is accessible
        self.system.refresh_cpu();
        Ok(())
    }
}

impl SystemPlugin {
    fn get_system_info(&self) -> Result<Value> {
        self.system.refresh_all();
        
        let hostname = gethostname::gethostname().to_string_lossy().to_string();
        
        Ok(json!({
            "hostname": hostname,
            "os": self.system.name(),
            "kernel_version": self.system.kernel_version(),
            "os_version": self.system.os_version(),
            "host_id": self.system.host_id(),
            "cpu_count": self.system.cpus().len(),
            "total_memory": self.system.total_memory(),
            "total_disk": self.system.disks().iter().map(|d| d.total_space()).sum::<u64>(),
            "boot_time": self.system.boot_time(),
            "uptime": self.system.uptime(),
        }))
    }

    fn get_processes(&self, params: HashMap<String, Value>) -> Result<Value> {
        let limit = params.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;
        
        self.system.refresh_processes();
        
        let processes: Vec<Value> = self.system
            .processes()
            .values()
            .take(limit)
            .map(|p| {
                json!({
                    "pid": p.pid(),
                    "name": p.name(),
                    "cmd": p.cmd(),
                    "cpu_usage": p.cpu_usage(),
                    "memory_usage": p.memory(),
                    "virtual_memory": p.virtual_memory(),
                    "status": p.status(),
                    "start_time": p.start_time(),
                    "run_time": p.run_time(),
                    "parent": p.parent(),
                    "user_id": p.user_id(),
                    "group_id": p.group_id(),
                })
            })
            .collect();
        
        Ok(json!({
            "processes": processes,
            "total_count": self.system.processes().len()
        }))
    }

    fn get_disk_info(&self) -> Result<Value> {
        self.system.refresh_disks();
        
        let disks: Vec<Value> = self.system.disks().iter().map(|d| {
            json!({
                "name": d.name().to_string_lossy(),
                "mount_point": d.mount_point().to_string_lossy(),
                "file_system": d.file_system().to_string_lossy(),
                "total_space": d.total_space(),
                "available_space": d.available_space(),
                "is_removable": d.is_removable(),
            })
        }).collect();
        
        Ok(json!({ "disks": disks }))
    }

    fn get_memory_info(&self) -> Result<Value> {
        self.system.refresh_memory();
        
        Ok(json!({
            "total_memory": self.system.total_memory(),
            "available_memory": self.system.available_memory(),
            "used_memory": self.system.used_memory(),
            "total_swap": self.system.total_swap(),
            "available_swap": self.system.available_swap(),
            "used_swap": self.system.used_swap(),
        }))
    }

    fn get_cpu_info(&self) -> Result<Value> {
        self.system.refresh_cpu();
        
        let cpus: Vec<Value> = self.system.cpus().iter().enumerate().map(|(i, cpu)| {
            json!({
                "id": i,
                "name": cpu.name(),
                "vendor_id": cpu.vendor_id(),
                "brand": cpu.brand(),
                "frequency": cpu.frequency(),
                "cpu_usage": cpu.cpu_usage(),
                "core_id": cpu.core_id(),
            })
        }).collect();
        
        Ok(json!({
            "cpus": cpus,
            "global_cpu_usage": self.system.global_cpu_info().cpu_usage(),
        }))
    }

    fn get_network_info(&self) -> Result<Value> {
        self.system.refresh_networks();
        
        let interfaces: Vec<Value> = self.system.networks().iter().map(|(name, data)| {
            let (prev_rx, prev_tx) = self.last_network_stats.get(name).unwrap_or(&(0, 0));
            let current_rx = data.total_received();
            let current_tx = data.total_transmitted();
            let rx_rate = current_rx.saturating_sub(*prev_rx);
            let tx_rate = current_tx.saturating_sub(*prev_tx);
            
            json!({
                "name": name,
                "total_received": current_rx,
                "total_transmitted": current_tx,
                "received_rate": rx_rate,
                "transmitted_rate": tx_rate,
                "packets_received": data.total_received_packets(),
                "packets_transmitted": data.total_transmitted_packets(),
                "errors_on_incoming": data.errors_on_incoming(),
                "errors_on_outgoing": data.errors_on_outgoing(),
            })
        }).collect();
        
        Ok(json!({ "interfaces": interfaces }))
    }

    fn get_uptime(&self) -> Result<Value> {
        Ok(json!({
            "uptime_seconds": self.system.uptime(),
            "boot_time": self.system.boot_time(),
        }))
    }

    fn get_load_average(&self) -> Result<Value> {
        if cfg!(target_os = "linux") {
            let load = self.system.load_average();
            Ok(json!({
                "one": load.0,
                "five": load.1,
                "fifteen": load.2,
            }))
        } else {
            Ok(json!({
                "error": "Load average not available on this platform"
            }))
        }
    }

    fn get_network_stats(&self) -> (u64, u64) {
        self.system.refresh_networks();
        
        let mut total_rx = 0u64;
        let mut total_tx = 0u64;
        
        for (name, data) in self.system.networks() {
            let current_rx = data.total_received();
            let current_tx = data.total_transmitted();
            
            total_rx += current_rx;
            total_tx += current_tx;
            
            self.last_network_stats.insert(name.clone(), (current_rx, current_tx));
        }
        
        (total_rx, total_tx)
    }
}

// Factory function for plugin loading
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut Box<dyn Plugin> {
    let plugin = Box::new(SystemPlugin::new());
    Box::into_raw(Box::new(plugin))
}

// Required for dynamic loading
#[no_mangle]
pub extern "C" fn get_plugin_info() -> PluginInfo {
    PluginInfo {
        name: "system_plugin".to_string(),
        version: "1.0.0".to_string(),
        description: "System metrics and information plugin".to_string(),
        author: "MSP Agent Team".to_string(),
        status: PluginStatus::Unloaded,
        loaded_at: None,
        last_error: None,
    }
}
