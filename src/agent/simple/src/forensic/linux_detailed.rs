use serde_json::{json, Value};
use std::fs;
use std::process::Command;

/// Detailed Linux forensic collector with comprehensive artifact collection
pub struct LinuxDetailedForensicCollector;

impl super::ForensicCollector for LinuxDetailedForensicCollector {
    fn collect(&self) -> Value {
        self.collect_detailed()
    }
    
    fn platform(&self) -> &'static str {
        "linux_detailed"
    }
}

impl LinuxDetailedForensicCollector {
    pub fn collect_detailed(&self) -> Value {
        let mut findings = vec![];
        
        // 1. Authentication & User Accounts
        findings.extend(self.collect_auth_artifacts());
        
        // 2. File System Artifacts
        findings.extend(self.collect_filesystem_artifacts());
        
        // 3. Persistence Mechanisms
        findings.extend(self.collect_persistence());
        
        // 4. Network Artifacts
        findings.extend(self.collect_network());
        
        // 5. Logs
        findings.extend(self.collect_logs());
        
        // 6. Process & Memory Artifacts
        findings.extend(self.collect_processes());
        
        // 7. Systemd & Services
        findings.extend(self.collect_systemd());
        
        json!({
            "platform": "linux",
            "variant": "detailed",
            "findings_count": findings.len(),
            "findings": findings
        })
    }
    
