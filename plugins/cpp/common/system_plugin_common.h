//! Common structures and interfaces for System Plugin v3
//! This header contains platform-independent types and interfaces

#pragma once

#include <memory>
#include <string>
#include <format>
#include <expected>
#include <span>
#include <ranges>
#include <algorithm>
#include <vector>
#include <chrono>
#include <atomic>
#include <thread>
#include <mutex>
#include <shared_mutex>
#include <concepts>
#include <type_traits>
#include <filesystem>
#include <system_error>
#include <iostream>
#include <fstream>
#include <unordered_map>

// FFI includes for Rust compatibility
extern "C" {
    // Forward declarations for Rust FFI structures
    struct PluginInfo {
        char* name;
        char* version;
        char* description;
        char* author;
        char* license;
        uint64_t m_timestamp;
    };

    struct PluginInterface {
        void* get_plugin_info;
        void* init;
        void* cleanup;
        void* get_system_metrics;
        void* get_processes;
        void* execute_command;
        void* read_file;
        void* get_system_info;
        void* get_directory_info_data;
        void* free_directory_info_data;
        void* get_file_signature_data;
        void* free_file_signature_data;
        void* get_root_directory_info;
        void* scan_directory;
        void* free_scan_result;
        void* create_folder_watcher;
        void* destroy_folder_watcher;
        void* create_file_listener;
        void* destroy_file_listener;
        void* get_watcher_events;
        void* free_watcher_events;
    };
}

// C++23 error handling
enum class SystemError : int {
    Success = 0,
    InvalidArgument = 1,
    SystemCallFailed = 2,
    OutOfMemory = 3,
    PermissionDenied = 4,
    NotFound = 5,
    Unknown = 99
};

// Error type with context
template<typename T>
using SystemResult = std::expected<T, SystemError>;

// String view wrapper
using StringView = std::string_view;

// Span wrapper
template<typename T>
using Span = std::span<T>;

// System metrics with C++23 features
struct SystemMetrics {
    double cpu_usage{0.0};
    double memory_usage{0.0};
    double disk_usage{0.0};
    uint64_t uptime_seconds{0};
    std::chrono::system_clock::time_point timestamp{};

    constexpr SystemMetrics() noexcept = default;

    constexpr SystemMetrics(
        double cpu,
        double memory,
        double disk,
        uint64_t uptime,
        std::chrono::system_clock::time_point time = std::chrono::system_clock::now()
    ) noexcept
        : cpu_usage{cpu}, memory_usage{memory}, disk_usage{disk},
          uptime_seconds{uptime}, timestamp{time} {}

    // Three-way comparison operator (C++20)
    auto operator<=>(const SystemMetrics& other) const noexcept -> std::partial_ordering {
        if (auto cmp = cpu_usage <=> other.cpu_usage; cmp != 0) return cmp;
        if (auto cmp = memory_usage <=> other.memory_usage; cmp != 0) return cmp;
        return disk_usage <=> other.disk_usage;
    }

    // formatting support
    friend auto operator<<(std::ostream& os, const SystemMetrics& metrics) -> std::ostream& {
        os << std::format("CPU: {:.1f}%, Memory: {:.1f}%, Disk: {:.1f}%, Uptime: {}s",
                       metrics.cpu_usage, metrics.memory_usage, metrics.disk_usage,
                       metrics.uptime_seconds);
        return os;
    }
};

// process information with RAII
struct ProcessInfo {
    uint32_t pid{0};
    std::string name{};
    double cpu_usage{0.0};
    uint64_t memory_usage{0};
    std::chrono::system_clock::time_point start_time{};

    constexpr ProcessInfo() noexcept = default;

    ProcessInfo(
        uint32_t id,
        std::string proc_name,
        double cpu,
        uint64_t memory,
        std::chrono::system_clock::time_point start
    ) noexcept
        : pid{id}, name{std::move(proc_name)}, cpu_usage{cpu},
          memory_usage{memory}, start_time{start} {}

    // range support
    [[nodiscard]] auto id() const noexcept -> uint32_t { return pid; }
    [[nodiscard]] auto get_name() const noexcept -> StringView { return name; }
};

// command result with expected
struct CommandResult {
    int exit_code{0};
    std::string stdout_data{};
    std::string stderr_data{};
    std::chrono::milliseconds execution_time{0};

    constexpr CommandResult() noexcept = default;

    CommandResult(
        int code,
        std::string stdout_str,
        std::string stderr_str,
        std::chrono::milliseconds time
    ) noexcept
        : exit_code{code}, stdout_data{std::move(stdout_str)},
          stderr_data{std::move(stderr_str)}, execution_time{time} {}

    [[nodiscard]] auto success() const noexcept -> bool { return exit_code == 0; }
};

// file content with RAII
struct FileContent {
    std::vector<uint8_t> data{};
    std::filesystem::path file_path{};
    std::chrono::system_clock::time_point last_modified{};

    constexpr FileContent() noexcept = default;

    FileContent(
        std::vector<uint8_t> content,
        std::filesystem::path path,
        std::chrono::system_clock::time_point modified
    ) noexcept
        : data{std::move(content)}, file_path{std::move(path)},
          last_modified{modified} {}

    [[nodiscard]] auto size() const noexcept -> size_t { return data.size(); }
    [[nodiscard]] auto empty() const noexcept -> bool { return data.empty(); }
};

// system information
struct SystemInfo {
    std::string hostname{};
    std::string os_name{};
    std::string os_version{};
    std::string architecture{};
    uint64_t total_memory{0};
    uint64_t total_disk_space{0};

    constexpr SystemInfo() noexcept = default;

    SystemInfo(
        std::string host,
        std::string os,
        std::string version,
        std::string arch,
        uint64_t memory,
        uint64_t disk
    ) noexcept
        : hostname{std::move(host)}, os_name{std::move(os)},
          os_version{std::move(version)}, architecture{std::move(arch)},
          total_memory{memory}, total_disk_space{disk} {}
};

// plugin interface with concepts
template<typename T>
concept SystemPluginConcept = requires(T plugin) {
    { plugin.get_name() } -> std::convertible_to<StringView>;
    { plugin.get_version() } -> std::convertible_to<StringView>;
    { plugin.initialize() } -> std::convertible_to<SystemResult<bool>>;
    { plugin.cleanup() } -> std::convertible_to<void>;
    { plugin.get_system_metrics() } -> std::convertible_to<SystemResult<SystemMetrics>>;
    { plugin.get_processes() } -> std::convertible_to<SystemResult<std::vector<ProcessInfo>>>;
    { plugin.execute_command(StringView{}) } -> std::convertible_to<SystemResult<CommandResult>>;
    { plugin.read_file(StringView{}) } -> std::convertible_to<SystemResult<FileContent>>;
    { plugin.get_system_info() } -> std::convertible_to<SystemResult<SystemInfo>>;
};

// Platform-specific interface
class PlatformInterface {
public:
    virtual ~PlatformInterface() = default;

    virtual SystemResult<std::vector<ProcessInfo>> get_processes() const = 0;
    virtual SystemResult<CommandResult> execute_command(StringView command) const = 0;
    virtual SystemResult<SystemInfo> get_system_info() const = 0;
    virtual uint64_t get_uptime_seconds() const = 0;
    virtual double get_memory_usage_percentage() const = 0;
};