# Modern C++23 CMakeLists.txt for Mini MSP Agent Plugin
# 
# This CMake configuration demonstrates modern CMake practices:
# - C++23 standard with latest features
# - Modern target-based configuration
# - Proper compiler-specific flags
# - Cross-platform support
# - Modern dependency management
# - Export targets with proper visibility

cmake_minimum_required(VERSION 3.25)

# Modern project configuration
project(ModernSystemPlugin
    VERSION 2.0.0
    DESCRIPTION "Modern C++23 System Plugin for Mini MSP Agent"
    LANGUAGES CXX
)

# Set C++23 standard with modern features
set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Enable modern CMake features
set(CMAKE_CXX_SCAN_FOR_MODULES ON)
set(CMAKE_ENABLE_EXPORTS ON)

# Compiler-specific optimizations
if(MSVC)
    # Modern MSVC flags for C++23
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
    # Modern GCC flags for C++23
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
    
    # Modern GCC definitions
    target_compile_definitions(ModernSystemPlugin PRIVATE
        _GNU_SOURCE
    )
    
elseif(CMAKE_CXX_COMPILER_ID STREQUAL "Clang")
    # Modern Clang flags for C++23
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
    
    # Modern Clang definitions
    target_compile_definitions(ModernSystemPlugin PRIVATE
        __STDC_LIBC1__
    )
endif()

# Modern source file organization
set(SOURCES
    src/working_system_plugin.cpp
)

# Create modern library target
add_library(ModernSystemPlugin SHARED ${SOURCES})

# Modern target properties
set_target_properties(ModernSystemPlugin PROPERTIES
    VERSION ${PROJECT_VERSION}
    SOVERSION ${PROJECT_VERSION_MAJOR}
    CXX_STANDARD 23
    CXX_STANDARD_REQUIRED ON
    CXX_EXTENSIONS OFF
    POSITION_INDEPENDENT_CODE ON
    VISIBILITY_INLINES_HIDDEN ON
)

