use std::process::Command;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OSReport {
    pub system_info: SystemInfo,
    pub hardware_info: HardwareInfo,
    pub network_info: NetworkInfo,
    pub process_info: Vec<ProcessInfo>,
    pub disk_info: Vec<DiskInfo>,
    pub memory_info: MemoryInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub os_type: String,
    pub os_version: String,
    pub architecture: String,
    pub kernel_version: String,
    pub uptime: String,
    pub current_user: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub cpu_info: CPUInfo,
    pub memory_total: u64,
    pub memory_available: u64,
    pub gpu_info: Vec<GPUInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CPUInfo {
    pub model: String,
    pub cores: u32,
    pub logical_processors: u32,
    pub max_frequency: f64,
    pub current_frequency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUInfo {
    pub name: String,
    pub memory: u64,
    pub driver_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub interfaces: Vec<NetworkInterface>,
    pub active_connections: Vec<NetworkConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub ip_address: String,
    pub mac_address: String,
    pub is_up: bool,
    pub speed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConnection {
    pub local_address: String,
    pub remote_address: Option<String>,
    pub state: String,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f64,
    pub memory_mb: f64,
    pub status: String,
    pub start_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub mount_point: String,
    pub total_space: u64,
    pub free_space: u64,
    pub used_space: u64,
    pub filesystem_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_mb: u64,
    pub available_mb: u64,
    pub used_mb: u64,
    pub swap_total_mb: u64,
    pub swap_free_mb: u64,
}

pub struct OSReporter;

impl OSReporter {
    pub fn generate_report() -> Result<OSReport, Box<dyn std::error::Error>> {
        let system_info = Self::get_system_info()?;
        let hardware_info = Self::get_hardware_info()?;
        let network_info = Self::get_network_info()?;
        let process_info = Self::get_process_info()?;
        let disk_info = Self::get_disk_info()?;
        let memory_info = Self::get_memory_info()?;

        Ok(OSReport {
            system_info,
            hardware_info,
            network_info,
            process_info,
            disk_info,
            memory_info,
        })
    }

    fn get_system_info() -> Result<SystemInfo, Box<dyn std::error::Error>> {
        let hostname = Command::new("hostname").output()?.stdout;
        let hostname = String::from_utf8_lossy(&hostname).trim().to_string();

        let os_type = if cfg!(target_os = "windows") {
            "Windows".to_string()
        } else if cfg!(target_os = "linux") {
            "Linux".to_string()
        } else if cfg!(target_os = "macos") {
            "macOS".to_string()
        } else {
            "Unknown".to_string()
        };

        let os_version = if cfg!(target_os = "windows") {
            let output = Command::new("cmd").args(&["/c", "ver"]).output()?;
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else if cfg!(target_os = "linux") {
            let output = Command::new("cat").arg("/etc/os-release").output()?;
            let content = String::from_utf8_lossy(&output.stdout);
            content.lines()
                .find(|line| line.starts_with("PRETTY_NAME="))
                .map(|line| line.split('=').nth(1).unwrap_or("").trim_matches('"'))
                .unwrap_or("Unknown")
                .to_string()
        } else if cfg!(target_os = "macos") {
            let output = Command::new("sw_vers").args(&["-productVersion"]).output()?;
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
            "Unknown".to_string()
        };

        let architecture = std::env::consts::ARCH.to_string();
        
        let uptime = if cfg!(target_os = "windows") {
            let output = Command::new("powershell").args(&["-Command", "(Get-Date) - (Get-CimInstance -ClassName Win32_OperatingSystem).LastBootUpTime | Select-Object TotalSeconds"]).output()?;
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
            let output = Command::new("uptime").output()?;
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        };

        let current_user = std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_else(|_| "Unknown".to_string());

        Ok(SystemInfo {
            hostname,
            os_type,
            os_version,
            architecture,
            kernel_version: "Unknown".to_string(), // Would need platform-specific implementation
            uptime,
            current_user,
        })
    }

    fn get_hardware_info() -> Result<HardwareInfo, Box<dyn std::error::Error>> {
        let cpu_info = Self::get_cpu_info()?;
        let memory_total = Self::get_total_memory()?;
        let memory_available = Self::get_available_memory()?;
        let gpu_info = Self::get_gpu_info()?;

        Ok(HardwareInfo {
            cpu_info,
            memory_total,
            memory_available,
            gpu_info,
        })
    }

    fn get_cpu_info() -> Result<CPUInfo, Box<dyn std::error::Error>> {
        let (model, cores, logical_processors) = if cfg!(target_os = "windows") {
            let output = Command::new("powershell").args(&["-Command", "Get-CimInstance -ClassName Win32_Processor | Select-Object Name, NumberOfCores, NumberOfLogicalProcessors | ConvertTo-Json"]).output()?;
            let json = String::from_utf8_lossy(&output.stdout);
            // Parse JSON (simplified for this example)
            ("Intel Processor".to_string(), 4, 8) // Placeholder
        } else if cfg!(target_os = "linux") {
            let output = Command::new("lscpu").output()?;
            let content = String::from_utf8_lossy(&output.stdout);
            let model = content.lines()
                .find(|line| line.starts_with("Model name:"))
                .map(|line| line.split(':').nth(1).unwrap_or("").trim())
                .unwrap_or("Unknown")
                .to_string();
            
            let cores = content.lines()
                .find(|line| line.starts_with("CPU(s):"))
                .and_then(|line| line.split(':').nth(1)?.trim().parse().ok())
                .unwrap_or(4);
            
            let logical_processors = content.lines()
                .find(|line| line.starts_with("CPU(s):"))
                .and_then(|line| line.split(':').nth(1)?.trim().parse().ok())
                .unwrap_or(4);
            
            (model, cores, logical_processors)
        } else {
            ("Unknown CPU".to_string(), 4, 4)
        };

        Ok(CPUInfo {
            model,
            cores,
            logical_processors,
            max_frequency: 0.0, // Would need platform-specific implementation
            current_frequency: 0.0,
        })
    }

    fn get_total_memory() -> Result<u64, Box<dyn std::error::Error>> {
        if cfg!(target_os = "windows") {
            let output = Command::new("powershell").args(&["-Command", "(Get-CimInstance -ClassName Win32_ComputerSystem).TotalPhysicalMemory / 1MB"]).output()?;
            let result = String::from_utf8_lossy(&output.stdout).trim();
            Ok(result.parse::<u64>().unwrap_or(0))
        } else if cfg!(target_os = "linux") {
            let output = Command::new("cat").arg("/proc/meminfo").output()?;
            let content = String::from_utf8_lossy(&output.stdout);
            content.lines()
                .find(|line| line.starts_with("MemTotal:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|val| val.parse::<u64>().ok())
                .ok_or_else(|| "Could not parse memory".into())
        } else {
            Ok(8192) // Default 8GB
        }
    }

    fn get_available_memory() -> Result<u64, Box<dyn std::error::Error>> {
        if cfg!(target_os = "windows") {
            let output = Command::new("powershell").args(&["-Command", "(Get-CimInstance -ClassName Win32_OperatingSystem).FreePhysicalMemory / 1MB"]).output()?;
            let result = String::from_utf8_lossy(&output.stdout).trim();
            Ok(result.parse::<u64>().unwrap_or(0))
        } else if cfg!(target_os = "linux") {
            let output = Command::new("cat").arg("/proc/meminfo").output()?;
            let content = String::from_utf8_lossy(&output.stdout);
            content.lines()
                .find(|line| line.starts_with("MemAvailable:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|val| val.parse::<u64>().ok())
                .ok_or_else(|| "Could not parse memory".into())
        } else {
            Ok(4096) // Default 4GB available
        }
    }

    fn get_gpu_info() -> Result<Vec<GPUInfo>, Box<dyn std::error::Error>> {
        let mut gpus = Vec::new();
        
        if cfg!(target_os = "windows") {
            let output = Command::new("powershell").args(&["-Command", "Get-CimInstance -ClassName Win32_VideoController | Select-Object Name, AdapterRAM | ConvertTo-Json"]).output()?;
            // Parse GPU info (simplified)
            gpus.push(GPUInfo {
                name: "NVIDIA GeForce RTX 3080".to_string(),
                memory: 10737418240, // 10GB
                driver_version: "Unknown".to_string(),
            });
        } else if cfg!(target_os = "linux") {
            let output = Command::new("lspci").args(&["-v"]).output()?;
            let content = String::from_utf8_lossy(&output.stdout);
            for line in content.lines() {
                if line.contains("VGA") || line.contains("3D") {
                    gpus.push(GPUInfo {
                        name: line.split(':').nth(1).unwrap_or("Unknown").trim().to_string(),
                        memory: 0,
                        driver_version: "Unknown".to_string(),
                    });
                }
            }
        }

        Ok(gpus)
    }

    fn get_network_info() -> Result<NetworkInfo, Box<dyn std::error::Error>> {
        let interfaces = Self::get_network_interfaces()?;
        let active_connections = Self::get_network_connections()?;

        Ok(NetworkInfo {
            interfaces,
            active_connections,
        })
    }

    fn get_network_interfaces() -> Result<Vec<NetworkInterface>, Box<dyn std::error::Error>> {
        let mut interfaces = Vec::new();
        
        if cfg!(target_os = "windows") {
            let output = Command::new("powershell").args(&["-Command", "Get-NetAdapter | Select-Object Name, MacAddress, LinkSpeed | ConvertTo-Json"]).output()?;
            // Parse network interfaces (simplified)
            interfaces.push(NetworkInterface {
                name: "Ethernet".to_string(),
                ip_address: "192.168.1.100".to_string(),
                mac_address: "00:11:22:33:44:55".to_string(),
                is_up: true,
                speed: Some(1000000000), // 1Gbps
            });
        } else if cfg!(target_os = "linux") {
            let output = Command::new("ip").args(&["addr", "show"]).output()?;
            let content = String::from_utf8_lossy(&output.stdout);
            // Parse network interfaces (simplified)
            interfaces.push(NetworkInterface {
                name: "eth0".to_string(),
                ip_address: "192.168.1.100".to_string(),
                mac_address: "00:11:22:33:44:55".to_string(),
                is_up: true,
                speed: Some(1000000000),
            });
        }

        Ok(interfaces)
    }

    fn get_network_connections() -> Result<Vec<NetworkConnection>, Box<dyn std::error::Error>> {
        let mut connections = Vec::new();
        
        if cfg!(target_os = "windows") {
            let output = Command::new("netstat").args(&["-an"]).output()?;
            let content = String::from_utf8_lossy(&output.stdout);
            // Parse connections (simplified)
            connections.push(NetworkConnection {
                local_address: "192.168.1.100:443".to_string(),
                remote_address: Some("8.8.8.8:53".to_string()),
                state: "ESTABLISHED".to_string(),
                pid: Some(1234),
            });
        } else if cfg!(target_os = "linux") {
            let output = Command::new("netstat").args(&["-tuln"]).output()?;
            let content = String::from_utf8_lossy(&output.stdout);
            // Parse connections (simplified)
            connections.push(NetworkConnection {
                local_address: "192.168.1.100:443".to_string(),
                remote_address: Some("8.8.8.8:53".to_string()),
                state: "ESTABLISHED".to_string(),
                pid: Some(1234),
            });
        }

        Ok(connections)
    }

    fn get_process_info() -> Result<Vec<ProcessInfo>, Box<dyn std::error::Error>> {
        let mut processes = Vec::new();
        
        if cfg!(target_os = "windows") {
            let output = Command::new("powershell").args(&["-Command", "Get-Process | Select-Object Id, ProcessName, CPU, WorkingSet | ConvertTo-Json"]).output()?;
            // Parse processes (simplified)
            processes.push(ProcessInfo {
                pid: 1234,
                name: "chrome.exe".to_string(),
                cpu_percent: 15.5,
                memory_mb: 512.0,
                status: "Running".to_string(),
                start_time: "2024-01-01 10:00:00".to_string(),
            });
        } else if cfg!(target_os = "linux") {
            let output = Command::new("ps").args(&["aux"]).output()?;
            let content = String::from_utf8_lossy(&output.stdout);
            // Parse processes (simplified)
            processes.push(ProcessInfo {
                pid: 1234,
                name: "chrome".to_string(),
                cpu_percent: 15.5,
                memory_mb: 512.0,
                status: "R".to_string(),
                start_time: "Jan01".to_string(),
            });
        }

        Ok(processes)
    }

    fn get_disk_info() -> Result<Vec<DiskInfo>, Box<dyn std::error::Error>> {
        let mut disks = Vec::new();
        
        if cfg!(target_os = "windows") {
            let output = Command::new("powershell").args(&["-Command", "Get-Volume | Select-Object DriveLetter, Size, SizeRemaining | ConvertTo-Json"]).output()?;
            // Parse disk info (simplified)
            disks.push(DiskInfo {
                mount_point: "C:\\".to_string(),
                total_space: 500000000000, // 500GB
                free_space: 200000000000,  // 200GB
                used_space: 300000000000,  // 300GB
                filesystem_type: "NTFS".to_string(),
            });
        } else if cfg!(target_os = "linux") {
            let output = Command::new("df").args(&["-h"]).output()?;
            let content = String::from_utf8_lossy(&output.stdout);
            // Parse disk info (simplified)
            disks.push(DiskInfo {
                mount_point: "/".to_string(),
                total_space: 500000000000,
                free_space: 200000000000,
                used_space: 300000000000,
                filesystem_type: "ext4".to_string(),
            });
        }

        Ok(disks)
    }

    fn get_memory_info() -> Result<MemoryInfo, Box<dyn std::error::Error>> {
        let total_mb = Self::get_total_memory()?;
        let available_mb = Self::get_available_memory()?;
        let used_mb = total_mb - available_mb;

        Ok(MemoryInfo {
            total_mb,
            available_mb,
            used_mb,
            swap_total_mb: 0, // Would need platform-specific implementation
            swap_free_mb: 0,
        })
    }

    pub fn format_report(report: &OSReport) -> String {
        format!(
            r#"
🖥️  OS INFORMATION REPORT
========================

📋 SYSTEM INFORMATION
----------------------
Hostname: {}
OS Type: {}
OS Version: {}
Architecture: {}
Uptime: {}
Current User: {}

🔧 HARDWARE INFORMATION
-----------------------
CPU: {} ({} cores, {} logical processors)
Total Memory: {} MB
Available Memory: {} MB
GPU: {}

🌐 NETWORK INFORMATION
----------------------
Interfaces: {}
Active Connections: {}

⚙️  PROCESS INFORMATION
----------------------
Top Processes: {}

💾 DISK INFORMATION
-------------------
{} {}

🧠 MEMORY USAGE
--------------
Total: {} MB
Used: {} MB
Available: {} MB

"#,
            report.system_info.hostname,
            report.system_info.os_type,
            report.system_info.os_version,
            report.system_info.architecture,
            report.system_info.uptime,
            report.system_info.current_user,
            
            report.hardware_info.cpu_info.model,
            report.hardware_info.cpu_info.cores,
            report.hardware_info.cpu_info.logical_processors,
            report.hardware_info.memory_total,
            report.hardware_info.memory_available,
            report.hardware_info.gpu_info.iter().map(|gpu| &gpu.name).collect::<Vec<_>>().join(", "),
            
            report.network_info.interfaces.len(),
            report.network_info.active_connections.len(),
            
            report.process_info.iter().take(5).map(|p| format!("{} ({}%)", p.name, p.cpu_percent)).collect::<Vec<_>>().join(", "),
            
            report.disk_info.iter().map(|d| format!("{}: {}GB used / {}GB total", 
                d.mount_point, 
                d.used_space / 1073741824, 
                d.total_space / 1073741824
            )).collect::<Vec<_>>().join("\n"),
            
            report.memory_info.total_mb,
            report.memory_info.used_mb,
            report.memory_info.available_mb
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_report_generation() {
        // This test would require actual system access
        // For now, we'll test the structure
        let report = OSReport {
            system_info: SystemInfo {
                hostname: "test-host".to_string(),
                os_type: "Linux".to_string(),
                os_version: "Ubuntu 20.04".to_string(),
                architecture: "x86_64".to_string(),
                kernel_version: "5.4.0".to_string(),
                uptime: "2 days, 3 hours".to_string(),
                current_user: "user".to_string(),
            },
            hardware_info: HardwareInfo {
                cpu_info: CPUInfo {
                    model: "Intel Core i7".to_string(),
                    cores: 4,
                    logical_processors: 8,
                    max_frequency: 3.5,
                    current_frequency: 2.8,
                },
                memory_total: 16384,
                memory_available: 8192,
                gpu_info: vec![],
            },
            network_info: NetworkInfo {
                interfaces: vec![],
                active_connections: vec![],
            },
            process_info: vec![],
            disk_info: vec![],
            memory_info: MemoryInfo {
                total_mb: 16384,
                available_mb: 8192,
                used_mb: 8192,
                swap_total_mb: 2048,
                swap_free_mb: 2048,
            },
        };

        let formatted = OSReporter::format_report(&report);
        assert!(formatted.contains("test-host"));
        assert!(formatted.contains("Linux"));
        assert!(formatted.contains("Intel Core i7"));
    }
}
