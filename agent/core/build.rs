use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    
    // Set build-time information
    let build_time = chrono::Utc::now().to_rfc3339();
    println!("cargo:rustc-env=BUILD_TIME={}", build_time);
    
    // Get git commit hash if available
    if let Ok(output) = std::process::Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
    {
        if output.status.success() {
            let git_hash = String::from_utf8_lossy(&output.stdout).trim();
            println!("cargo:rustc-env=GIT_HASH={}", git_hash);
        }
    }
    
    // Set target triple
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=TARGET_TRIPLE={}", target);
    
    // Create version info
    let version_info = format!(
        r#"pub const BUILD_INFO: &str = "Built: {} | Git: {} | Target: {}";"#,
        build_time,
        env::var("GIT_HASH").unwrap_or_else(|_| "unknown".to_string()),
        target
    );
    
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("build_info.rs");
    fs::write(dest_path, version_info).unwrap();
}
