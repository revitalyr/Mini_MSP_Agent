use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let os_type = if cfg!(target_os = "windows") {
        "Windows"
    } else if cfg!(target_os = "linux") {
        "Linux"
    } else if cfg!(target_os = "macos") {
        "macOS"
    } else {
        "Unknown"
    };
    println!("OS Type: {}", os_type);

    let architecture = std::env::consts::ARCH;
    println!("Architecture: {}", architecture);

    let current_user = std::env::var("USER").unwrap_or_else(|_| "Unknown".to_string());
    println!("Current User: {}", current_user);

    // OS Version
    println!();
    println!("📋 OS VERSION:");
    println!("-------------");
    
    if cfg!(target_os = "linux") {
        let os_info = Command::new("lsb_release").args(&["-d"]).output()?;
        if os_info.status.success() {
            let info = String::from_utf8_lossy(&os_info.stdout);
            println!("{}", info.trim().split(':').nth(1).unwrap_or("Unknown").trim());
        } else {
            let os_info = Command::new("cat").args(&["/etc/os-release"]).output()?;
            let info = String::from_utf8_lossy(&os_info.stdout);
            for line in info.lines() {
                if line.starts_with("PRETTY_NAME=") {
                    println!("{}", line.split('=').nth(1).unwrap_or("Unknown").trim_matches('"'));
                    break;
                }
            }
        }
    }

    // Uptime
    println!();
    println!("⏰ UPTIME:");
    println!("---------");
    
    if cfg!(target_os = "linux") {
        let uptime = Command::new("uptime").output()?;
        let uptime_str = String::from_utf8_lossy(&uptime.stdout);
        println!("{}", uptime_str.trim());
    }

    // CPU Information
    println!();
    println!("🔧 CPU INFORMATION:");
    println!("------------------");
    
    if cfg!(target_os = "linux") {
        let cpu_info = Command::new("lscpu").output()?;
        let info = String::from_utf8_lossy(&cpu_info.stdout);
        for line in info.lines() {
            if line.contains("Architecture:") || line.contains("CPU(s):") || line.contains("Model name:") {
                println!("{}", line);
            }
        }
    }

    // Memory Information
    println!();
    println!("💾 MEMORY INFORMATION:");
    println!("---------------------");
    
    if cfg!(target_os = "linux") {
        let mem_info = Command::new("free").args(&["-h"]).output()?;
        let info = String::from_utf8_lossy(&mem_info.stdout);
        for line in info.lines() {
            if line.starts_with("Mem:") {
                println!("{}", line);
                break;
            }
        }
    }

    // Disk Information
    println!();
    println!("💿 DISK INFORMATION:");
    println!("-------------------");
    
    if cfg!(target_os = "linux") {
        let disk_info = Command::new("df").args(&["-h"]).output()?;
        let info = String::from_utf8_lossy(&disk_info.stdout);
        for line in info.lines() {
            if line.starts_with("/dev/") && !line.contains("tmpfs") {
                println!("{}", line);
            }
        }
    }

    // Network Information
    println!();
    println!("🌐 NETWORK INFORMATION:");
    println!("----------------------");
    
    if cfg!(target_os = "linux") {
        let net_info = Command::new("ip").args(&["addr", "show"]).output()?;
        let info = String::from_utf8_lossy(&net_info.stdout);
        for line in info.lines() {
            if line.trim().starts_with("inet ") && !line.contains("127.0.0.1") {
                println!("{}", line.trim());
            }
        }
    }

    // Top Processes
    println!();
    println!("⚙️  TOP PROCESSES:");
    println!("-----------------");
    
    if cfg!(target_os = "linux") {
        let processes = Command::new("ps").args(&["-eo", "pid,comm,%mem,rss", "--sort=-%mem", "--head=5"]).output()?;
        let info = String::from_utf8_lossy(&processes.stdout);
        println!("{}", info.trim());
    }

    println!();
    println!("🎉 OS Information Report Complete!");
    println!("==================================");
    
    Ok(())
}
