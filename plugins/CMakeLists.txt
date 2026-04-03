cmake_minimum_required(VERSION 3.12)
project(mini_msp_plugins)

# Set C standard (C++ removed)
set(CMAKE_C_STANDARD 99)
set(CMAKE_C_STANDARD_REQUIRED ON)

# --- Security Configuration ---
find_program(OPENSSL_EXECUTABLE openssl REQUIRED)
set(PLUGIN_PRIVATE_KEY "${CMAKE_CURRENT_SOURCE_DIR}/../security/agent_private.pem")

if(NOT EXISTS ${PLUGIN_PRIVATE_KEY})
    message(WARNING "Private key for signing not found at ${PLUGIN_PRIVATE_KEY}. Plugins will not be signed correctly for production use.")
endif()

message(STATUS "Using OpenSSL for signing: ${OPENSSL_EXECUTABLE}")
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
