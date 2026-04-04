# Modern C++23 Plugin System for Mini MSP Agent
cmake_minimum_required(VERSION 3.25)

project(MiniMSPAgentPlugins
    VERSION 3.0.0
    DESCRIPTION "Modern C++23 Plugin System for Mini MSP Agent"
    LANGUAGES CXX
)

# Set C++23 standard
set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Compiler-specific flags
if(MSVC)
    add_compile_options(
        /std:c++latest
        /permissive-
        /W4
        /EHsc
    )
    add_compile_definitions(
        WIN32_LEAN_AND_MEAN
        NOMINMAX
        _WIN32_WINNT=0x0A00
    )
else()
    add_compile_options(
        -std=c++23
        -Wall
        -Wextra
        -O3
    )
endif()

# Modern system plugin
add_library(modern_system_plugin SHARED
    src/modern_system_plugin_v3.cpp
)

set_target_properties(modern_system_plugin PROPERTIES
    VERSION ${PROJECT_VERSION}
    CXX_STANDARD 23
    CXX_STANDARD_REQUIRED ON
    POSITION_INDEPENDENT_CODE ON
)

if(WIN32)
    target_link_libraries(modern_system_plugin PRIVATE
        kernel32
        user32
        psapi
        advapi32
        shell32
    )
    set_target_properties(modern_system_plugin PROPERTIES
        WINDOWS_EXPORT_ALL_SYMBOLS ON
    )
endif()

# Modern directory plugin
add_library(modern_directory_info_plugin SHARED
    src/modern_directory_info_plugin.cpp
)

set_target_properties(modern_directory_info_plugin PROPERTIES
    VERSION ${PROJECT_VERSION}
    CXX_STANDARD 23
    CXX_STANDARD_REQUIRED ON
    POSITION_INDEPENDENT_CODE ON
)

if(WIN32)
    target_link_libraries(modern_directory_info_plugin PRIVATE
        kernel32
        user32
        advapi32
        shell32
    )
    set_target_properties(modern_directory_info_plugin PROPERTIES
        WINDOWS_EXPORT_ALL_SYMBOLS ON
    )
endif()

# Installation
install(TARGETS modern_system_plugin modern_directory_info_plugin
    LIBRARY DESTINATION lib
    RUNTIME DESTINATION bin
)
