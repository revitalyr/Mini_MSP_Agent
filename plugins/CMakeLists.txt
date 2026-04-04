# Modern C++23 Plugin System for Mini MSP Agent
# 
# This CMake configuration builds modern C++23 plugins with:
# - Latest C++23 features (concepts, ranges, coroutines)
# - Modern CMake practices (target-based configuration)
# - Cross-platform support (Windows, Linux, macOS)
# - Advanced compiler optimizations
# - Proper dependency management
# - Export configuration for dynamic loading

cmake_minimum_required(VERSION 3.25)

# Modern project setup
project(MiniMSPAgentPlugins
    VERSION 3.0.0
    DESCRIPTION "Modern C++23 Plugin System for Mini MSP Agent"
    LANGUAGES CXX
)

# Set C++23 standard with modern features
set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Enable modern CMake features
set(CMAKE_CXX_SCAN_FOR_MODULES ON)
set(CMAKE_ENABLE_EXPORTS ON)

# Compiler-specific modern flags
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
        /EHsc
    )
    
    # Modern MSVC definitions
    add_compile_definitions(
        WIN32_LEAN_AND_MEAN
        NOMINMAX
        _WIN32_WINNT=0x0A00
        WINVER=0x0A00
        UNICODE
        _UNICODE
    )
    
