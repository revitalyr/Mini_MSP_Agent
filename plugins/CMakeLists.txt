cmake_minimum_required(VERSION 3.12)
project(mini_msp_plugins)

# Set C standard (C++ removed)
set(CMAKE_C_STANDARD 99)
set(CMAKE_C_STANDARD_REQUIRED ON)

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

set_target_properties(system_plugin PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
    ARCHIVE_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
    RUNTIME_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
)

# Platform-specific linking and libraries
if(WIN32)
    target_link_libraries(system_plugin psapi kernel32)
    target_compile_definitions(system_plugin PRIVATE WIN32_LEAN_AND_MEAN)
elseif(APPLE)
    find_library(COREFOUNDATION_LIBRARY CoreFoundation)
    find_library(IOKIT_LIBRARY IOKit)
    find_library(MACH_LIBRARY mach)
    target_link_libraries(system_plugin ${COREFOUNDATION_LIBRARY} ${IOKIT_LIBRARY} ${MACH_LIBRARY})
else()
    target_link_libraries(system_plugin)
endif()

# Custom target to copy plugin to agent directory
add_custom_target(copy_plugins ALL
    COMMAND ${CMAKE_COMMAND} -E make_directory ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins
    COMMAND ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/system_plugin.dll ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/ || ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/libsystem_plugin.so ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/ || ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/libsystem_plugin.dylib ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/
    DEPENDS system_plugin
    COMMENT "Copying system plugin to agent directory"
)

# Make copy_plugins run after building plugin
add_dependencies(copy_plugins system_plugin)

# Compiler-specific optimizations for C
if(CMAKE_C_COMPILER_ID STREQUAL "GNU" OR CMAKE_C_COMPILER_ID STREQUAL "Clang")
    target_compile_options(system_plugin PRIVATE
        -Wall
        -Wextra
        -O2
        -fPIC
    )
elseif(CMAKE_C_COMPILER_ID STREQUAL "MSVC")
    target_compile_options(system_plugin PRIVATE
        /W4
        /O2
        /MD
    )
endif()

# Custom command to copy plugin to agent directory
add_custom_command(TARGET system_plugin POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E make_directory ${CMAKE_SOURCE_DIR}/../agent/plugins
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
        $<TARGET_FILE:system_plugin>
        ${CMAKE_SOURCE_DIR}/../agent/plugins/
    COMMENT "Copying system_plugin to agent directory"
)

# Install targets
install(TARGETS system_plugin
    LIBRARY DESTINATION plugins
    ARCHIVE DESTINATION plugins
    RUNTIME DESTINATION plugins
)

# Print plugin information
message(STATUS "Plugin configuration:")
message(STATUS "  Platform: ${PLATFORM}")
message(STATUS "  Output directory: ${PLUGIN_OUTPUT_DIR}")
message(STATUS "  Source: src/${PLATFORM}/system_plugin_${PLATFORM}.c")
