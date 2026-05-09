use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=CMakeLists.txt");
    
    // Set Qt paths
    let qt_include_paths = vec![
        "/usr/include/qt6",
        "/usr/local/include/qt6",
        "/usr/include/qt5",
        "/usr/local/include/qt5",
    ];
    
    let qt_lib_paths = vec![
        "/usr/lib",
        "/usr/local/lib",
        "/usr/lib/x86_64-linux-gnu",
        "/usr/local/lib/x86_64-linux-gnu",
    ];
    
    // Add include paths
    for path in qt_include_paths {
        if PathBuf::from(path).exists() {
            println!("cargo:include={}", path);
        }
    }
    
    // Add library paths
    for path in qt_lib_paths {
        if PathBuf::from(path).exists() {
            println!("cargo:rustc-link-search=native={}", path);
        }
    }
    
    // Link Qt libraries
    println!("cargo:rustc-link-lib=Qt6Core");
    println!("cargo:rustc-link-lib=Qt6Widgets");
    println!("cargo:rustc-link-lib=Qt6Gui");
    
    // Windows specific
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=qt6core");
        println!("cargo:rustc-link-lib=qt6widgets");
        println!("cargo:rustc-link-lib=qt6gui");
    }
    
    // Linux specific
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=Qt6Core");
        println!("cargo:rustc-link-lib=Qt6Widgets");
        println!("cargo:rustc-link-lib=Qt6Gui");
    }
}
