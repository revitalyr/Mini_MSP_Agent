use serde_json::{json, Value};

pub struct WindowsForensicCollector;

impl super::ForensicCollector for WindowsForensicCollector {
    fn collect(&self) -> Value {
        let findings = vec![
            // Registry autorun locations
            json!({
                "category": "Persistence",
                "severity": "warning",
                "title": "Registry: HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
                "description": "Common persistence mechanism for malware",
                "type": "registry_autorun",
                "registry_path": "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run"
            }),
            json!({
                "category": "Persistence", 
                "severity": "warning",
                "title": "Registry: HKCU\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
                "description": "User-level persistence mechanism",
                "type": "registry_autorun",
                "registry_path": "HKCU\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run"
            }),
            // Winlogon keys
            json!({
                "category": "Persistence",
                "severity": "critical",
                "title": "Winlogon Shell Registry Key",
                "description": "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon\\Shell - Controls user shell",
                "type": "winlogon",
                "registry_path": "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon\\Shell"
            }),
            json!({
                "category": "Persistence",
                "severity": "critical", 
                "title": "Winlogon Userinit Registry Key",
                "description": "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon\\Userinit - Controls user initialization",
                "type": "winlogon",
                "registry_path": "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon\\Userinit"
            }),
            // LSA keys
            json!({
                "category": "Security",
                "severity": "warning",
                "title": "LSA Notification Packages",
                "description": "SYSTEM\\CurrentControlSet\\Control\\Lsa\\Notification Packages",
                "type": "lsa",
                "registry_path": "SYSTEM\\CurrentControlSet\\Control\\Lsa\\Notification Packages"
            }),
            json!({
                "category": "Security",
                "severity": "warning", 
                "title": "LSA Security Providers",
                "description": "SYSTEM\\CurrentControlSet\\Control\\SecurityProviders\\SecurityProviders",
                "type": "lsa",
                "registry_path": "SYSTEM\\CurrentControlSet\\Control\\SecurityProviders\\SecurityProviders"
            }),
            // Services
            json!({
                "category": "Persistence",
                "severity": "info",
                "title": "Windows Services",
                "description": "HKLM\\SYSTEM\\CurrentControlSet\\Services - Service-based persistence",
                "type": "services",
                "registry_path": "HKLM\\SYSTEM\\CurrentControlSet\\Services"
            }),
            // Scheduled tasks
            json!({
                "category": "Persistence",
                "severity": "warning",
                "title": "Scheduled Tasks",
                "description": "C:\\Windows\\System32\\Tasks - Task scheduler persistence",
                "type": "scheduled_tasks",
                "path": "C:\\Windows\\System32\\Tasks"
            }),
            // WMI persistence
            json!({
                "category": "Persistence",
                "severity": "critical",
                "title": "WMI Event Subscriptions",
                "description": "WMI event consumers can be used for persistence",
                "type": "wmi",
                "wmi_namespace": "root\\subscription"
            }),
            // Startup folders
            json!({
                "category": "Persistence",
                "severity": "info",
                "title": "Startup Folders",
                "description": "AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\Startup",
                "type": "startup_folder"
            }),
            // Amcache.hve
            json!({
                "category": "Evidence",
                "severity": "info", 
                "title": "Amcache.hve",
                "description": "Program execution evidence in C:\\Windows\\AppCompat\\Programs\\Amcache.hve",
                "type": "amcache",
                "path": "C:\\Windows\\AppCompat\\Programs\\Amcache.hve"
            }),
            // Prefetch
            json!({
                "category": "Evidence",
                "severity": "info",
                "title": "Prefetch Files",
                "description": "Program execution evidence in C:\\Windows\\Prefetch",
                "type": "prefetch",
                "path": "C:\\Windows\\Prefetch"
            }),
            // Event logs
            json!({
                "category": "Logs",
                "severity": "info",
                "title": "Windows Event Logs",
                "description": "Security, System, Application event logs",
                "type": "event_logs",
                "paths": vec!["C:\\Windows\\System32\\winevt\\Logs\\Security.evtx", "C:\\Windows\\System32\\winevt\\Logs\\System.evtx"]
            }),
        ];
        
        json!({
            "platform": "windows",
            "findings_count": findings.len(),
            "note": "Windows forensic artifacts - registry and file system locations",
            "findings": findings
        })
    }
    
    fn platform(&self) -> &'static str {
        "windows"
    }
}
