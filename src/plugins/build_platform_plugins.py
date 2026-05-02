#!/usr/bin/env python3
"""
Platform-aware plugin builder for Mini MSP Agent
Automatically detects platform and uses appropriate CMakeLists.txt
"""

import os
import sys
import platform
import subprocess
import shutil
from pathlib import Path

def detect_platform():
    """Detect the current platform"""
    system = platform.system().lower()
    if system == "windows":
        return "windows"
    elif system == "linux":
        return "linux"
    elif system == "darwin":
        return "macos"
    else:
        return "unknown"

def get_cmake_file(platform_name):
    """Get the CMakeLists.txt file (unified for all platforms)"""
    # Project uses unified CMakeLists.txt for all platforms
    return "CMakeLists.txt"

def copy_plugins_to_platform_dir(platform_name, source_dir):
    """Copy built plugins to platform-specific directory"""
    platform_dir = source_dir / platform_name
    platform_dir.mkdir(exist_ok=True)
    
    # Copy DLL files to platform directory
    for dll_file in source_dir.glob("*.dll"):
        dest = platform_dir / dll_file.name
        shutil.copy2(dll_file, dest)
        print(f"✅ Copied {dll_file.name} to {platform_dir}")
    
    # Copy SO files for Linux/macOS
    if platform_name in ["linux", "macos"]:
        for so_file in source_dir.glob("*.so"):
            dest = platform_dir / so_file.name
            shutil.copy2(so_file, dest)
            print(f"✅ Copied {so_file.name} to {platform_dir}")

def main():
    """Main build function"""
    print("🔧 Platform-aware Plugin Builder for Mini MSP Agent")
    
    # Detect platform
    current_platform = detect_platform()
    print(f"🖥️  Detected platform: {current_platform}")
    
    # Get script directory
    script_dir = Path(__file__).parent
    source_dir = script_dir
    
    # Create build directory
    build_dir = source_dir / "build"
    build_dir.mkdir(exist_ok=True)
    
    # Get appropriate CMakeLists.txt
    cmake_file = get_cmake_file(current_platform)
    cmake_source = source_dir / cmake_file
    
    if not cmake_source.exists():
        print(f"❌ CMake file not found: {cmake_file}")
        return 1
    
    print(f"📋 Using CMake file: {cmake_file} (unified for all platforms)")
    
    # Copy unified CMakeLists.txt to build directory
    shutil.copy2(cmake_source, build_dir / "CMakeLists.txt")
    print(f"✅ Copied {cmake_file} to build/CMakeLists.txt")
    
    # Change to build directory
    os.chdir(build_dir)
    
    # Configure with CMake
    print("🔧 Configuring with CMake...")
    cmake_cmd = ["cmake", ".", "-DCMAKE_BUILD_TYPE=Release"]
    
    if current_platform == "windows":
        # Try to find Visual Studio
        vs_where = r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe"
        if os.path.exists(vs_where):
            try:
                result = subprocess.run([vs_where, "-latest", "-products", "*", "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-property", "installationPath"], capture_output=True, text=True)
                if result.returncode == 0:
                    vs_path = result.stdout.strip()
                    vc_vars = os.path.join(vs_path, "VC", "Auxiliary", "Build", "vcvars64.bat")
                    if os.path.exists(vc_vars):
                        print("🔧 Setting up Visual Studio environment...")
                        cmake_cmd.insert(0, f'cmd /c "{vc_vars}" && set')
            except Exception as e:
                print(f"⚠️  VS detection failed: {e}")
    
    # Run CMake configure
    result = subprocess.run(cmake_cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"❌ CMake configuration failed:")
        print(result.stderr)
        return 1
    
    print("✅ CMake configuration successful")
    
    # Build with appropriate tool
    print("🏗️  Building plugins...")
    
    if current_platform == "windows":
        build_cmd = ["ninja"]
    else:
        build_cmd = ["make", "-j4"]
    
    result = subprocess.run(build_cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"❌ Build failed:")
        print(result.stderr)
        return 1
    
    print("✅ Build successful")
    
    # Copy plugins to platform directory
    copy_plugins_to_platform_dir(current_platform, source_dir)
    
    print(f"🎉 Platform-specific build completed for {current_platform}")
    return 0

if __name__ == "__main__":
    sys.exit(main())
