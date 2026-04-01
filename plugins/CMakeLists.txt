cmake_minimum_required(VERSION 3.12)
project(mini_msp_plugins)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Plugin configuration
set(PLUGIN_OUTPUT_DIR ${CMAKE_BINARY_DIR}/plugins)

# Detect platform
if(WIN32)
    set(PLATFORM "windows")
elseif(APPLE)
    set(PLATFORM "macos")
else()
    set(PLATFORM "unix")
endif()

message(STATUS "Building plugins for platform: ${PLATFORM}")

# System plugin for the detected platform
add_library(system_plugin SHARED
    src/${PLATFORM}/system_plugin_${PLATFORM}.cpp
)

target_include_directories(system_plugin PUBLIC 
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

# Set output directory for plugins
set_target_properties(system_plugin PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
    ARCHIVE_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
    RUNTIME_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
)

# Platform-specific settings
if(WIN32)
    target_compile_definitions(system_plugin PUBLIC 
        _WIN32 
        WIN32_LEAN_AND_MEAN
        NOMINMAX
    )
    
    # Link Windows libraries
    target_link_libraries(system_plugin 
        kernel32
        psapi
        user32
        shell32
    )
    
    # Set DLL export
    target_compile_definitions(system_plugin PUBLIC PLUGIN_EXPORT)
    
elseif(UNIX AND NOT APPLE)
    # Linux
    target_link_libraries(system_plugin pthread)
    
elseif(APPLE)
    # macOS
    target_link_libraries(system_plugin pthread)
endif()

# Compiler-specific optimizations
if(CMAKE_CXX_COMPILER_ID STREQUAL "GNU" OR CMAKE_CXX_COMPILER_ID STREQUAL "Clang")
    target_compile_options(system_plugin PRIVATE
        -Wall
        -Wextra
        -O2
        -fPIC
    )
elseif(CMAKE_CXX_COMPILER_ID STREQUAL "MSVC")
    target_compile_options(system_plugin PRIVATE
        /W4
        /O2
        /MD
    )
endif()

# Plugin manager (optional - for advanced plugin management)
option(BUILD_PLUGIN_MANAGER "Build C++ plugin manager" OFF)
if(BUILD_PLUGIN_MANAGER)
    add_library(plugin_manager SHARED
        src/plugin_manager.cpp
    )
    
    target_include_directories(plugin_manager PUBLIC 
        ${CMAKE_CURRENT_SOURCE_DIR}/include
    )
    
    target_link_libraries(plugin_manager 
        ${CMAKE_DL_LIBS}
    )
    
    if(WIN32)
        target_link_libraries(plugin_manager 
            kernel32
            user32
        )
    endif()
    
    set_target_properties(plugin_manager PROPERTIES
        LIBRARY_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
        ARCHIVE_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
        RUNTIME_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
    )
endif()

# Custom command to copy plugins to agent directory
add_custom_command(TARGET system_plugin POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E make_directory ${CMAKE_SOURCE_DIR}/../agent/plugins
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
        $<TARGET_FILE:system_plugin>
        ${CMAKE_SOURCE_DIR}/../agent/plugins/
    COMMENT "Copying system_plugin to agent directory"
)

# Copy plugin manager if built
if(BUILD_PLUGIN_MANAGER)
    add_custom_command(TARGET plugin_manager POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
            $<TARGET_FILE:plugin_manager>
            ${CMAKE_SOURCE_DIR}/../agent/plugins/
        COMMENT "Copying plugin_manager to agent directory"
    )
endif()

# Install targets
install(TARGETS system_plugin
    LIBRARY DESTINATION plugins
    ARCHIVE DESTINATION plugins
    RUNTIME DESTINATION plugins
)

if(BUILD_PLUGIN_MANAGER)
    install(TARGETS plugin_manager
        LIBRARY DESTINATION plugins
        ARCHIVE DESTINATION plugins
        RUNTIME DESTINATION plugins
    )
endif()

# Print plugin information
message(STATUS "Plugin configuration:")
message(STATUS "  Platform: ${PLATFORM}")
message(STATUS "  Output directory: ${PLUGIN_OUTPUT_DIR}")
message(STATUS "  Plugin manager: ${BUILD_PLUGIN_MANAGER}")