    fn collect_auth_artifacts(&self) -> Vec<Value> {
        let mut findings = vec![];
        
        // /etc/passwd - user accounts
        if let Ok(content) = fs::read_to_string("/etc/passwd") {
            let users: Vec<String> = content.lines()
                .filter(|l| !l.starts_with('#'))
                .map(|l| l.split(':').next().unwrap_or("").to_string())
                .filter(|u| !u.is_empty())
                .collect();
            findings.push(json!({
                "category": "Users & Authentication",
                "severity": "info",
                "title": "/etc/passwd entries",
                "description": format!("Found {} user accounts", users.len()),
                "type": "passwd",
                "users": users
            }));
        }
        
        // Check for SSH keys
        if let Ok(entries) = fs::read_dir("/home") {
            for entry in entries.flatten() {
                let auth_keys = entry.path().join(".ssh/authorized_keys");
                if auth_keys.exists() {
                    if let Ok(keys) = fs::read_to_string(&auth_keys) {
                        let key_count = keys.lines().filter(|l| !l.is_empty() && !l.starts_with('#')).count();
                        if key_count > 0 {
                            findings.push(json!({
                                "category": "Users & Authentication",
                                "severity": "warning",
                                "title": format!("SSH authorized_keys for {:?}", entry.file_name()),
                                "description": format!("{} SSH keys configured", key_count),
                                "type": "ssh_keys",
                                "path": auth_keys.to_string_lossy()
                            }));
                        }
                    }
                }
            }
        }
        
        // Check root SSH keys
        if let Ok(keys) = fs::read_to_string("/root/.ssh/authorized_keys") {
            let key_count = keys.lines().filter(|l| !l.is_empty() && !l.starts_with('#')).count();
            if key_count > 0 {
                findings.push(json!({
                    "category": "Users & Authentication",
                    "severity": "critical",
                    "title": "Root SSH authorized_keys",
                    "description": format!("{} SSH keys for root - potential security risk", key_count),
                    "type": "ssh_keys_root"
                }));
            }
        }
        
        // Check sudoers
        if fs::metadata("/etc/sudoers").is_ok() {
            findings.push(json!({
                "category": "Users & Authentication",
                "severity": "warning",
                "title": "/etc/sudoers",
                "description": "Sudo configuration file exists",
                "type": "sudoers"
            }));
        }
        
        // Check for passwordless sudo
        if let Ok(output) = Command::new("grep").args(&["-r", "NOPASSWD", "/etc/sudoers.d/"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !stdout.is_empty() {
                findings.push(json!({
                    "category": "Users & Authentication",
                    "severity": "critical",
                    "title": "Passwordless sudo detected",
                    "description": "NOPASSWD entries found in sudoers",
                    "type": "nopasswd_sudo",
                    "matches": stdout.lines().collect::<Vec<_>>()
                }));
            }
        }
        
        // Shell history files
        for user_dir in ["/home", "/root"] {
            if let Ok(entries) = fs::read_dir(user_dir) {
                for entry in entries.flatten() {
                    for hist_file in [".bash_history", ".zsh_history", ".sh_history"] {
                        let hist_path = entry.path().join(hist_file);
                        if hist_path.exists() {
                            findings.push(json!({
                                "category": "Users & Authentication",
                                "severity": "info",
                                "title": format!("Shell history: {:?}/{}", entry.file_name(), hist_file),
                                "description": "Command history file exists",
                                "type": "shell_history"
                            }));
                        }
                    }
                }
            }
        }
        
        findings
    }
    
    fn collect_filesystem_artifacts(&self) -> Vec<Value> {
        let mut findings = vec![];
        
        // SUID/SGID binaries
        let suid_paths = vec!["/usr/bin/sudo", "/bin/su", "/usr/bin/pkexec", "/usr/bin/passwd", "/bin/mount", "/bin/umount"];
        for path in suid_paths {
            if let Ok(metadata) = fs::metadata(path) {
                use std::os::unix::fs::PermissionsExt;
                let mode = metadata.permissions().mode();
                if mode & 0o4000 != 0 { // SUID bit
                    findings.push(json!({
                        "category": "File System",
                        "severity": "warning",
                        "title": format!("SUID binary: {}", path),
                        "description": "SetUID bit set - potential privilege escalation vector",
                        "type": "suid_binary"
                    }));
                }
                if mode & 0o2000 != 0 { // SGID bit
                    findings.push(json!({
                        "category": "File System",
                        "severity": "warning",
                        "title": format!("SGID binary: {}", path),
                        "description": "SetGID bit set",
                        "type": "sgid_binary"
                    }));
                }
            }
        }
        
        // Hidden files in /home
        let mut hidden_count = 0;
        if let Ok(entries) = fs::read_dir("/home") {
            for entry in entries.flatten() {
                if let Ok(files) = fs::read_dir(entry.path()) {
                    for file in files.flatten() {
                        if file.file_name().to_string_lossy().starts_with('.') {
                            hidden_count += 1;
                        }
                    }
                }
            }
        }
        if hidden_count > 0 {
            findings.push(json!({
                "category": "File System",
                "severity": "info",
                "title": "Hidden files in home directories",
                "description": format!("Found {} hidden files/directories", hidden_count),
                "type": "hidden_files"
            }));
        }
        
        // Check /tmp and /var/tmp for suspicious files
        for tmp_dir in ["/tmp", "/var/tmp"] {
            if let Ok(metadata) = fs::metadata(tmp_dir) {
                findings.push(json!({
                    "category": "File System",
                    "severity": "info",
                    "title": format!("Temporary directory: {}", tmp_dir),
                    "description": "Check for suspicious files",
                    "type": "temp_directory"
                }));
            }
        }
        
        findings
    }
    
    fn collect_persistence(&self) -> Vec<Value> {
        let mut findings = vec![];
        
        // Cron jobs
        let cron_paths = vec!["/etc/crontab", "/etc/cron.d", "/etc/cron.daily", "/etc/cron.weekly", "/etc/cron.monthly", "/var/spool/cron/crontabs"];
        for path in cron_paths {
            if fs::metadata(path).is_ok() {
                findings.push(json!({
                    "category": "Persistence",
                    "severity": "warning",
                    "title": format!("Cron location: {}", path),
                    "description": "Potential persistence mechanism",
                    "type": "cron"
                }));
            }
        }
        
        // Shell profile persistence
        let profile_files = vec!["/etc/profile", "/etc/bash.bashrc", "/etc/zsh/zshrc"];
        for profile in profile_files {
            if fs::metadata(profile).is_ok() {
                findings.push(json!({
                    "category": "Persistence",
                    "severity": "warning",
                    "title": format!("System shell profile: {}", profile),
                    "description": "Global shell initialization - potential persistence",
                    "type": "shell_profile"
                }));
            }
        }
        
        // LD_PRELOAD check
        if let Ok(output) = Command::new("sh").arg("-c").arg("env | grep LD_PRELOAD").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !stdout.is_empty() {
                findings.push(json!({
                    "category": "Persistence",
                    "severity": "critical",
                    "title": "LD_PRELOAD detected",
                    "description": format!("Library injection: {}", stdout.trim()),
                    "type": "ld_preload"
                }));
            }
        }
        
        // Kernel modules
        if let Ok(modules) = fs::read_to_string("/proc/modules") {
            let loaded_count = modules.lines().count();
            findings.push(json!({
                "category": "Persistence",
                "severity": "info",
                "title": "Loaded kernel modules",
                "description": format!("{} kernel modules loaded", loaded_count),
                "type": "kernel_modules"
            }));
        }
        
        findings
    }
    
