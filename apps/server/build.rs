//! Build script for Mini MSP Server
//!
//! Optionally links Boost.DLL Plugin Manager C++ library

use std::env;
use std::path::PathBuf;

fn main() {
    let target = env::var("TARGET").unwrap();
    let _out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    // Check if Boost.DLL C++ libraries exist
    let boost_lib_path = PathBuf::from("../../plugins/cpp/build/lib");
    
    if boost_lib_path.exists() {
        // Enable boost_dll feature
        println!("cargo:rustc-cfg=feature=\"boost_dll\"");
        println!("cargo:rustc-link-search=native={}", boost_lib_path.display());
        
        // Link BoostPluginManager
        println!("cargo:rustc-link-lib=dylib=BoostPluginManager");
        
        // Link Boost libraries
        if let Ok(boost_root) = env::var("BOOST_ROOT") {
            let boost_lib = PathBuf::from(&boost_root).join("lib");
            if boost_lib.exists() {
                println!("cargo:rustc-link-search=native={}", boost_lib.display());
            }
        }
        
        // Platform-specific libraries
        if target.contains("linux") {
            println!("cargo:rustc-link-lib=dylib=boost_filesystem");
            println!("cargo:rustc-link-lib=dylib=boost_system");
            println!("cargo:rustc-link-lib=dylib=stdc++");
            println!("cargo:rustc-link-lib=dylib=pthread");
            println!("cargo:rustc-link-lib=dylib=dl");
        } else if target.contains("macos") {
            println!("cargo:rustc-link-lib=dylib=c++");
            println!("cargo:rustc-link-lib=dylib=boost_filesystem");
            println!("cargo:rustc-link-lib=dylib=boost_system");
        }
        
        // RPATH for runtime library search
        if let Ok(canonical) = boost_lib_path.canonicalize() {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", canonical.display());
        }
        
        println!("cargo:warning=Boost.DLL plugin support ENABLED");
    } else {
        println!("cargo:warning=Boost.DLL C++ libraries not found - building WITHOUT boost plugin support");
        println!("cargo:warning=To enable: cd plugins/cpp && cmake -B build -C CMakeLists.txt.boost && cmake --build build");
    }
    
    // Rebuild if C++ sources change
    println!("cargo:rerun-if-changed=../../plugins/cpp/include");
    println!("cargo:rerun-if-changed=../../plugins/cpp/src");
}
