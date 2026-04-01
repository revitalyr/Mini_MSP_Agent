cmake_minimum_required(VERSION 3.12)
project(mini_msp_plugins)

# Set C and C++ standards
set(CMAKE_C_STANDARD 99)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Detect platform
if(WIN32)
    set(PLATFORM "windows")
elseif(APPLE)
    set(PLATFORM "macos")
else()
    set(PLATFORM "unix")
endif()

message(STATUS "Building plugins for platform: ${PLATFORM}")

# Set output directory for plugins
set(PLUGIN_OUTPUT_DIR ${CMAKE_CURRENT_BINARY_DIR}/plugins)

# Create plugin output directory
file(MAKE_DIRECTORY ${PLUGIN_OUTPUT_DIR})

# System plugin (basic)
add_library(system_plugin SHARED
    src/${PLATFORM}/simple_plugin.c
)

target_include_directories(system_plugin PUBLIC 
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

set_target_properties(system_plugin PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
    ARCHIVE_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
    RUNTIME_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
)

# Platform-specific linking
if(WIN32)
    target_link_libraries(system_plugin psapi)
elseif(APPLE)
    target_link_libraries(system_plugin)
else()
    target_link_libraries(system_plugin)
endif()

# Directory Information Plugin
add_library(directory_info_plugin SHARED
    src/${PLATFORM}/directory_info_plugin.c
)

target_include_directories(directory_info_plugin PUBLIC 
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

set_target_properties(directory_info_plugin PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
    ARCHIVE_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
    RUNTIME_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
)

if(WIN32)
    target_link_libraries(directory_info_plugin psapi)
endif()

# Event Data Plugin
add_library(event_data_plugin SHARED
    src/${PLATFORM}/event_data_plugin.c
)

target_include_directories(event_data_plugin PUBLIC 
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

set_target_properties(event_data_plugin PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
    ARCHIVE_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
    RUNTIME_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
)

if(WIN32)
    target_link_libraries(event_data_plugin psapi wevtapi)
endif()

# File Signature Plugin
add_library(file_signature_plugin SHARED
    src/${PLATFORM}/file_signature_plugin.c
)

target_include_directories(file_signature_plugin PUBLIC 
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

set_target_properties(file_signature_plugin PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
    ARCHIVE_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
    RUNTIME_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
)

if(WIN32)
    target_link_libraries(file_signature_plugin psapi advapi32)
endif()

# Folder Watcher Plugin
add_library(folder_watcher_plugin SHARED
    src/${PLATFORM}/folder_watcher_plugin.c
)

target_include_directories(folder_watcher_plugin PUBLIC 
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

set_target_properties(folder_watcher_plugin PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
    ARCHIVE_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
    RUNTIME_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
)

if(WIN32)
    target_link_libraries(folder_watcher_plugin psapi)
endif()

# Custom target to copy all plugins to agent directory
add_custom_target(copy_plugins ALL
    COMMAND ${CMAKE_COMMAND} -E make_directory ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins
    COMMAND ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/system_plugin.dll ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/ || ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/libsystem_plugin.so ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/ || ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/libsystem_plugin.dylib ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/
    COMMAND ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/directory_info_plugin.dll ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/ || ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/libdirectory_info_plugin.so ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/ || ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/libdirectory_info_plugin.dylib ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/
    COMMAND ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/event_data_plugin.dll ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/ || ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/libevent_data_plugin.so ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/ || ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/libevent_data_plugin.dylib ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/
    COMMAND ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/file_signature_plugin.dll ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/ || ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/libfile_signature_plugin.so ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/ || ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/libfile_signature_plugin.dylib ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/
    COMMAND ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/folder_watcher_plugin.dll ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/ || ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/libfolder_watcher_plugin.so ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/ || ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/libfolder_watcher_plugin.dylib ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/
    DEPENDS system_plugin directory_info_plugin event_data_plugin file_signature_plugin folder_watcher_plugin
    COMMENT "Copying all plugins to agent directory"
)

# Make copy_plugins run after building all plugins
add_dependencies(copy_plugins system_plugin directory_info_plugin event_data_plugin file_signature_plugin folder_watcher_plugin)

# Compiler-specific optimizations
if(CMAKE_CXX_COMPILER_ID STREQUAL "GNU" OR CMAKE_CXX_COMPILER_ID STREQUAL "Clang")
    target_compile_options(system_plugin PRIVATE
        -Wall
        -Wextra
        -O2
        -fPIC
    )
    target_compile_options(directory_info_plugin PRIVATE
        -Wall
        -Wextra
        -O2
        -fPIC
    )
    target_compile_options(event_data_plugin PRIVATE
        -Wall
        -Wextra
        -O2
        -fPIC
    )
    target_compile_options(file_signature_plugin PRIVATE
        -Wall
        -Wextra
        -O2
        -fPIC
    )
    target_compile_options(folder_watcher_plugin PRIVATE
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

# Custom command to copy plugins to agent directory
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
