# C++23 CMakeLists.txt for Mini MSP Agent Plugin
# 
# This CMake configuration demonstrates CMake practices:
# - C++23 standard with latest features
# - Target-based configuration
# - Proper compiler-specific flags
# - Cross-platform support
# - Dependency management
# - Export targets with proper visibility

cmake_minimum_required(VERSION 3.25)

# Project configuration
project(SystemPluginV3
    VERSION 2.0.0
    DESCRIPTION "C++23 System Plugin for Mini MSP Agent"
    LANGUAGES CXX
)

# Set C++23 standard with features
set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Enable CMake features
# Note: CMAKE_CXX_SCAN_FOR_MODULES disabled due to AppleClang compatibility
set(CMAKE_ENABLE_EXPORTS ON)

# Compiler-specific optimizations
if(MSVC)
    #  MSVC flags for C++23
    add_compile_options(
        /std:c++latest
        /permissive-
        /Zc:__cplusplus
        /Zc:preprocessor
        /Zc:externConstexpr
        /Zc:lambda
        /Zc:throwingNew
        /Zc:referenceBinding
        /Zc:rvalueCast
        /Zc:twoPhaseLookup
        /Zc:threadSafeInit
        /W4
        /WX
        /utf-8
        /bigobj
    )
    
elseif(CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
    #  GCC flags for C++23
    add_compile_options(
        -std=c++23
        -fmodules-ts
        -fconcepts-ts
        -fcoroutines-ts
        -Wall
        -Wextra
        -Wpedantic
        -Werror
        -fno-rtti
        -fanalyzer
        -fno-exceptions
        -O3
        -march=native
    )
    
    #  GCC definitions
    target_compile_definitions(SystemPluginV3 PRIVATE
        _GNU_SOURCE
    )
    
elseif(CMAKE_CXX_COMPILER_ID STREQUAL "Clang")
    #  Clang flags for C++23
    add_compile_options(
        -std=c++23
        -fmodules
        -fconcepts-ts
        -fcoroutines-ts
        -Wall
        -Wextra
        -Wpedantic
        -Werror
        -O3
        -march=native
    )
    
    #  Clang definitions
    target_compile_definitions(SystemPluginV3 PRIVATE
        __STDC_LIBC1__
    )
endif()

#  source file organization - Base plugin
set(SOURCES
    src/system_plugin_v3.cpp
)

# Create library target
add_library(SystemPluginV3 SHARED ${SOURCES})

# Platform-specific Forensic Plugin
# This plugin provides real forensic artifact collection for each OS
if(WIN32)
    # Windows Forensic Plugin
    set(FORENSIC_SOURCES
        src/windows/forensic_plugin.cpp
    )
    add_library(ForensicPlugin SHARED ${FORENSIC_SOURCES})
    set_target_properties(ForensicPlugin PROPERTIES
        VERSION ${PROJECT_VERSION}
        SOVERSION ${PROJECT_VERSION_MAJOR}
        CXX_STANDARD 23
        CXX_STANDARD_REQUIRED ON
        CXX_EXTENSIONS OFF
        POSITION_INDEPENDENT_CODE ON
    )
    target_include_directories(ForensicPlugin PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/include
    )
    target_compile_definitions(ForensicPlugin PRIVATE
        WIN32_LEAN_AND_MEAN
        NOMINMAX
        _WIN32_WINNT=0x0A00
        UNICODE
        _UNICODE
    )
    target_link_libraries(ForensicPlugin PRIVATE
        kernel32
        user32
        psapi
        advapi32
        shell32
        ole32
        oleaut32
        uuid
        comdlg32
        gdi32
        winspool
    )
    
elseif(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    # Linux Forensic Plugin
    set(FORENSIC_SOURCES
        src/linux/forensic_plugin.cpp
    )
    add_library(ForensicPlugin SHARED ${FORENSIC_SOURCES})
    set_target_properties(ForensicPlugin PROPERTIES
        VERSION ${PROJECT_VERSION}
        SOVERSION ${PROJECT_VERSION_MAJOR}
        CXX_STANDARD 23
        CXX_STANDARD_REQUIRED ON
        CXX_EXTENSIONS OFF
        POSITION_INDEPENDENT_CODE ON
    )
    target_include_directories(ForensicPlugin PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/include
    )
    target_compile_definitions(ForensicPlugin PRIVATE
        _POSIX_C_SOURCE=200809L
        _XOPEN_SOURCE=700
        _GNU_SOURCE
    )
    target_link_libraries(ForensicPlugin PRIVATE
        pthread
        dl
        m
    )
    
elseif(CMAKE_SYSTEM_NAME STREQUAL "Darwin")
    # macOS Forensic Plugin
    set(FORENSIC_SOURCES
        src/macos/forensic_plugin.cpp
    )
    add_library(ForensicPlugin SHARED ${FORENSIC_SOURCES})
    set_target_properties(ForensicPlugin PROPERTIES
        VERSION ${PROJECT_VERSION}
        SOVERSION ${PROJECT_VERSION_MAJOR}
        CXX_STANDARD 23
        CXX_STANDARD_REQUIRED ON
        CXX_EXTENSIONS OFF
        POSITION_INDEPENDENT_CODE ON
    )
    target_include_directories(ForensicPlugin PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/include
    )
    find_library(FOUNDATION_LIBRARY Foundation)
    find_library(IOKIT_LIBRARY IOKit)
    target_link_libraries(ForensicPlugin PRIVATE
        ${FOUNDATION_LIBRARY}
        ${IOKIT_LIBRARY}
        pthread
        dl
        m
    )
endif()

# Custom Plugin - Example for extensible functionality
# This plugin demonstrates how to create custom commands and metrics
set(CUSTOM_PLUGIN_SOURCES
    src/custom_plugin.cpp
)
add_library(CustomPlugin SHARED ${CUSTOM_PLUGIN_SOURCES})
set_target_properties(CustomPlugin PROPERTIES
    VERSION ${PROJECT_VERSION}
    SOVERSION ${PROJECT_VERSION_MAJOR}
    CXX_STANDARD 23
    CXX_STANDARD_REQUIRED ON
    CXX_EXTENSIONS OFF
    POSITION_INDEPENDENT_CODE ON
)
target_include_directories(CustomPlugin PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

# Generate export header for forensic plugin if it exists
if(TARGET ForensicPlugin)
    generate_export_header(ForensicPlugin
        EXPORT_FILE_NAME "forensic_plugin_exports.h"
        EXPORT_MACRO_NAME "FORENSIC_PLUGIN_API"
        STATIC_DEFINE "FORENSIC_PLUGIN_STATIC"
    )
endif()

# Target properties
set_target_properties(SystemPluginV3 PROPERTIES
    VERSION ${PROJECT_VERSION}
    SOVERSION ${PROJECT_VERSION_MAJOR}
    CXX_STANDARD 23
    CXX_STANDARD_REQUIRED ON
    CXX_EXTENSIONS OFF
    POSITION_INDEPENDENT_CODE ON
    VISIBILITY_INLINES_HIDDEN ON
)

#  Windows-specific configuration
if(WIN32)
    # Windows-specific definitions
    target_compile_definitions(SystemPluginV3 PRIVATE
        WIN32_LEAN_AND_MEAN
        NOMINMAX
        _WIN32_WINNT=0x0A00
        WINVER=0x0A00
        UNICODE
        _UNICODE
        SYSTEM_PLUGIN_V3_EXPORTS
    )
    
    # Windows-specific libraries
    target_link_libraries(SystemPluginV3 PRIVATE
        kernel32
        user32
        psapi
        advapi32
        shell32
        ole32
        oleaut32
        uuid
        comdlg32
        gdi32
        winspool
    )
    
    #  Windows DLL configuration
    set_target_properties(SystemPluginV3 PROPERTIES
        WINDOWS_EXPORT_ALL_SYMBOLS ON
        LINK_FLAGS "/IGNORE:4221"
    )
    
    #  Windows subsystem
    if(MSVC)
        set_target_properties(SystemPluginV3 PROPERTIES
            LINK_FLAGS "/SUBSYSTEM:WINDOWS"
        )
    endif()
endif()

#  Unix-like configuration
if(UNIX)
    # Unix-specific libraries
    target_link_libraries(SystemPluginV3 PRIVATE
        pthread
        dl
        m
    )
    
    # Unix-specific definitions
    target_compile_definitions(SystemPluginV3 PRIVATE
        _POSIX_C_SOURCE=200809L
        _XOPEN_SOURCE=700
        SYSTEM_PLUGIN_V3_EXPORTS
    )
endif()

# Include directories
target_include_directories(SystemPluginV3 PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    ${CMAKE_CURRENT_SOURCE_DIR}/src
)

# Compiler-specific include directories
if(MSVC)
    target_include_directories(SystemPluginV3 PRIVATE
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/msvc>
    )
else()
    target_include_directories(SystemPluginV3 PRIVATE
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/unix>
    )
endif()

#  export configuration
include(GenerateExportHeader)

# Generate export header
generate_export_header(SystemPluginV3
    EXPORT_FILE_NAME "system_plugin_v3_exports.h"
    EXPORT_MACRO_NAME "SYSTEM_PLUGIN_V3_API"
    STATIC_DEFINE "SYSTEM_PLUGIN_V3_STATIC"
)

# Installation configuration
include(GNUInstallDirs)

install(TARGETS SystemPluginV3
    EXPORT SystemPluginV3Targets
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
    INCLUDES DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
)

# Header installation
install(FILES
    ${CMAKE_CURRENT_BINARY_DIR}/system_plugin_v3_exports.h
    DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
)

#  CMake package configuration (disabled for now - requires cmake/ directory)
# include(CMakePackageConfigHelpers)
# 
# write_basic_package_version_file(
#     "${CMAKE_CURRENT_BINARY_DIR}/SystemPluginConfigVersion.cmake"
#     VERSION ${PROJECT_VERSION}
#     COMPATIBILITY SameMajorVersion
# )
# 
# configure_package_config_file(
#     "${CMAKE_CURRENT_SOURCE_DIR}/cmake/SystemPluginConfig.cmake.in"
#     "${CMAKE_CURRENT_BINARY_DIR}/SystemPluginConfig.cmake"
#     INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/SystemPlugin
# )
# 
# install(FILES
#     "${CMAKE_CURRENT_BINARY_DIR}/SystemPluginConfig.cmake"
#     "${CMAKE_CURRENT_BINARY_DIR}/SystemPluginConfigVersion.cmake"
#     DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/SystemPlugin
# )
# 
# #  export configuration
# install(EXPORT SystemPluginTargets
#     FILE SystemPluginTargets.cmake
#     NAMESPACE SystemPlugin::
#     DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/SystemPlugin
# )

#  testing configuration
option(BUILD_TESTING "Build tests" OFF)

if(BUILD_TESTING)
    enable_testing()
    
    #  test executable
    add_executable(SystemPluginTest
        tests/test_system_plugin_v3.cpp
    )
    
    target_link_libraries(SystemPluginTest PRIVATE
        SystemPluginV3
    )
    
    add_test(NAME SystemPluginTest COMMAND SystemPluginTest)
endif()

#  documentation configuration
option(BUILD_DOCS "Build documentation" OFF)

if(BUILD_DOCS)
    find_package(Doxygen QUIET)
    if(DOXYGEN_FOUND)
        #  Doxygen configuration
        set(DOXYGEN_INPUT_DIR ${CMAKE_CURRENT_SOURCE_DIR}/src)
        set(DOXYGEN_OUTPUT_DIR ${CMAKE_CURRENT_BINARY_DIR}/docs)
        set(DOXYGEN_PROJECT_NAME " System Plugin")
        set(DOXYGEN_PROJECT_NUMBER ${PROJECT_VERSION})
        
        doxygen_add_docs(SystemPluginV3
            WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
            COMMENT "Generate plugin documentation"
        )
    endif()
endif()

#  benchmarking configuration
option(BUILD_BENCHMARKS "Build benchmarks" OFF)

if(BUILD_BENCHMARKS)
    #  benchmark executable
    add_executable(SystemPluginBenchmark
        benchmarks/benchmark_system_plugin_v3.cpp
    )
    
    target_link_libraries(SystemPluginBenchmark PRIVATE
        SystemPluginV3
    )
endif()

#  code analysis
option(ENABLE_CODE_ANALYSIS "Enable static code analysis" OFF)

if(ENABLE_CODE_ANALYSIS AND MSVC)
    # Enable MSVC code analysis
    add_compile_options(/analyze)
    set_target_properties(SystemPluginV3 PROPERTIES
        VS_DEBUGGER_WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    )
endif()

#  sanitizers
option(ENABLE_SANITIZERS "Enable address and undefined behavior sanitizers" OFF)

if(ENABLE_SANITIZERS AND NOT MSVC)
    # Enable GCC/Clang sanitizers
    add_compile_options(
        -fsanitize=address
        -fsanitize=undefined
        -fno-omit-frame-pointer
    )
    
    target_link_options(SystemPluginV3 PRIVATE
        -fsanitize=address
        -fsanitize=undefined
    )
endif()

#  profiling
option(ENABLE_PROFILING "Enable profiling support" OFF)

if(ENABLE_PROFILING)
    if(MSVC)
        add_compile_options(/PROFILE)
        target_link_options(SystemPluginV3 PRIVATE /PROFILE)
    else()
        add_compile_options(-pg)
        target_link_options(SystemPluginV3 PRIVATE -pg)
    endif()
endif()

# Print configuration summary
message(STATUS "")
message(STATUS " System Plugin Configuration Summary:")
message(STATUS "  Version: ${PROJECT_VERSION}")
message(STATUS "  C++ Standard: ${CMAKE_CXX_STANDARD}")
message(STATUS "  Build Type: ${CMAKE_BUILD_TYPE}")
message(STATUS "  Compiler: ${CMAKE_CXX_COMPILER_ID}")
message(STATUS "  Platform: ${CMAKE_SYSTEM_NAME}")
message(STATUS "  Architecture: ${CMAKE_SYSTEM_PROCESSOR}")
message(STATUS "  Install Prefix: ${CMAKE_INSTALL_PREFIX}")
message(STATUS "  Testing: ${BUILD_TESTING}")
message(STATUS "  Documentation: ${BUILD_DOCS}")
message(STATUS "  Benchmarks: ${BUILD_BENCHMARKS}")
message(STATUS "  Code Analysis: ${ENABLE_CODE_ANALYSIS}")
message(STATUS "  Sanitizers: ${ENABLE_SANITIZERS}")
message(STATUS "  Profiling: ${ENABLE_PROFILING}")
message(STATUS "")
