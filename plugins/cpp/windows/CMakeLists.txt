# Windows-specific system plugin CMakeLists.txt

cmake_minimum_required(VERSION 3.20)
project(MiniMSP_Plugin_Windows)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Windows-specific sources
set(WINDOWS_SOURCES
    system_plugin.cpp
    forensic_plugin.cpp
)

# Include common headers
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/../common)

# Create shared library
add_library(mini_msp_plugin_windows SHARED ${WINDOWS_SOURCES})

# Link Windows libraries
target_link_libraries(mini_msp_plugin_windows
    # Windows-specific libraries
)

# Set output name
set_target_properties(mini_msp_plugin_windows PROPERTIES
    OUTPUT_NAME "mini_msp_plugin"
    PREFIX ""
)