pub mod linux;
pub mod linux_detailed;
pub mod windows;
pub mod macos;
pub mod macos_detailed;

use serde_json::{json, Value};

/// Platform-specific forensic data collector trait
pub trait ForensicCollector {
    fn collect(&self) -> Value;
    fn platform(&self) -> &'static str;
}

/// Get the appropriate forensic collector for current platform
pub fn get_collector() -> Box<dyn ForensicCollector> {
    #[cfg(target_os = "linux")]
    return Box::new(linux_detailed::LinuxDetailedForensicCollector);
    
    #[cfg(target_os = "windows")]
    return Box::new(windows::WindowsForensicCollector);
    
    #[cfg(target_os = "macos")]
    return Box::new(macos_detailed::MacOSDetailedForensicCollector);
    
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    return Box::new(GenericForensicCollector);
}

/// Generic fallback collector
pub struct GenericForensicCollector;

impl ForensicCollector for GenericForensicCollector {
    fn collect(&self) -> Value {
        json!({
            "platform": std::env::consts::OS,
            "note": "Forensic data collection not implemented for this platform",
            "findings": []
        })
    }
    
    fn platform(&self) -> &'static str {
        "generic"
    }
}
