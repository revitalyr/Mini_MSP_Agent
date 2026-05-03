use serde_json::{json, Value};
use std::fs;
use std::process::Command;

/// Detailed macOS forensic collector with comprehensive artifact collection
pub struct MacOSDetailedForensicCollector;

impl super::ForensicCollector for MacOSDetailedForensicCollector {
    fn collect(&self) -> Value {
        self.collect_detailed()
    }
    
    fn platform(&self) -> &'static str {
        "macos_detailed"
    }
}

impl MacOSDetailedForensicCollector {
    pub fn collect_detailed(&self) -> Value {
        let mut findings = vec![];
        
        // 1. Persistence Mechanisms
        findings.extend(self.collect_persistence());
        
        // 2. User artifacts
        findings.extend(self.collect_user_artifacts());
        
        // 3. System & Security
        findings.extend(self.collect_security());
        
        // 4. Network
        findings.extend(self.collect_network());
        
        // 5. Logs
        findings.extend(self.collect_logs());
        
        // 6. Applications
        findings.extend(self.collect_applications());
        
        json!({
            "platform": "macos",
            "variant": "detailed",
            "findings_count": findings.len(),
            "findings": findings
        })
    }
    
    fn collect_persistence(&self) -> Vec<Value> {
        let mut findings = vec![];
        
        // LaunchAgents (User)
        let user_la = format!("{}/Library/LaunchAgents", std::env::var("HOME").unwrap_or_default());
        if let Ok(entries) = fs::read_dir(&user_la) {
            let count = entries.flatten().filter(|e| e.path().extension().map(|e| e == "plist").unwrap_or(false)).count();
            if count > 0 {
                findings.push(json!({
                    "category": "Persistence",
                    "severity": "warning",
                    "title": "User LaunchAgents",
                    "description": format!("{} .plist files in ~/Library/LaunchAgents", count),
                    "type": "launchagents_user"
                }));
            }
        }
        
        // LaunchAgents (System)
        if let Ok(entries) = fs::read_dir("/Library/LaunchAgents") {
            let count = entries.flatten().filter(|e| e.path().extension().map(|e| e == "plist").unwrap_or(false)).count();
            if count > 0 {
                findings.push(json!({
                    "category": "Persistence",
                    "severity": "critical",
                    "title": "System LaunchAgents",
                    "description": format!("{} .plist files in /Library/LaunchAgents", count),
                    "type": "launchagents_system"
                }));
            }
        }
        
        // LaunchDaemons
        if let Ok(entries) = fs::read_dir("/Library/LaunchDaemons") {
            let count = entries.flatten().filter(|e| e.path().extension().map(|e| e == "plist").unwrap_or(false)).count();
            if count > 0 {
                findings.push(json!({
                    "category": "Persistence",
                    "severity": "critical",
                    "title": "LaunchDaemons",
                    "description": format!("{} .plist files in /Library/LaunchDaemons", count),
                    "type": "launchdaemons"
                }));
            }
        }
        
        // Kernel Extensions
        if let Ok(entries) = fs::read_dir("/Library/Extensions") {
            let count = entries.flatten().count();
            if count > 0 {
                findings.push(json!({
                    "category": "Persistence",
                    "severity": "critical",
                    "title": "Kernel Extensions (KEXTs)",
                    "description": format!("{} third-party kernel extensions", count),
                    "type": "kexts"
                }));
            }
        }
        
        // System Extensions (modern KEXT replacement)
        if let Ok(entries) = fs::read_dir("/Library/SystemExtensions") {
            let count = entries.flatten().count();
            if count > 0 {
                findings.push(json!({
                    "category": "Persistence",
                    "severity": "warning",
                    "title": "System Extensions",
                    "description": format!("{} system extensions installed", count),
                    "type": "system_extensions"
                }));
            }
        }
        
        // Cron (deprecated but still works)
        if fs::metadata("/etc/crontab").is_ok() {
            findings.push(json!({
                "category": "Persistence",
                "severity": "warning",
                "title": "/etc/crontab exists",
                "description": "Legacy cron persistence",
                "type": "cron"
            }));
        }
        
        // Periodic scripts
        let periodic_dirs = vec!["/etc/periodic/daily", "/etc/periodic/weekly", "/etc/periodic/monthly"];
        for dir in periodic_dirs {
            if let Ok(entries) = fs::read_dir(dir) {
                let count = entries.flatten().count();
                if count > 0 {
                    findings.push(json!({
                        "category": "Persistence",
                        "severity": "info",
                        "title": format!("Periodic scripts: {}", dir),
                        "description": format!("{} scripts configured", count),
                        "type": "periodic"
                    }));
                }
            }
        }
        
        // Login Items (background task management)
        let bg_tasks = format!("{}/Library/Application Support/com.apple.backgroundtaskmanagementagent", std::env::var("HOME").unwrap_or_default());
        if fs::metadata(&bg_tasks).is_ok() {
            findings.push(json!({
                "category": "Persistence",
                "severity": "warning",
                "title": "Login Items / Background Tasks",
                "description": "Background task management database exists",
                "type": "login_items"
            }));
        }
        
        findings
    }
    
