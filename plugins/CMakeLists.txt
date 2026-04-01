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

# Add all new plugins
set(NEW_PLUGIN_SOURCES
    src/directory_info_plugin.c
    src/event_data_plugin.c
    src/file_reader_plugin.c
    src/file_signature_plugin.c
    src/folder_watcher_plugin.c
    src/file_listener_plugin.c
    src/root_directory_info_plugin.c
    src/scanner_plugin.c
    src/watchers_manager_plugin.c
    src/plugin_registry.c
)

# Build each new plugin
foreach(PLUGIN_SOURCE ${NEW_PLUGIN_SOURCES})
    get_filename_component(PLUGIN_NAME ${PLUGIN_SOURCE} NAME_WE)
    
    add_library(${PLUGIN_NAME} SHARED ${PLUGIN_SOURCE})
    
    target_include_directories(${PLUGIN_NAME} PUBLIC 
        ${CMAKE_CURRENT_SOURCE_DIR}/include
    )
    
    set_target_properties(${PLUGIN_NAME} PROPERTIES
        LIBRARY_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
        ARCHIVE_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
        RUNTIME_OUTPUT_DIRECTORY ${PLUGIN_OUTPUT_DIR}
    )
    
    # Platform-specific linking for new plugins
    if(WIN32)
        target_link_libraries(${PLUGIN_NAME} kernel32 user32 shell32 advapi32)
    else()
        target_link_libraries(${PLUGIN_NAME} pthread dl)
        # Try to find and link OpenSSL for crypto plugins
        if(PLUGIN_NAME MATCHES "file_signature")
            find_package(PkgConfig REQUIRED)
            pkg_check_modules(OPENSSL REQUIRED openssl)
            target_link_libraries(${PLUGIN_NAME} ${OPENSSL_LIBRARIES})
            target_include_directories(${PLUGIN_NAME} PUBLIC ${OPENSSL_INCLUDE_DIRS})
        endif()
    endif()
    
    message(STATUS "Added plugin: ${PLUGIN_NAME}")
endforeach()

# Custom target to copy plugins to agent directory
add_custom_target(copy_all_plugins ALL
    COMMAND ${CMAKE_COMMAND} -E make_directory ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins
    COMMAND ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/system_plugin.dll ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/ || ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/libsystem_plugin.so ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/ || ${CMAKE_COMMAND} -E copy_if_different ${PLUGIN_OUTPUT_DIR}/libsystem_plugin.dylib ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/
    DEPENDS system_plugin
    COMMENT "Copying system plugin to agent directory"
)

# Copy all new plugins
foreach(PLUGIN_SOURCE ${NEW_PLUGIN_SOURCES})
    get_filename_component(PLUGIN_NAME ${PLUGIN_SOURCE} NAME_WE)
    add_custom_command(TARGET ${PLUGIN_NAME} POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
        $<TARGET_FILE:${PLUGIN_NAME}>
        ${CMAKE_CURRENT_SOURCE_DIR}/../agent/plugins/
        COMMENT "Copying ${PLUGIN_NAME} to agent directory"
    )
endforeach()

# Make copy_plugins run after building all plugins
add_dependencies(copy_all_plugins system_plugin)
foreach(PLUGIN_SOURCE ${NEW_PLUGIN_SOURCES})
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
    foreach(PLUGIN_SOURCE ${NEW_PLUGIN_SOURCES})
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
    foreach(PLUGIN_SOURCE ${NEW_PLUGIN_SOURCES})
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

foreach(PLUGIN_SOURCE ${NEW_PLUGIN_SOURCES})
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
message(STATUS "  New plugins: ${NEW_PLUGIN_SOURCES}")
message(STATUS "")
