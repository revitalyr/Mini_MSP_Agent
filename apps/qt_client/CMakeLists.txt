cmake_minimum_required(VERSION 3.20)
project(MiniMSPQtClient VERSION 0.1.0 LANGUAGES CXX)
 
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)
 
# Find Qt6 packages
find_package(Qt6 REQUIRED COMPONENTS
    Core
    Gui
    Widgets
    Network
)
 
# Find nats.c library (cnats)
# Option 1: Use system-installed cnats
find_package(cnats QUIET)
 
# Option 2: Fetch cnats from source if not found
if(NOT cnats_FOUND)
    include(FetchContent)
    FetchContent_Declare(
        cnats
        GIT_REPOSITORY https://github.com/nats-io/nats.c.git
        GIT_TAG v3.9.2
        GIT_SHALLOW TRUE
    )
 
    # cnats uses its own CMakeLists.txt
    # Set options before fetching
    set(NATS_BUILD_STREAMING OFF CACHE BOOL "" FORCE)
    set(NATS_BUILD_TESTS OFF CACHE BOOL "" FORCE)
    set(NATS_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
    set(NATS_BUILD_STATIC ON CACHE BOOL "" FORCE)
 
    FetchContent_MakeAvailable(cnats)
endif()
 
# Find nlohmann/json for JSON handling
find_package(nlohmann_json QUIET)
if(NOT nlohmann_json_FOUND)
    include(FetchContent)
    FetchContent_Declare(
        json
        GIT_REPOSITORY https://github.com/nlohmann/json.git
        GIT_TAG v3.11.3
        GIT_SHALLOW TRUE
    )
    FetchContent_MakeAvailable(json)
endif()
 
# Source files
set(SOURCES
    src/main.cpp
    src/natsclient.cpp
    src/mainwindow.cpp
    src/agentmodel.cpp
    src/agentwidget.cpp
)
 
set(HEADERS
    src/natsclient.h
    src/mainwindow.h
    src/agentmodel.h
    src/agentwidget.h
)
 
set(UI_FILES
    src/mainwindow.ui
    src/agentwidget.ui
)
 
# Create executable
add_executable(${PROJECT_NAME}
    ${SOURCES}
    ${HEADERS}
    ${UI_FILES}
)
 
# Link libraries
target_link_libraries(${PROJECT_NAME} PRIVATE
    Qt6::Core
    Qt6::Gui
    Qt6::Widgets
    Qt6::Network
    nats_static
    nlohmann_json::nlohmann_json
)
 
# Include directories
target_include_directories(${PROJECT_NAME} PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/src
    ${cnats_SOURCE_DIR}/include
)
 
# Compiler options
target_compile_options(${PROJECT_NAME} PRIVATE
    -Wall
    -Wextra
    -Wpedantic
)
 
# Installation
install(TARGETS ${PROJECT_NAME}
    RUNTIME DESTINATION bin
)