    fn collect_user_artifacts(&self) -> Vec<Value> {
        let mut findings = vec![];
        let home = std::env::var("HOME").unwrap_or_default();
        
        // Shell history
        for hist in [".bash_history", ".zsh_history", ".sh_history"] {
            let path = format!("{}/{}", home, hist);
            if fs::metadata(&path).is_ok() {
                findings.push(json!({
                    "category": "User Artifacts",
                    "severity": "info",
                    "title": format!("Shell history: {}", hist),
                    "description": "Command history available",
                    "type": "shell_history"
                }));
            }
        }
        
        // SSH
        let ssh_dir = format!("{}/.ssh", home);
        if let Ok(entries) = fs::read_dir(&ssh_dir) {
            let files: Vec<String> = entries.flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .collect();
            if !files.is_empty() {
                findings.push(json!({
                    "category": "User Artifacts",
                    "severity": "warning",
                    "title": "SSH directory",
                    "description": format!("SSH files: {:?}", files),
                    "type": "ssh"
                }));
            }
        }
        
        // Recent Items
        let recent = format!("{}/Library/Preferences/com.apple.recentitems.plist", home);
        if fs::metadata(&recent).is_ok() {
            findings.push(json!({
                "category": "User Artifacts",
                "severity": "info",
                "title": "Recent Items",
                "description": "Recent documents and applications history",
                "type": "recent_items"
            }));
        }
        
        // Quarantine database
        let quarantine = format!("{}/Library/Preferences/com.apple.LaunchServices.QuarantineEventsV2", home);
        if fs::metadata(&quarantine).is_ok() {
            findings.push(json!({
                "category": "User Artifacts",
                "severity": "info",
                "title": "Quarantine Database",
                "description": "Downloaded files history (browser, email attachments)",
                "type": "quarantine_db"
            }));
        }
        
        // KnowledgeC (very valuable!)
        let knowledgec = format!("{}/Library/Application Support/Knowledge/knowledgeC.db", home);
        if fs::metadata(&knowledgec).is_ok() {
            findings.push(json!({
                "category": "User Artifacts",
                "severity": "info",
                "title": "KnowledgeC Database",
                "description": "Application usage, device activity, geolocation history",
                "type": "knowledgec"
            }));
        }
        
        // Safari history
        let safari_history = format!("{}/Library/Safari/History.db", home);
        if fs::metadata(&safari_history).is_ok() {
            findings.push(json!({
                "category": "User Artifacts",
                "severity": "info",
                "title": "Safari History",
                "description": "Browser history database",
                "type": "safari_history"
            }));
        }
        
        // iMessage
        let messages = format!("{}/Library/Messages/chat.db", home);
        if fs::metadata(&messages).is_ok() {
            findings.push(json!({
                "category": "User Artifacts",
                "severity": "info",
                "title": "Messages (iMessage/SMS)",
                "description": "Message history database",
                "type": "imessage"
            }));
        }
        
        findings
    }
    
    fn collect_security(&self) -> Vec<Value> {
        let mut findings = vec![];
        let home = std::env::var("HOME").unwrap_or_default();
        
        // TCC database (Transparency, Consent, Control)
        let tcc_system = "/Library/Application Support/com.apple.TCC/TCC.db";
        let tcc_user = format!("{}/Library/Application Support/com.apple.TCC/TCC.db", home);
        
        for (path, scope) in [(tcc_system, "System"), (&tcc_user, "User")] {
            if fs::metadata(path).is_ok() {
                findings.push(json!({
                    "category": "Security",
                    "severity": "warning",
                    "title": format!("TCC Database ({})", scope),
                    "description": "App permissions for camera, microphone, disk access",
                    "type": "tcc_db"
                }));
            }
        }
        
        // SIP status
        if let Ok(output) = Command::new("csrutil").arg("status").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let enabled = stdout.contains("enabled");
            findings.push(json!({
                "category": "Security",
                "severity": if enabled { "info" } else { "critical" },
                "title": "System Integrity Protection (SIP)",
                "description": if enabled { "SIP is enabled" } else { "SIP is DISABLED - security risk" },
                "type": "sip_status",
                "enabled": enabled
            }));
        }
        
