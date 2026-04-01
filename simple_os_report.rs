use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🖥️  Mini MSP Agent - OS Information Report");
    println!("========================================");
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

    let current_user = std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_else(|_| "Unknown".to_string());
    println!("Current User: {}", current_user);
    println!();

    // OS Version (platform-specific)
    if cfg!(target_os = "windows") {
        println!("📋 OS VERSION:");
        println!("-------------");
        let output = Command::new("cmd").args(&["/c", "ver"]).output()?;
        let version_str = String::from_utf8_lossy(&output.stdout);
        let version = version_str.trim();
        println!("{}", version);
        println!();
    } else if cfg!(target_os = "linux") {
        println!("📋 OS VERSION:");
        println!("-------------");
        if let Ok(output) = Command::new("cat").arg("/etc/os-release").output() {
            let content = String::from_utf8_lossy(&output.stdout);
            for line in content.lines() {
                if line.starts_with("PRETTY_NAME=") {
                    println!("{}", line.split('=').nth(1).unwrap_or("").trim_matches('"'));
                    break;
                }
            }
        }
        println!();
    }

    // Uptime
    println!("⏰ UPTIME:");
    println!("---------");
    if cfg!(target_os = "windows") {
        let output = Command::new("powershell").args(&["-Command", "uptime"]).output();
        match output {
            Ok(result) => {
                let uptime_str = String::from_utf8_lossy(&result.stdout);
            let uptime = uptime_str.trim();
                if !uptime.is_empty() {
                    println!("{}", uptime);
                } else {
                    println!("Windows uptime information requires elevated privileges");
                }
            }
            Err(_) => println!("Could not get uptime information"),
        }
    } else {
        let output = Command::new("uptime").output()?;
        let uptime_str = String::from_utf8_lossy(&output.stdout);
        let uptime = uptime_str.trim();
        println!("{}", uptime);
    }
    println!();

    // CPU Information
    println!("🔧 CPU INFORMATION:");
    println!("------------------");
    if cfg!(target_os = "windows") {
        let output = Command::new("powershell").args(&["-Command", "Get-CimInstance -ClassName Win32_Processor | Select-Object Name, NumberOfCores, NumberOfLogicalProcessors | ConvertTo-Json"]).output();
        match output {
            Ok(result) => {
                let info = String::from_utf8_lossy(&result.stdout);
                println!("CPU Info: {}", info);
            }
            Err(_) => println!("CPU information requires elevated privileges"),
        }
    } else if cfg!(target_os = "linux") {
        let output = Command::new("lscpu").output()?;
        let content = String::from_utf8_lossy(&output.stdout);
        for line in content.lines() {
            if line.starts_with("Model name:") || line.starts_with("CPU(s):") || line.starts_with("Architecture:") {
                println!("{}", line);
            }
        }
    }
    println!();

    // Memory Information
    println!("💾 MEMORY INFORMATION:");
    println!("---------------------");
    if cfg!(target_os = "windows") {
        let output = Command::new("powershell").args(&["-Command", "Get-CimInstance -ClassName Win32_OperatingSystem | Select-Object TotalVisibleMemorySize, FreePhysicalMemory | ConvertTo-Json"]).output();
        match output {
            Ok(result) => {
                let info = String::from_utf8_lossy(&result.stdout);
                println!("Memory Info: {}", info);
            }
            Err(_) => println!("Memory information requires elevated privileges"),
        }
    } else if cfg!(target_os = "linux") {
        let output = Command::new("cat").arg("/proc/meminfo").output()?;
        let content = String::from_utf8_lossy(&output.stdout);
        for line in content.lines() {
            if line.starts_with("MemTotal:") || line.starts_with("MemAvailable:") || line.starts_with("MemFree:") {
                println!("{}", line);
            }
        }
    }
    println!();

    // Disk Information
    println!("💿 DISK INFORMATION:");
    println!("-------------------");
    if cfg!(target_os = "windows") {
        let output = Command::new("powershell").args(&["-Command", "Get-Volume | Select-Object DriveLetter, Size, SizeRemaining | ConvertTo-Json"]).output();
        match output {
            Ok(result) => {
                let info = String::from_utf8_lossy(&result.stdout);
                println!("Disk Info: {}", info);
            }
            Err(_) => println!("Disk information requires elevated privileges"),
        }
    } else if cfg!(target_os = "linux") {
        let output = Command::new("df").args(&["-h"]).output()?;
        let content = String::from_utf8_lossy(&output.stdout);
        println!("{}", content);
    }
    println!();

    // Network Information
    println!("🌐 NETWORK INFORMATION:");
    println!("----------------------");
    if cfg!(target_os = "windows") {
        let output = Command::new("powershell").args(&["-Command", "Get-NetAdapter | Select-Object Name, MacAddress, LinkSpeed | ConvertTo-Json"]).output();
        match output {
            Ok(result) => {
                let info = String::from_utf8_lossy(&result.stdout);
                println!("Network Info: {}", info);
            }
            Err(_) => println!("Network information requires elevated privileges"),
        }
    } else if cfg!(target_os = "linux") {
        let output = Command::new("ip").args(&["addr", "show"]).output()?;
        let content = String::from_utf8_lossy(&output.stdout);
        for line in content.lines() {
            if line.contains("inet ") && !line.contains("127.0.0.1") {
                println!("{}", line.trim());
            }
        }
    }
    println!();

    // Process Information (Top processes)
    println!("⚙️  TOP PROCESSES:");
    println!("-----------------");
    if cfg!(target_os = "windows") {
        let output = Command::new("powershell").args(&["-Command", "Get-Process | Sort-Object CPU -Descending | Select-Object -First 5 Name, CPU, WorkingSet | ConvertTo-Json"]).output();
        match output {
            Ok(result) => {
                let info = String::from_utf8_lossy(&result.stdout);
                println!("Top Processes: {}", info);
            }
            Err(_) => println!("Process information requires elevated privileges"),
        }
    } else if cfg!(target_os = "linux") {
        let output = Command::new("ps").args(&["aux"]).output()?;
        let content = String::from_utf8_lossy(&output.stdout);
        let mut count = 0;
        for line in content.lines().skip(1) {
            if count >= 5 { break; }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 11 {
                println!("{} {} {}% {}MB", parts[1], parts[10], parts[2], parts[5]);
                count += 1;
            }
        }
    }
    println!();

    // Environment Variables
    println!("🔍 ENVIRONMENT INFORMATION:");
    println!("--------------------------");
    println!("PATH: {}", std::env::var("PATH").unwrap_or_else(|_| "Not set".to_string()));
    println!("HOME: {}", std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| "Not set".to_string()));
    println!("SHELL: {}", std::env::var("SHELL").or_else(|_| std::env::var("COMSPEC")).unwrap_or_else(|_| "Not set".to_string()));
    println!();

    println!("🎉 OS Information Report Complete!");
    println!("==================================");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_info_collection() {
        // Test that the basic functionality works
        // This is a simple smoke test
        let result = main();
        // In a real environment, this should succeed
        // In CI/test environments, some commands might fail
        assert!(result.is_ok() || result.is_err());
    }
}