    fn collect_network(&self) -> Vec<Value> {
        let mut findings = vec![];
        
        // Network connections from /proc/net/tcp
        if let Ok(tcp) = fs::read_to_string("/proc/net/tcp") {
            let mut listening = 0;
            let mut established = 0;
            for line in tcp.lines().skip(1) {
                if line.contains(":0000 0A ") {
                    listening += 1;
                } else if line.contains(":0000 01 ") {
                    established += 1;
                }
            }
            if listening > 0 {
                findings.push(json!({
                    "category": "Network",
                    "severity": "warning",
                    "title": "Listening TCP ports",
                    "description": format!("{} listening ports found", listening),
                    "type": "listening_ports",
                    "established": established
                }));
            }
        }
        
        // ARP cache
        if let Ok(arp) = fs::read_to_string("/proc/net/arp") {
            let entries = arp.lines().skip(1).count();
            findings.push(json!({
                "category": "Network",
                "severity": "info",
                "title": "ARP cache",
                "description": format!("{} ARP entries", entries),
                "type": "arp_cache"
            }));
        }
        
        // Network interfaces
        if let Ok(ifaces) = fs::read_dir("/sys/class/net") {
            let iface_count = ifaces.flatten().count();
            findings.push(json!({
                "category": "Network",
                "severity": "info",
                "title": "Network interfaces",
                "description": format!("{} network interfaces detected", iface_count),
                "type": "interfaces"
            }));
        }
        
        // Check iptables
        if fs::metadata("/sbin/iptables").is_ok() {
            if let Ok(output) = Command::new("iptables").arg("-L").output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let rules = stdout.lines().filter(|l| l.starts_with("ACCEPT") || l.starts_with("DROP")).count();
                findings.push(json!({
                    "category": "Network",
                    "severity": "info",
                    "title": "iptables rules",
                    "description": format!("{} firewall rules configured", rules),
                    "type": "firewall"
                }));
            }
        }
        
        // /etc/hosts
        if let Ok(hosts) = fs::read_to_string("/etc/hosts") {
            let entries = hosts.lines()
                .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
                .count();
            if entries > 2 {
                findings.push(json!({
                    "category": "Network",
                    "severity": "warning",
                    "title": "/etc/hosts entries",
                    "description": format!("{} custom host entries", entries - 2),
                    "type": "hosts_file"
                }));
            }
        }
        