        // Gatekeeper
        if let Ok(output) = Command::new("spctl").arg("--status").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            findings.push(json!({
                "category": "Security",
                "severity": "info",
                "title": "Gatekeeper Status",
                "description": stdout.trim(),
                "type": "gatekeeper"
            }));
        }
        
        // XProtect
        let xprotect = "/Library/Apple/System/Library/CoreServices/XProtect.bundle";
        if fs::metadata(xprotect).is_ok() {
            findings.push(json!({
                "category": "Security",
                "severity": "info",
                "title": "XProtect",
                "description": "Apple's malware signature database present",
                "type": "xprotect"
            }));
        }
        
        findings
    }
    
    fn collect_network(&self) -> Vec<Value> {
        let mut findings = vec![];
        
        // WiFi preferences (network history)
        let wifi_prefs = "/Library/Preferences/SystemConfiguration/com.apple.airport.preferences.plist";
        if fs::metadata(wifi_prefs).is_ok() {
            findings.push(json!({
                "category": "Network",
                "severity": "info",
                "title": "WiFi Connection History",
                "description": "Known WiFi networks and connection history",
                "type": "wifi_history"
            }));
        }
        
        // Network interfaces
        if let Ok(output) = Command::new("ifconfig").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let interfaces: Vec<&str> = stdout.lines()
                .filter(|l| l.contains(":"))
                .filter(|l| !l.starts_with("\t"))
                .filter(|l| l.contains("flags"))
                .map(|l| l.split(':').next().unwrap_or(""))
                .filter(|s| !s.is_empty())
                .collect();
            findings.push(json!({
                "category": "Network",
                "severity": "info",
                "title": "Network Interfaces",
                "description": format!("Interfaces: {:?}", interfaces),
                "type": "interfaces"
            }));
        }
        
        // ARP table
        if let Ok(output) = Command::new("arp").arg("-a").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let entries = stdout.lines().count();
            findings.push(json!({
                "category": "Network",
                "severity": "info",
                "title": "ARP Cache",
                "description": format!("{} ARP entries", entries),
                "type": "arp_cache"
            }));
        }
        
        findings
    }
    
    fn collect_logs(&self) -> Vec<Value> {
        let mut findings = vec![];
        
        // Unified Log - check if we can access it
        findings.push(json!({
            "category": "Logs",
            "severity": "info",
            "title": "Unified Logging System",
            "description": "Binary logs in /var/db/diagnostics/ - use 'log' command",
            "type": "unified_log",
            "command": "log show --predicate 'eventMessage contains \"error\"' --last 1h"
        }));
        
        // System log (legacy)
        if fs::metadata("/var/log/system.log").is_ok() {
            findings.push(json!({
                "category": "Logs",
                "severity": "info",
                "title": "System Log",
                "description": "Legacy syslog format",
                "type": "system_log"
            }));
        }
        
        // Install log
        if fs::metadata("/var/log/install.log").is_ok() {
            findings.push(json!({
                "category": "Logs",
                "severity": "info",
                "title": "Install Log",
                "description": "Software installation history",
                "type": "install_log"
            }));
        }
        
        // ASL logs
        if let Ok(entries) = fs::read_dir("/var/log/asl") {
            let count = entries.flatten().count();
            if count > 0 {
                findings.push(json!({
                    "category": "Logs",
                    "severity": "info",
                    "title": "ASL Logs",
                    "description": format!("{} ASL log files", count),
                    "type": "asl_logs"
                }));
            }
        }
        
        // Crash reports
        let home = std::env::var("HOME").unwrap_or_default();
        let crash_dir = format!("{}/Library/Logs/DiagnosticReports", home);
        if let Ok(entries) = fs::read_dir(&crash_dir) {
            let count = entries.flatten().count();
            if count > 0 {
                findings.push(json!({
                    "category": "Logs",
                    "severity": "warning",
                    "title": "Crash Reports",
                    "description": format!("{} crash/diagnostic reports", count),
                    "type": "crash_reports"
                }));
            }
        }
        
        findings
    }
    
    fn collect_applications(&self) -> Vec<Value> {
        let mut findings = vec![];
        let home = std::env::var("HOME").unwrap_or_default();
        
        // Installed applications
        if let Ok(entries) = fs::read_dir("/Applications") {
            let count = entries.flatten().filter(|e| e.path().extension().map(|e| e == "app").unwrap_or(false)).count();
            findings.push(json!({
                "category": "Applications",
                "severity": "info",
                "title": "System Applications",
                "description": format!("{} .app bundles in /Applications", count),
                "type": "system_apps"
            }));
        }
        
        // User applications
        let user_apps = format!("{}/Applications", home);
        if let Ok(entries) = fs::read_dir(&user_apps) {
            let count = entries.flatten().filter(|e| e.path().extension().map(|e| e == "app").unwrap_or(false)).count();
            if count > 0 {
                findings.push(json!({
                    "category": "Applications",
                    "severity": "info",
                    "title": "User Applications",
                    "description": format!("{} .app bundles in ~/Applications", count),
                    "type": "user_apps"
                }));
            }
        }
        
        // Dock items
        let dock_plist = format!("{}/Library/Preferences/com.apple.dock.plist", home);
        if fs::metadata(&dock_plist).is_ok() {
            findings.push(json!({
                "category": "Applications",
                "severity": "info",
                "title": "Dock Configuration",
                "description": "Applications in Dock",
                "type": "dock"
            }));
        }
        
        findings
    }
}
