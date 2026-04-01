use std::process::Command;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let output_format = if args.len() > 1 {
        match args[1].as_str() {
            "json" => "json",
            "table" => "table",
            _ => "table"
        }
    } else {
        "table"
    };

    if output_format == "json" {
        generate_json_report()
    } else {
        generate_table_report()
    }
}

fn generate_json_report() -> Result<(), Box<dyn std::error::Error>> {
    let hostname = Command::new("hostname").output()?.stdout;
    let hostname_str = String::from_utf8_lossy(&hostname);
    let hostname = hostname_str.trim();

    let current_user = std::env::var("USER").unwrap_or_else(|_| "Unknown".to_string());
    
    let report = json!({
        "system_info": {
            "hostname": hostname,
            "os_type": "Linux",
            "architecture": std::env::consts::ARCH,
            "current_user": current_user,
            "os_version": get_os_version()?,
            "uptime": get_uptime()?
        },
        "hardware_info": {
            "cpu": {
                "model": get_cpu_model()?,
                "cores": get_cpu_cores()?,
                "architecture": std::env::consts::ARCH
            },
            "memory": {
                "total_mb": get_memory_info()?
            }
        },
        "network_info": {
            "interfaces": get_network_interfaces()?
        },
        "disk_info": get_disk_info()?,
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
    });

    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn generate_table_report() -> Result<(), Box<dyn std::error::Error>> {
    println!("🖥️  Mini MSP Agent - OS Information Reporter");
    println!("============================================");
    println!();

    // System Information
    println!("📋 SYSTEM INFORMATION:");
    println!("---------------------");
    
    let hostname = Command::new("hostname").output()?.stdout;
    let hostname_str = String::from_utf8_lossy(&hostname);
    let hostname = hostname_str.trim();
    println!("Hostname: {}", hostname);

    println!("OS Type: Linux");
    println!("Architecture: {}", std::env::consts::ARCH);

    let current_user = std::env::var("USER").unwrap_or_else(|_| "Unknown".to_string());
    println!("Current User: {}", current_user);

    // OS Version
    println!();
    println!("📋 OS VERSION:");
    println!("-------------");
    println!("{}", get_os_version()?);

    // Uptime
    println!();
    println!("⏰ UPTIME:");
    println!("---------");
    println!("{}", get_uptime()?);

    // CPU Information
    println!();
    println!("🔧 CPU INFORMATION:");
    println!("------------------");
    println!("Model: {}", get_cpu_model()?);
    println!("Cores: {}", get_cpu_cores()?);

    // Memory Information
    println!();
    println!("💾 MEMORY INFORMATION:");
    println!("---------------------");
    println!("Total: {} MB", get_memory_info()?);

    // Network Information
    println!();
    println!("🌐 NETWORK INFORMATION:");
    println!("----------------------");
    let interfaces = get_network_interfaces()?;
    for interface in interfaces {
        if let Some(ip) = interface.get("ip_address") {
            println!("{}: {}", interface.get("name").unwrap_or(&json!("unknown")), ip);
        }
    }

    println!();
    println!("🎉 OS Information Report Complete!");
    println!("==================================");
    
    Ok(())
}

fn get_os_version() -> Result<String, Box<dyn std::error::Error>> {
    let os_info = Command::new("cat").args(&["/etc/os-release"]).output()?;
    let info = String::from_utf8_lossy(&os_info.stdout);
    for line in info.lines() {
        if line.starts_with("PRETTY_NAME=") {
            return Ok(line.split('=').nth(1).unwrap_or("Unknown").trim_matches('"').to_string());
        }
    }
    Ok("Unknown".to_string())
}

fn get_uptime() -> Result<String, Box<dyn std::error::Error>> {
    let uptime = Command::new("uptime").output()?;
    let uptime_str = String::from_utf8_lossy(&uptime.stdout);
    Ok(uptime_str.trim().to_string())
}

fn get_cpu_model() -> Result<String, Box<dyn std::error::Error>> {
    let cpu_info = Command::new("lscpu").output()?;
    let info = String::from_utf8_lossy(&cpu_info.stdout);
    for line in info.lines() {
        if line.contains("Model name:") {
            return Ok(line.split(':').nth(1).unwrap_or("Unknown").trim().to_string());
        }
    }
    Ok("Unknown".to_string())
}

fn get_cpu_cores() -> Result<String, Box<dyn std::error::Error>> {
    let cpu_info = Command::new("lscpu").output()?;
    let info = String::from_utf8_lossy(&cpu_info.stdout);
    for line in info.lines() {
        if line.contains("CPU(s):") {
            return Ok(line.split(':').nth(1).unwrap_or("Unknown").trim().to_string());
        }
    }
    Ok("Unknown".to_string())
}

fn get_memory_info() -> Result<String, Box<dyn std::error::Error>> {
    let mem_info = Command::new("free").args(&["-m"]).output()?;
    let info = String::from_utf8_lossy(&mem_info.stdout);
    for line in info.lines() {
        if line.starts_with("Mem:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1 {
                return Ok(parts[1].to_string());
            }
        }
    }
    Ok("Unknown".to_string())
}

fn get_network_interfaces() -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let mut interfaces = Vec::new();
    
    let net_info = Command::new("ip").args(&["addr", "show"]).output()?;
    let info = String::from_utf8_lossy(&net_info.stdout);
    
    let mut current_interface = serde_json::Map::new();
    let mut interface_name = String::new();
    
    for line in info.lines() {
        if line.trim().starts_with("inet ") && !line.contains("127.0.0.1") {
            let ip_parts: Vec<&str> = line.trim().split_whitespace().collect();
            if ip_parts.len() > 1 {
                current_interface.insert("ip_address".to_string(), json!(ip_parts[1]));
                current_interface.insert("name".to_string(), json!(interface_name));
                interfaces.push(json!(current_interface.clone()));
                current_interface.clear();
            }
        } else if line.chars().next().unwrap_or(' ') != ' ' && line.contains(':') {
            interface_name = line.split(':').next().unwrap_or("unknown").to_string();
        }
    }
    
    Ok(interfaces)
}

fn get_disk_info() -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let mut disks = Vec::new();
    
    let disk_info = Command::new("df").args(&["-h"]).output()?;
    let info = String::from_utf8_lossy(&disk_info.stdout);
    
    for line in info.lines() {
        if line.starts_with("/dev/") && !line.contains("tmpfs") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 6 {
                let disk = json!({
                    "filesystem": parts[0],
                    "size": parts[1],
                    "used": parts[2],
                    "available": parts[3],
                    "use_percent": parts[4],
                    "mount_point": parts[5]
                });
                disks.push(disk);
            }
        }
    }
    
    Ok(disks)
}