        findings
    }
    
    fn collect_logs(&self) -> Vec<Value> {
        let mut findings = vec![];
        
        let log_locations = vec![
            ("/var/log/syslog", "System log"),
            ("/var/log/messages", "Messages log"),
            ("/var/log/auth.log", "Authentication log"),
            ("/var/log/secure", "Security log (RHEL)"),
            ("/var/log/kern.log", "Kernel log"),
            ("/var/log/dmesg", "Boot messages"),
        ];
        
        for (path, desc) in log_locations {
            if fs::metadata(path).is_ok() {
                findings.push(json!({
                    "category": "Logs",
                    "severity": "info",
                    "title": desc,
                    "description": format!("Log file: {}", path),
                    "type": "log_file"
                }));
            }
        }
        
        // Check for auditd
        if fs::metadata("/var/log/audit/audit.log").is_ok() {
            findings.push(json!({
                "category": "Logs",
                "severity": "info",
                "title": "Linux Audit Framework",
                "description": "auditd logging active",
                "type": "auditd"
            }));
        }
        
        // wtmp/utmp
        if fs::metadata("/var/log/wtmp").is_ok() {
            findings.push(json!({
                "category": "Logs",
                "severity": "info",
                "title": "Login history (wtmp)",
                "description": "Binary login records",
                "type": "wtmp"
            }));
        }
        
        findings
    }
    
    fn collect_processes(&self) -> Vec<Value> {
        let mut findings = vec![];
        
        // Running processes from /proc
        let mut processes = vec![];
        if let Ok(entries) = fs::read_dir("/proc") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.chars().all(|c| c.is_ascii_digit()) {
                    let pid: i32 = name_str.parse().unwrap_or(0);
                    
                    // Get cmdline
                    let mut cmd = String::new();
                    if let Ok(cmdline) = fs::read_to_string(format!("/proc/{}/cmdline", pid)) {
                        cmd = cmdline.replace('\0', " ").trim().to_string();
                    }
                    
                    // Get status
                    let mut status_info = String::new();
                    if let Ok(status) = fs::read_to_string(format!("/proc/{}/status", pid)) {
                        for line in status.lines() {
                            if line.starts_with("Name:") || line.starts_with("State:") {
                                status_info.push_str(line);
                                status_info.push(' ');
                            }
                        }
                    }
                    
                    if !cmd.is_empty() || !status_info.is_empty() {
                        processes.push(json!({
                            "pid": pid,
                            "cmd": cmd,
                            "status": status_info
                        }));
                    }
                }
            }
        }
        
        findings.push(json!({
            "category": "Processes",
            "severity": "info",
            "title": "Running processes",
            "description": format!("{} processes currently running", processes.len()),
            "type": "processes",
            "top_processes": processes.into_iter().take(15).collect::<Vec<_>>()
        }));
        
        // Environment of init (PID 1)
        if let Ok(environ) = fs::read_to_string("/proc/1/environ") {
            let env_vars: Vec<String> = environ.split('\0')
                .filter(|s| !s.is_empty())
                .filter(|s| s.contains('='))
                .map(|s| s.split('=').next().unwrap_or("").to_string())
                .filter(|s| !s.is_empty())
                .take(10)
                .collect();
            findings.push(json!({
                "category": "Processes",
                "severity": "info",
                "title": "Init (PID 1) environment",
                "description": format!("Variables: {:?}", env_vars),
                "type": "init_env"
            }));
        }
        
        // Open file descriptors for suspicious processes
        if let Ok(entries) = fs::read_dir("/proc") {
            let mut suspicious = vec![];
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.chars().all(|c| c.is_ascii_digit()) {
                    let fd_dir = entry.path().join("fd");
                    if let Ok(fds) = fs::read_dir(&fd_dir) {
                        let count = fds.flatten().count();
                        if count > 100 { // Suspicious number of open FDs
                            suspicious.push(json!({
                                "pid": name_str.parse::<i32>().unwrap_or(0),
                                "open_fds": count
                            }));
                        }
                    }
                }
            }
            if !suspicious.is_empty() {
                findings.push(json!({
                    "category": "Processes",
                    "severity": "warning",
                    "title": "Processes with many open file descriptors",
                    "description": format!("{} processes with >100 FDs", suspicious.len()),
                    "type": "suspicious_fds",
                    "processes": suspicious
                }));
            }
        }
        
        findings
    }
    
    fn collect_systemd(&self) -> Vec<Value> {
        let mut findings = vec![];
        
        // Check if systemd is used
        if fs::metadata("/run/systemd/system").is_ok() {
            findings.push(json!({
                "category": "Systemd",
                "severity": "info",
                "title": "Systemd detected",
                "description": "System is using systemd as init",
                "type": "systemd"
            }));
            
            // List running services
            if let Ok(output) = Command::new("systemctl").args(&["list-units", "--type=service", "--state=running", "--no-pager", "--no-legend"]).output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let services: Vec<&str> = stdout.lines().take(10).collect();
                findings.push(json!({
                    "category": "Systemd",
                    "severity": "info",
                    "title": "Running services",
                    "description": format!("Top running services: {}", services.join(", ")),
                    "type": "services"
                }));
            }
            
            // Timers
            if let Ok(output) = Command::new("systemctl").args(&["list-timers", "--all"]).output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let timer_count = stdout.lines().filter(|l| l.contains(".timer")).count();
                if timer_count > 0 {
                    findings.push(json!({
                        "category": "Systemd",
                        "severity": "warning",
                        "title": "Systemd timers",
                        "description": format!("{} timer units configured", timer_count),
                        "type": "timers"
                    }));
                }
            }
        }
        
        findings
    }
}
