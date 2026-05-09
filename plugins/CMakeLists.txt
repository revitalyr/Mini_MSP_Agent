cmake_minimum_required(VERSION 3.20)
project(MiniMSPPlugins VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Platform-specific configuration
if(WIN32)
    add_definitions(-DWIN32_LEAN_AND_MEAN)
    add_definitions(-DNOMINMAX)
endif()

# Include directories
include_directories(
    ${CMAKE_CURRENT_SOURCE_DIR}/common
    ${CMAKE_CURRENT_SOURCE_DIR}/cpp
)

# Source files
file(GLOB_RECURSE PLUGIN_SOURCES
    "cpp/*.cpp"
    "common/*.cpp"
)

file(GLOB_RECURSE PLUGIN_HEADERS
    "cpp/*.h"
    "common/*.h"
)

# Create plugin library
add_library(mini_msp_plugins STATIC ${PLUGIN_SOURCES} ${PLUGIN_HEADERS})

# Platform-specific libraries
if(WIN32)
    target_link_libraries(mini_msp_plugins 
        kernel32
        user32
        shell32
        ole32
        oleaut32
        uuid
        comdlg32
        advapi32
    )
else()
    target_link_libraries(mini_msp_plugins
        pthread
        dl
    )
endif()

# Compiler-specific options
if(CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
    target_compile_options(mini_msp_plugins PRIVATE -Wall -Wextra -Wpedantic)
elseif(CMAKE_CXX_COMPILER_ID STREQUAL "MSVC")
    target_compile_options(mini_msp_plugins PRIVATE /W4)
endif()

# Install rules
install(TARGETS mini_msp_plugins
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
)
