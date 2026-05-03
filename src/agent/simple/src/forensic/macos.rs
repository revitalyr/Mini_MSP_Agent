use serde_json::{json, Value};

pub struct MacOSForensicCollector;

impl super::ForensicCollector for MacOSForensicCollector {
    fn collect(&self) -> Value {
        let findings = vec![
            // LaunchAgents - user persistence
            json!({
                "category": "Persistence",
                "severity": "warning",
                "title": "LaunchAgents (User)",
                "description": "~/Library/LaunchAgents - User-level persistence mechanism",
                "type": "launchagents",
                "path": "~/Library/LaunchAgents"
            }),
            // LaunchDaemons - system persistence  
            json!({
                "category": "Persistence",
                "severity": "critical",
                "title": "LaunchDaemons (System)",
                "description": "/Library/LaunchDaemons - System-level persistence mechanism",
                "type": "launchdaemons",
                "path": "/Library/LaunchDaemons"
            }),
            // StartupItems (deprecated but still used)
            json!({
                "category": "Persistence",
                "severity": "warning",
                "title": "StartupItems",
                "description": "/Library/StartupItems - Legacy startup mechanism",
                "type": "startupitems",
                "path": "/Library/StartupItems"
            }),
            // Login Items
            json!({
                "category": "Persistence",
                "severity": "info",
                "title": "Login Items",
                "description": "Applications set to launch at user login via System Preferences",
                "type": "loginitems"
            }),
            // Kernel extensions
            json!({
                "category": "Kernel",
                "severity": "critical",
                "title": "Kernel Extensions (KEXTs)",
                "description": "/Library/Extensions - Kernel-level code that could hide rootkits",
                "type": "kexts",
                "paths": vec!["/Library/Extensions", "/System/Library/Extensions"]
            }),
            // System Extensions (modern replacement for KEXTs)
            json!({
                "category": "Kernel",
                "severity": "warning",
                "title": "System Extensions",
                "description": "Modern driver extensions - replacement for KEXTs in 10.15+",
                "type": "system_extensions"
            }),
            // Cron on macOS
            json!({
                "category": "Persistence",
                "severity": "warning",
                "title": "Crontab Entries",
                "description": "/usr/lib/cron/tabs/ - User crontab files",
                "type": "cron",
                "path": "/usr/lib/cron/tabs/"
            }),
            // Periodic scripts
            json!({
                "category": "Persistence",
                "severity": "info",
                "title": "Periodic Scripts",
                "description": "/etc/periodic/ - Scheduled maintenance scripts",
                "type": "periodic",
                "paths": vec!["/etc/periodic/daily", "/etc/periodic/weekly", "/etc/periodic/monthly"]
            }),
            // Emond (Event Monitor Daemon)
            json!({
                "category": "Persistence",
                "severity": "critical",
                "title": "Emond (Event Monitor Daemon)",
                "description": "/private/var/db/emondClients - Event-based persistence mechanism",
                "type": "emond",
                "path": "/private/var/db/emondClients"
            }),
            // Authorization plugins
            json!({
                "category": "Persistence",
                "severity": "critical",
                "title": "Authorization Plugins",
                "description": "/Library/Security/SecurityAgentPlugins - Login hook persistence",
                "type": "auth_plugins",
                "path": "/Library/Security/SecurityAgentPlugins"
            }),
            // Universal Access plugins
            json!({
                "category": "Persistence",
                "severity": "warning",
                "title": "Universal Access Plugins",
                "description": "Accessibility plugin persistence mechanism",
                "type": "universal_access"
            }),
            // Bash/Zsh profiles
            json!({
                "category": "Persistence",
                "severity": "warning",
                "title": "Shell Profile Persistence",
                "description": "~/.bash_profile, ~/.zshrc - Shell initialization scripts",
                "type": "shell_profiles",
                "files": vec!["~/.bash_profile", "~/.bashrc", "~/.zshrc", "~/.zprofile"]
            }),
            // Browser extensions
            json!({
                "category": "Persistence",
                "severity": "info",
                "title": "Browser Extensions",
                "description": "Safari/Chrome/Firefox extensions - potential adware/malware vectors",
                "type": "browser_extensions"
            }),
            // Application bundles
            json!({
                "category": "Persistence",
                "severity": "info",
                "title": "Installed Applications",
                "description": "/Applications - Check for suspicious/malicious apps",
                "type": "applications",
                "paths": vec!["/Applications", "~/Applications"]
            }),
            // Unified Logs
            json!({
                "category": "Logs",
                "severity": "info",
                "title": "Unified Logs",
                "description": "log show -- unified logging system (10.12+)",
                "type": "unified_logs",
                "command": "log show"
            }),
            // Airport preferences (WiFi history)
            json!({
                "category": "Evidence",
                "severity": "info",
                "title": "WiFi Connection History",
                "description": "/Library/Preferences/SystemConfiguration/com.apple.airport.preferences.plist",
                "type": "wifi_history",
                "path": "/Library/Preferences/SystemConfiguration/com.apple.airport.preferences.plist"
            }),
            // Quarantine database
            json!({
                "category": "Evidence",
                "severity": "warning",
                "title": "Quarantine Database",
                "description": "~/Library/Preferences/com.apple.LaunchServices.QuarantineEventsV2 - Downloaded files history",
                "type": "quarantine",
                "path": "~/Library/Preferences/com.apple.LaunchServices.QuarantineEventsV2"
            }),
            // Recent items
            json!({
                "category": "Evidence",
                "severity": "info",
                "title": "Recent Items",
                "description": "~/.bash_history, recent documents, downloads",
                "type": "recent_items"
            }),
        ];
        
        json!({
            "platform": "macos",
            "findings_count": findings.len(),
            "note": "macOS forensic artifacts - LaunchAgents, LaunchDaemons, KEXTs, and more",
            "findings": findings
        })
    }
    
    fn platform(&self) -> &'static str {
        "macos"
    }
}