elseif(CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
    # Modern GCC flags for C++23
    add_compile_options(
        -std=c++23
        -fconcepts
        -fcoroutines
        -Wall
        -Wextra
        -Wpedantic
        -Werror
        -O3
        -march=native
    )
    
    # Modern GCC definitions
    add_compile_definitions(
        _GNU_SOURCE
    )
    
elseif(CMAKE_CXX_COMPILER_ID STREQUAL "Clang")
    # Modern Clang flags for C++23
    add_compile_options(
        -std=c++23
        -fconcepts
        -fcoroutines
        -Wall
        -Wextra
        -Wpedantic
        -Werror
        -O3
        -march=native
    )
    
    # Modern Clang definitions
    add_compile_definitions(
        __STDC_LIBC1__
    )
endif()

# Modern plugin sources
set(MODERN_PLUGIN_SOURCES
    src/modern_system_plugin_v3.cpp
    src/modern_directory_info_plugin.cpp
)

# Create modern system plugin library
add_library(modern_system_plugin SHARED src/modern_system_plugin_v3.cpp)

# Modern target properties
set_target_properties(modern_system_plugin PROPERTIES
    VERSION ${PROJECT_VERSION}
    SOVERSION ${PROJECT_VERSION_MAJOR}
    CXX_STANDARD 23
    CXX_STANDARD_REQUIRED ON
    CXX_EXTENSIONS OFF
    POSITION_INDEPENDENT_CODE ON
    VISIBILITY_INLINES_HIDDEN ON
)

# Windows-specific modern configuration
if(WIN32)
    # Modern Windows libraries
    target_link_libraries(modern_system_plugin PRIVATE
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
    set_target_properties(modern_system_plugin PROPERTIES
        WINDOWS_EXPORT_ALL_SYMBOLS ON
        LINK_FLAGS "/IGNORE:4221"
    )
    
    # Windows subsystem
    if(MSVC)
        set_target_properties(modern_system_plugin PROPERTIES
            LINK_FLAGS "/SUBSYSTEM:WINDOWS"
        )
    endif()
endif()

# Unix-like configuration
if(UNIX)
    # Modern Unix libraries
    target_link_libraries(modern_system_plugin PRIVATE
        pthread
        dl
        m
    )
    
    # Unix definitions
    target_compile_definitions(modern_system_plugin PRIVATE
        _POSIX_C_SOURCE=200809L
        _XOPEN_SOURCE=700
    )
endif()

# Create modern directory info plugin library
add_library(modern_directory_info_plugin SHARED src/modern_directory_info_plugin.cpp)

# Modern target properties
set_target_properties(modern_directory_info_plugin PROPERTIES
    VERSION ${PROJECT_VERSION}
    SOVERSION ${PROJECT_VERSION_MAJOR}
    CXX_STANDARD 23
    CXX_STANDARD_REQUIRED ON
    CXX_EXTENSIONS OFF
    POSITION_INDEPENDENT_CODE ON
    VISIBILITY_INLINES_HIDDEN ON
)

# Windows-specific configuration for directory plugin
if(WIN32)
    target_link_libraries(modern_directory_info_plugin PRIVATE
        kernel32
        user32
        advapi32
        shell32
    )
    
    set_target_properties(modern_directory_info_plugin PROPERTIES
        WINDOWS_EXPORT_ALL_SYMBOLS ON
        LINK_FLAGS "/IGNORE:4221"
    )
endif()

# Unix-like configuration for directory plugin
if(UNIX)
    target_link_libraries(modern_directory_info_plugin PRIVATE
        pthread
        dl
    )
    
    target_compile_definitions(modern_directory_info_plugin PRIVATE
        _POSIX_C_SOURCE=200809L
        _XOPEN_SOURCE=700
    )
endif()

# Modern include directories
target_include_directories(modern_system_plugin PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/src
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

target_include_directories(modern_directory_info_plugin PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/src
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

# Modern installation
include(GNUInstallDirs)

# Install modern system plugin
install(TARGETS modern_system_plugin
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
)

# Install modern directory plugin
install(TARGETS modern_directory_info_plugin
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
)

# Create plugin configuration file
configure_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/plugins.json.in
    ${CMAKE_CURRENT_BINARY_DIR}/plugins.json
    @ONLY
)

# Install plugin configuration
install(FILES
    ${CMAKE_CURRENT_BINARY_DIR}/plugins.json
    DESTINATION ${CMAKE_INSTALL_SYSCONFDIR}
)

# Modern testing configuration
option(BUILD_PLUGIN_TESTS "Build plugin tests" OFF)

if(BUILD_PLUGIN_TESTS)
    enable_testing()
    
    # Modern test executable for system plugin
    add_executable(modern_system_plugin_test
        tests/test_modern_system_plugin.cpp
    )
    
    target_link_libraries(modern_system_plugin_test PRIVATE
        modern_system_plugin
        gtest_main
    )
    
    add_test(NAME ModernSystemPluginTest COMMAND modern_system_plugin_test)
    
    # Modern test executable for directory plugin
    add_executable(modern_directory_info_plugin_test
        tests/test_modern_directory_info_plugin.cpp
    )
    
    target_link_libraries(modern_directory_info_plugin_test PRIVATE
        modern_directory_info_plugin
        gtest_main
    )
    
    add_test(NAME ModernDirectoryInfoPluginTest COMMAND modern_directory_info_plugin_test)
endif()

# Modern benchmarking configuration
option(BUILD_PLUGIN_BENCHMARKS "Build plugin benchmarks" OFF)

if(BUILD_PLUGIN_BENCHMARKS)
    # Modern benchmark executable for system plugin
    add_executable(modern_system_plugin_benchmark
        benchmarks/benchmark_modern_system_plugin.cpp
    )
    
    target_link_libraries(modern_system_plugin_benchmark PRIVATE
        modern_system_plugin
        benchmark
    )
    
    # Modern benchmark executable for directory plugin
    add_executable(modern_directory_info_plugin_benchmark
        benchmarks/benchmark_modern_directory_info_plugin.cpp
    )
    
    target_link_libraries(modern_directory_info_plugin_benchmark PRIVATE
        modern_directory_info_plugin
        benchmark
    )
endif()

# Modern code analysis
option(ENABLE_PLUGIN_CODE_ANALYSIS "Enable static code analysis for plugins" OFF)

if(ENABLE_PLUGIN_CODE_ANALYSIS AND MSVC)
    # Enable MSVC code analysis
    add_compile_options(/analyze)
    set_target_properties(modern_system_plugin PROPERTIES
        VS_DEBUGGER_WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    )
    set_target_properties(modern_directory_info_plugin PROPERTIES
        VS_DEBUGGER_WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    )
endif()

# Modern sanitizers
option(ENABLE_PLUGIN_SANITIZERS "Enable address and undefined behavior sanitizers for plugins" OFF)

if(ENABLE_PLUGIN_SANITIZERS AND NOT MSVC)
    # Enable GCC/Clang sanitizers
    add_compile_options(
        -fsanitize=address
        -fsanitize=undefined
        -fno-omit-frame-pointer
    )
    
    target_link_options(modern_system_plugin PRIVATE
        -fsanitize=address
        -fsanitize=undefined
    )
    
    target_link_options(modern_directory_info_plugin PRIVATE
        -fsanitize=address
        -fsanitize=undefined
    )
endif()

# Modern profiling
option(ENABLE_PLUGIN_PROFILING "Enable profiling support for plugins" OFF)

if(ENABLE_PLUGIN_PROFILING)
    if(MSVC)
        add_compile_options(/PROFILE)
        target_link_options(modern_system_plugin PRIVATE /PROFILE)
        target_link_options(modern_directory_info_plugin PRIVATE /PROFILE)
    else()
        add_compile_options(-pg)
        target_link_options(modern_system_plugin PRIVATE -pg)
        target_link_options(modern_directory_info_plugin PRIVATE -pg)
    endif()
endif()

# Print modern configuration summary
message(STATUS "")
message(STATUS "Modern Plugin System Configuration:")
message(STATUS "  Version: ${PROJECT_VERSION}")
message(STATUS "  C++ Standard: ${CMAKE_CXX_STANDARD}")
message(STATUS "  Build Type: ${CMAKE_BUILD_TYPE}")
message(STATUS "  Compiler: ${CMAKE_CXX_COMPILER_ID}")
message(STATUS "  Platform: ${CMAKE_SYSTEM_NAME}")
message(STATUS "  Architecture: ${CMAKE_SYSTEM_PROCESSOR}")
message(STATUS "  Install Prefix: ${CMAKE_INSTALL_PREFIX}")
message(STATUS "  Testing: ${BUILD_PLUGIN_TESTS}")
message(STATUS "  Benchmarks: ${BUILD_PLUGIN_BENCHMARKS}")
message(STATUS "  Code Analysis: ${ENABLE_PLUGIN_CODE_ANALYSIS}")
message(STATUS "  Sanitizers: ${ENABLE_PLUGIN_SANITIZERS}")
message(STATUS "  Profiling: ${ENABLE_PLUGIN_PROFILING}")
message(STATUS "")

# Plugin list for configuration
set(PLUGIN_LIST
    "modern_system_plugin"
    "modern_directory_info_plugin"
)

# Create plugin configuration
file(WRITE ${CMAKE_CURRENT_BINARY_DIR}/plugins.json.in "")
foreach(PLUGIN ${PLUGIN_LIST})
    file(APPEND ${CMAKE_CURRENT_BINARY_DIR}/plugins.json.in "  \"${PLUGIN}\",\n")
endforeach()
# ------------------------------

# Detect platform
if(WIN32)
    set(PLATFORM "windows")
elseif(APPLE)
    set(PLATFORM "macos")
else()
    set(PLATFORM "linux")
endif()

message(STATUS "Building plugins for platform: ${PLATFORM}")

# Set output directory for plugins
set(PLUGIN_OUTPUT_DIR ${CMAKE_CURRENT_BINARY_DIR}/plugins)

# Create plugin output directory
file(MAKE_DIRECTORY ${PLUGIN_OUTPUT_DIR})

# System plugin (platform-specific C implementation)
add_library(system_plugin SHARED
    src/${PLATFORM}/system_plugin_${PLATFORM}.c
)

target_include_directories(system_plugin PUBLIC 
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

# Set compiler flags for Windows
if(WIN32 AND MSVC)
    target_compile_options(system_plugin PRIVATE /W3 /utf-8)
endif()

# Add signature generation for system_plugin
add_custom_command(TARGET system_plugin POST_BUILD
    COMMAND ${OPENSSL_EXECUTABLE} dgst -sha256 -sign ${PLUGIN_PRIVATE_KEY} 
            -out "$<TARGET_FILE:system_plugin>.sig" 
            "$<TARGET_FILE:system_plugin>"
    WORKING_DIRECTORY ${PLUGIN_OUTPUT_DIR}
    COMMENT "🛡️ Generating digital signature for system_plugin"
    VERBATIM
)

set_target_properties(system_plugin PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
    ARCHIVE_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
)

# Platform-specific sources
set(PLATFORM_SOURCES)
set(PLATFORM_LIBS)
set(PLATFORM_DEFINES)

if(WIN32)
    list(APPEND PLATFORM_SOURCES
        ${CMAKE_CURRENT_SOURCE_DIR}/platform/windows/file_reader_plugin_windows.c
        ${CMAKE_CURRENT_SOURCE_DIR}/platform/windows/watchers_manager_windows.c
        ${CMAKE_CURRENT_SOURCE_DIR}/platform/windows/plugin_loader_windows.c
    )
    list(APPEND PLATFORM_LIBS kernel32 user32)
    list(APPEND PLATFORM_DEFINES _WIN32)
elseif(APPLE)
    list(APPEND PLATFORM_SOURCES
        ${CMAKE_CURRENT_SOURCE_DIR}/platform/macos/file_reader_plugin_macos.c
        ${CMAKE_CURRENT_SOURCE_DIR}/platform/macos/watchers_manager_macos.c
        ${CMAKE_CURRENT_SOURCE_DIR}/platform/macos/plugin_loader_macos.c
    )
    list(APPEND PLATFORM_LIBS pthread)
    list(APPEND PLATFORM_DEFINES __APPLE__)
else()
    list(APPEND PLATFORM_SOURCES
        ${CMAKE_CURRENT_SOURCE_DIR}/platform/linux/file_reader_plugin_linux.c
        ${CMAKE_CURRENT_SOURCE_DIR}/platform/linux/watchers_manager_linux.c
        ${CMAKE_CURRENT_SOURCE_DIR}/platform/linux/plugin_loader_linux.c
    )
    list(APPEND PLATFORM_LIBS pthread)
endif()

# Common sources (shared across all platforms)
set(COMMON_SOURCES
    ${CMAKE_CURRENT_SOURCE_DIR}/common/plugin_utils.c
    ${CMAKE_CURRENT_SOURCE_DIR}/common/safe_functions.c
)

# Plugin sources
set(PLUGIN_SOURCES
    ${CMAKE_CURRENT_SOURCE_DIR}/src/directory_info_plugin.c
    ${CMAKE_CURRENT_SOURCE_DIR}/src/event_data_plugin.c
    ${CMAKE_CURRENT_SOURCE_DIR}/src/file_reader_plugin.c
    ${CMAKE_CURRENT_SOURCE_DIR}/src/file_signature_plugin.c
    ${CMAKE_CURRENT_SOURCE_DIR}/src/folder_watcher_plugin.c
    ${CMAKE_CURRENT_SOURCE_DIR}/src/file_listener_plugin.c
    ${CMAKE_CURRENT_SOURCE_DIR}/src/root_directory_info_plugin.c
    ${CMAKE_CURRENT_SOURCE_DIR}/src/scanner_plugin.c
    ${CMAKE_CURRENT_SOURCE_DIR}/src/watchers_manager_plugin.c
    ${CMAKE_CURRENT_SOURCE_DIR}/src/plugin_registry.c
)

# Create shared library for each plugin
foreach(PLUGIN_SOURCE ${PLUGIN_SOURCES})
    get_filename_component(PLUGIN_NAME ${PLUGIN_SOURCE} NAME_WE)
    
    add_library(${PLUGIN_NAME} SHARED
        ${PLUGIN_SOURCE}
        ${COMMON_SOURCES}
        ${PLATFORM_SOURCES}
    )
    
    # Add signature generation for each plugin
    add_custom_command(TARGET ${PLUGIN_NAME} POST_BUILD
        COMMAND ${OPENSSL_EXECUTABLE} dgst -sha256 -sign ${PLUGIN_PRIVATE_KEY} 
                -out "$<TARGET_FILE:${PLUGIN_NAME}>.sig" 
                "$<TARGET_FILE:${PLUGIN_NAME}>"
        WORKING_DIRECTORY ${PLUGIN_OUTPUT_DIR}
        COMMENT "🛡️ Generating digital signature for ${PLUGIN_NAME}"
        VERBATIM
    )

    target_include_directories(${PLUGIN_NAME} PUBLIC 
        ${CMAKE_CURRENT_SOURCE_DIR}/include
    )
    
    # Set compiler flags for Windows
    if(WIN32 AND MSVC)
        target_compile_options(${PLUGIN_NAME} PRIVATE /W3 /utf-8)
    endif()
    
    target_compile_definitions(${PLUGIN_NAME} PRIVATE ${PLATFORM_DEFINES})
    target_link_libraries(${PLUGIN_NAME} PRIVATE ${PLATFORM_LIBS})
    
    # Set output name
    set_target_properties(${PLUGIN_NAME} PROPERTIES
        OUTPUT_NAME "${PLUGIN_NAME}"
        PREFIX ""
    )
    
    # Copy to agent directory
    if(OPENSSL_FOUND)
        target_link_libraries(${PLUGIN_NAME} ${OPENSSL_LIBRARIES})
        target_include_directories(${PLUGIN_NAME} PUBLIC ${OPENSSL_INCLUDE_DIRS})
    endif()
    
    message(STATUS "Added plugin: ${PLUGIN_NAME}")
endforeach()

# Custom target to copy plugins to agent directory
add_custom_target(copy_all_plugins ALL
    COMMAND ${CMAKE_COMMAND} -E make_directory ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins
    # Copy libraries
    COMMAND ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/system_plugin.dll ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/ || ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/libsystem_plugin.so ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/ || ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/libsystem_plugin.dylib ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/
    # Copy signatures
    COMMAND ${CMAKE_COMMAND} -E copy_if_different 
        ${PLUGIN_OUTPUT_DIR}/system_plugin.dll.sig 
        ${PLUGIN_OUTPUT_DIR}/libsystem_plugin.so.sig 
        ${PLUGIN_OUTPUT_DIR}/libsystem_plugin.dylib.sig 
        ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/ || ${CMAKE_COMMAND} -E echo "Signature for system_plugin not found, skipping..."
    
    DEPENDS system_plugin
    COMMENT "Copying system plugin to agent directory"
)

# Copy all new plugins
foreach(PLUGIN_SOURCE ${PLUGIN_SOURCES})
    get_filename_component(PLUGIN_NAME ${PLUGIN_SOURCE} NAME_WE)
    add_custom_command(TARGET ${PLUGIN_NAME} POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
        $<TARGET_FILE:${PLUGIN_NAME}>
        "$<TARGET_FILE:${PLUGIN_NAME}>.sig"
        ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/
        COMMENT "Copying ${PLUGIN_NAME} and signature to agent directory"
    )
endforeach()

# Make copy_plugins run after building all plugins
add_dependencies(copy_all_plugins system_plugin)
foreach(PLUGIN_SOURCE ${PLUGIN_SOURCES})
    get_filename_component(PLUGIN_NAME ${PLUGIN_SOURCE} NAME_WE)
    add_dependencies(copy_all_plugins ${PLUGIN_NAME})
endforeach()

# Compiler-specific optimizations for C
if(CMAKE_C_COMPILER_ID STREQUAL "GNU" OR CMAKE_C_COMPILER_ID STREQUAL "Clang")
    target_compile_options(system_plugin PRIVATE
        -Wall
        -Wextra
        -O2
        -fPIC
    )
    foreach(PLUGIN_SOURCE ${PLUGIN_SOURCES})
        get_filename_component(PLUGIN_NAME ${PLUGIN_SOURCE} NAME_WE)
        target_compile_options(${PLUGIN_NAME} PRIVATE
            -Wall
            -Wextra
            -O2
            -fPIC
        )
    endforeach()
elseif(CMAKE_C_COMPILER_ID STREQUAL "MSVC")
    target_compile_options(system_plugin PRIVATE
        /W4
        /O2
        /MD
    )
    foreach(PLUGIN_SOURCE ${PLUGIN_SOURCES})
        get_filename_component(PLUGIN_NAME ${PLUGIN_SOURCE} NAME_WE)
        target_compile_options(${PLUGIN_NAME} PRIVATE
            /W4
            /O2
            /MD
        )
    endforeach()
endif()

# Install targets
install(TARGETS system_plugin
    LIBRARY DESTINATION plugins
    ARCHIVE DESTINATION plugins
    RUNTIME DESTINATION plugins
)

foreach(PLUGIN_SOURCE ${PLUGIN_SOURCES})
    get_filename_component(PLUGIN_NAME ${PLUGIN_SOURCE} NAME_WE)
    install(TARGETS ${PLUGIN_NAME}
        LIBRARY DESTINATION plugins
        ARCHIVE DESTINATION plugins
        RUNTIME DESTINATION plugins
    )
endforeach()

# Print plugin information
message(STATUS "Plugin configuration:")
message(STATUS "  Platform: ${PLATFORM}")
message(STATUS "  Output directory: ${PLUGIN_OUTPUT_DIR}")
message(STATUS "  System plugin: src/${PLATFORM}/system_plugin_${PLATFORM}.c")
message(STATUS "  New plugins: ${PLUGIN_SOURCES}")
message(STATUS "")