# Modern Windows-specific configuration
if(WIN32)
    # Windows-specific definitions
    target_compile_definitions(ModernSystemPlugin PRIVATE
        WIN32_LEAN_AND_MEAN
        NOMINMAX
        _WIN32_WINNT=0x0A00
        WINVER=0x0A00
        UNICODE
        _UNICODE
        MODERN_SYSTEM_PLUGIN_EXPORTS
    )
    
    # Windows-specific libraries
    target_link_libraries(ModernSystemPlugin PRIVATE
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
    
    # Modern Windows DLL configuration
    set_target_properties(ModernSystemPlugin PROPERTIES
        WINDOWS_EXPORT_ALL_SYMBOLS ON
        LINK_FLAGS "/IGNORE:4221"
    )
    
    # Modern Windows subsystem
    if(MSVC)
        set_target_properties(ModernSystemPlugin PROPERTIES
            LINK_FLAGS "/SUBSYSTEM:WINDOWS"
        )
    endif()
endif()

# Modern Unix-like configuration
if(UNIX)
    # Unix-specific libraries
    target_link_libraries(ModernSystemPlugin PRIVATE
        pthread
        dl
        m
    )
    
    # Unix-specific definitions
    target_compile_definitions(ModernSystemPlugin PRIVATE
        _POSIX_C_SOURCE=200809L
        _XOPEN_SOURCE=700
        MODERN_SYSTEM_PLUGIN_EXPORTS
    )
endif()

# Modern include directories
target_include_directories(ModernSystemPlugin PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    ${CMAKE_CURRENT_SOURCE_DIR}/src
)

# Modern compiler-specific include directories
if(MSVC)
    target_include_directories(ModernSystemPlugin PRIVATE
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/msvc>
    )
else()
    target_include_directories(ModernSystemPlugin PRIVATE
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include/unix>
    )
endif()

# Modern export configuration
include(GenerateExportHeader)

# Generate modern export header
generate_export_header(ModernSystemPlugin
    EXPORT_FILE_NAME "modern_system_plugin_exports.h"
    EXPORT_MACRO_NAME "MODERN_SYSTEM_PLUGIN_API"
    STATIC_DEFINE "MODERN_SYSTEM_PLUGIN_STATIC"
)

# Modern installation configuration
include(GNUInstallDirs)

install(TARGETS ModernSystemPlugin
    EXPORT ModernSystemPluginTargets
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
    INCLUDES DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
)

# Modern header installation
install(FILES
    ${CMAKE_CURRENT_BINARY_DIR}/modern_system_plugin_exports.h
    DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
)

# Modern CMake package configuration (disabled for now - requires cmake/ directory)
# include(CMakePackageConfigHelpers)
# 
# write_basic_package_version_file(
#     "${CMAKE_CURRENT_BINARY_DIR}/ModernSystemPluginConfigVersion.cmake"
#     VERSION ${PROJECT_VERSION}
#     COMPATIBILITY SameMajorVersion
# )
# 
# configure_package_config_file(
#     "${CMAKE_CURRENT_SOURCE_DIR}/cmake/ModernSystemPluginConfig.cmake.in"
#     "${CMAKE_CURRENT_BINARY_DIR}/ModernSystemPluginConfig.cmake"
#     INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/ModernSystemPlugin
# )
# 
# install(FILES
#     "${CMAKE_CURRENT_BINARY_DIR}/ModernSystemPluginConfig.cmake"
#     "${CMAKE_CURRENT_BINARY_DIR}/ModernSystemPluginConfigVersion.cmake"
#     DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/ModernSystemPlugin
# )
# 
# # Modern export configuration
# install(EXPORT ModernSystemPluginTargets
#     FILE ModernSystemPluginTargets.cmake
#     NAMESPACE ModernSystemPlugin::
#     DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/ModernSystemPlugin
# )

# Modern testing configuration
option(BUILD_TESTING "Build tests" OFF)

if(BUILD_TESTING)
    enable_testing()
    
    # Modern test executable
    add_executable(ModernSystemPluginTest
        tests/test_modern_plugin.cpp
    )
    
    target_link_libraries(ModernSystemPluginTest PRIVATE
        ModernSystemPlugin
    )
    
    add_test(NAME ModernSystemPluginTest COMMAND ModernSystemPluginTest)
endif()

# Modern documentation configuration
option(BUILD_DOCS "Build documentation" OFF)

if(BUILD_DOCS)
    find_package(Doxygen QUIET)
    if(DOXYGEN_FOUND)
        # Modern Doxygen configuration
        set(DOXYGEN_INPUT_DIR ${CMAKE_CURRENT_SOURCE_DIR}/src)
        set(DOXYGEN_OUTPUT_DIR ${CMAKE_CURRENT_BINARY_DIR}/docs)
        set(DOXYGEN_PROJECT_NAME "Modern System Plugin")
        set(DOXYGEN_PROJECT_NUMBER ${PROJECT_VERSION})
        
        doxygen_add_docs(ModernSystemPlugin
            WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
            COMMENT "Generate modern plugin documentation"
        )
    endif()
endif()

# Modern benchmarking configuration
option(BUILD_BENCHMARKS "Build benchmarks" OFF)

if(BUILD_BENCHMARKS)
    # Modern benchmark executable
    add_executable(ModernSystemPluginBenchmark
        benchmarks/benchmark_modern_plugin.cpp
    )
    
    target_link_libraries(ModernSystemPluginBenchmark PRIVATE
        ModernSystemPlugin
    )
endif()

# Modern code analysis
option(ENABLE_CODE_ANALYSIS "Enable static code analysis" OFF)

if(ENABLE_CODE_ANALYSIS AND MSVC)
    # Enable MSVC code analysis
    add_compile_options(/analyze)
    set_target_properties(ModernSystemPlugin PROPERTIES
        VS_DEBUGGER_WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    )
endif()

# Modern sanitizers
option(ENABLE_SANITIZERS "Enable address and undefined behavior sanitizers" OFF)

if(ENABLE_SANITIZERS AND NOT MSVC)
    # Enable GCC/Clang sanitizers
    add_compile_options(
        -fsanitize=address
        -fsanitize=undefined
        -fno-omit-frame-pointer
    )
    
    target_link_options(ModernSystemPlugin PRIVATE
        -fsanitize=address
        -fsanitize=undefined
    )
endif()

# Modern profiling
option(ENABLE_PROFILING "Enable profiling support" OFF)

if(ENABLE_PROFILING)
    if(MSVC)
        add_compile_options(/PROFILE)
        target_link_options(ModernSystemPlugin PRIVATE /PROFILE)
    else()
        add_compile_options(-pg)
        target_link_options(ModernSystemPlugin PRIVATE -pg)
    endif()
endif()

# Print modern configuration summary
message(STATUS "")
message(STATUS "Modern System Plugin Configuration Summary:")
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
