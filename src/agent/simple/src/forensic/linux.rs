use serde_json::{json, Value};
use std::fs;
use std::process::Command;

pub struct LinuxForensicCollector;

impl super::ForensicCollector for LinuxForensicCollector {
    fn collect(&self) -> Value {
        let mut findings = vec![];
        
        // 1. Kernel modules
        if let Ok(modules) = fs::read_to_string("/proc/modules") {
            for line in modules.lines().take(20) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    findings.push(json!({
                        "category": "Kernel Modules",
                        "severity": "info",
                        "title": format!("Module: {}", parts[0]),
                        "description": format!("Size: {}, Used: {}", parts[1], parts[2]),
                        "type": "kernel_module"
                    }));
                }
            }
        }
        
        // 2. Running processes from /proc
        if let Ok(entries) = fs::read_dir("/proc") {
            let mut processes = vec![];
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.chars().all(|c| c.is_ascii_digit()) {
                    let pid: i32 = name_str.parse().unwrap_or(0);
                    if let Ok(cmdline) = fs::read_to_string(format!("/proc/{}/cmdline", pid)) {
                        let cmd = cmdline.replace('\0', " ").trim().to_string();
                        if !cmd.is_empty() && cmd.len() < 200 {
                            processes.push(json!({
                                "pid": pid,
                                "cmd": cmd
                            }));
                        }
                    }
                }
            }
            findings.push(json!({
                "category": "Processes",
                "severity": "info", 
                "title": format!("Running Processes: {}", processes.len()),
                "description": "Active processes from /proc",
                "type": "processes",
                "data": processes.into_iter().take(10).collect::<Vec<_>>()
            }));
        }
        
        // 3. Crontab entries
        let cron_locations = vec![
            "/etc/crontab",
            "/etc/cron.d",
            "/var/spool/cron/crontabs",
        ];
        for location in cron_locations {
            if fs::metadata(location).is_ok() {
                findings.push(json!({
                    "category": "Persistence",
                    "severity": "info",
                    "title": format!("Cron location: {}", location),
                    "description": "Potential persistence mechanism",
                    "type": "cron"
                }));
            }
        }
        
        // 4. Systemd services
        if let Ok(output) = Command::new("systemctl").args(&["list-units", "--type=service", "--state=running"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let services: Vec<&str> = stdout.lines().skip(1).take(5).collect();
            findings.push(json!({
                "category": "Services",
                "severity": "info",
                "title": "Running Systemd Services",
                "description": services.join(", "),
                "type": "systemd"
            }));
        }
        
        // 5. Network connections
        if let Ok(tcp) = fs::read_to_string("/proc/net/tcp") {
            let listening: Vec<&str> = tcp.lines()
                .skip(1)
                .filter(|l| l.contains(":0000 0A "))  // LISTEN state
                .take(5)
                .collect();
            if !listening.is_empty() {
                findings.push(json!({
                    "category": "Network",
                    "severity": "warning",
                    "title": "Listening TCP Ports",
                    "description": format!("{} listening ports found", listening.len()),
                    "type": "network"
                }));
            }
        }
        
        // 6. Loaded kernel modules (security check)
        if let Ok(modules) = fs::read_to_string("/proc/modules") {
            let suspicious = vec!["rootkit", "hide", "hook", "backdoor"];
            for line in modules.lines() {
                for s in &suspicious {
                    if line.to_lowercase().contains(s) {
                        findings.push(json!({
                            "category": "IOC Detection",
                            "severity": "critical",
                            "title": format!("Suspicious Module: {}", line.split_whitespace().next().unwrap_or("unknown")),
                            "description": format!("Contains suspicious keyword: {}", s),
                            "type": "suspicious_module"
                        }));
                    }
                }
            }
        }
        
        // 7. Environment variables of init process (PID 1)
        if let Ok(environ) = fs::read_to_string("/proc/1/environ") {
            let env_vars: Vec<String> = environ.split('\0')
                .filter(|s| !s.is_empty())
                .take(5)
                .map(|s| s.to_string())
                .collect();
            findings.push(json!({
                "category": "System",
                "severity": "info",
                "title": "Init Process Environment",
                "description": env_vars.join(", "),
                "type": "environment"
            }));
        }
        
        // 8. Check for SUID binaries (common privilege escalation vector)
        let suid_paths = vec!["/usr/bin/sudo", "/bin/su", "/usr/bin/pkexec"];
        for path in suid_paths {
            if fs::metadata(path).is_ok() {
                findings.push(json!({
                    "category": "Privilege Escalation",
                    "severity": "info",
                    "title": format!("SUID binary: {}", path),
                    "description": "Binary has setuid bit - potential privilege escalation vector",
                    "type": "suid"
                }));
            }
        }
        
        json!({
            "platform": "linux",
            "findings_count": findings.len(),
            "findings": findings
        })
    }
    
    fn platform(&self) -> &'static str {
        "linux"
    }
}
