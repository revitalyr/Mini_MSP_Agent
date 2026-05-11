//! Windows-specific implementation of System Plugin v3

#include "../common/system_plugin_common.h"
#include <windows.h>
#include <psapi.h>
#include <tlhelp32.h>
#include <shlobj.h>
#include <sysinfoapi.h>
#include <memoryapi.h>
#include <processthreadsapi.h>

// Windows-specific PlatformInterface implementation
class WindowsPlatform : public PlatformInterface {
public:
    SystemResult<std::vector<ProcessInfo>> get_processes() const override {
        try {
            std::vector<ProcessInfo> processes;

            // Windows API with RAII
            HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if (snapshot == INVALID_HANDLE_VALUE) {
                return std::unexpected(SystemError::SystemCallFailed);
            }

            // RAII handle management
            auto snapshot_guard = std::unique_ptr<void, decltype(&CloseHandle)>(
                snapshot, &CloseHandle);

            PROCESSENTRY32W pe32{};
            pe32.dwSize = sizeof(pe32);

            if (Process32FirstW(snapshot, &pe32)) {
                do {
                    // Convert wide string to UTF-8 using C++
                    int size = WideCharToMultiByte(CP_UTF8, 0, pe32.szExeFile, -1,
                                                 nullptr, 0, nullptr, nullptr);
                    if (size > 0) {
                        std::string name(size, 0);
                        WideCharToMultiByte(CP_UTF8, 0, pe32.szExeFile, -1,
                                         name.data(), size, nullptr, nullptr);

                        processes.emplace_back(
                            pe32.th32ProcessID,
                            std::move(name),
                            0.0, // TODO: Calculate CPU usage
                            0,    // TODO: Get memory usage
                            std::chrono::system_clock::now()
                        );
                    }
                } while (Process32NextW(snapshot, &pe32));
            }

            // Traditional sort instead of ranges
            std::sort(processes.begin(), processes.end(), [](const auto& a, const auto& b) {
                return a.pid < b.pid;
            });

            return processes;
        } catch (const std::system_error& e) {
            std::cerr << "System error in get_processes: " << e.what() << " (code: " << e.code() << ")\n";
            return std::unexpected(SystemError::SystemCallFailed);
        } catch (...) {
            std::cerr << "Unknown error in get_processes\n";
            return std::unexpected(SystemError::Unknown);
        }
    }

    SystemResult<CommandResult> execute_command(StringView command) const override {
        if (command.empty()) {
            return std::unexpected(SystemError::InvalidArgument);
        }

        try {
            auto start_time = std::chrono::steady_clock::now();

            // Windows process creation
            STARTUPINFOW si{};
            PROCESS_INFORMATION pi{};

            std::wstring wide_cmd(command.begin(), command.end());

            if (!CreateProcessW(
                nullptr, wide_cmd.data(), nullptr, nullptr, FALSE,
                CREATE_NO_WINDOW, nullptr, nullptr, &si, &pi)) {
                return std::unexpected(SystemError::SystemCallFailed);
            }

            // RAII process handle
            auto process_guard = std::unique_ptr<void, decltype(&CloseHandle)>(
                pi.hProcess, &CloseHandle);
            auto thread_guard = std::unique_ptr<void, decltype(&CloseHandle)>(
                pi.hThread, &CloseHandle);

            // Wait with timeout
            DWORD wait_result = WaitForSingleObject(pi.hProcess,
                static_cast<DWORD>(std::chrono::milliseconds{30000}.count()));

            if (wait_result == WAIT_TIMEOUT) {
                TerminateProcess(pi.hProcess, 1);
                return std::unexpected(SystemError::SystemCallFailed);
            }

            DWORD exit_code;
            if (!GetExitCodeProcess(pi.hProcess, &exit_code)) {
                return std::unexpected(SystemError::SystemCallFailed);
            }

            auto execution_time = std::chrono::duration_cast<std::chrono::milliseconds>(
                std::chrono::steady_clock::now() - start_time);

            return CommandResult{
                static_cast<int>(exit_code),
                "", // TODO: Capture stdout
                "", // TODO: Capture stderr
                execution_time
            };
        } catch (const std::system_error& e) {
            std::cerr << "System error in execute_command: " << e.what() << " (code: " << e.code() << ")\n";
            return std::unexpected(SystemError::SystemCallFailed);
        } catch (...) {
            std::cerr << "Unknown error in execute_command\n";
            return std::unexpected(SystemError::Unknown);
        }
    }

    SystemResult<SystemInfo> get_system_info() const override {
        try {
            SystemInfo info{};

            // Windows system information
            char hostname_buffer[256] = {};
            DWORD hostname_size = sizeof(hostname_buffer);
            if (GetComputerNameA(hostname_buffer, &hostname_size)) {
                info.hostname = hostname_buffer;
            }

            // OS information (use RtlGetVersion to avoid deprecation warning)
            typedef LONG (WINAPI *RtlGetVersionPtr)(POSVERSIONINFOEXA);
            OSVERSIONINFOEXA osvi{};
            osvi.dwOSVersionInfoSize = sizeof(osvi);

            HMODULE hMod = GetModuleHandleA("ntdll.dll");
            if (hMod) {
                RtlGetVersionPtr rtlGetVersion = (RtlGetVersionPtr)GetProcAddress(hMod, "RtlGetVersion");
                if (rtlGetVersion && rtlGetVersion(&osvi) == 0) {
                    info.os_name = "Windows";
                    info.os_version = std::format("{}.{}.{}",
                        osvi.dwMajorVersion, osvi.dwMinorVersion, osvi.dwBuildNumber);
                }
            }

            // Architecture
            SYSTEM_INFO si{};
            GetSystemInfo(&si);
            info.architecture = (si.wProcessorArchitecture == PROCESSOR_ARCHITECTURE_AMD64) ? "x64" : "x86";

            // Memory information
            MEMORYSTATUSEX memStatus{};
            memStatus.dwLength = sizeof(memStatus);
            if (GlobalMemoryStatusEx(&memStatus)) {
                info.total_memory = memStatus.ullTotalPhys;
            }

            // Disk space information
            try {
                auto space = std::filesystem::space(std::filesystem::current_path());
                info.total_disk_space = space.capacity;
            } catch (...) {
                // Ignore disk space errors
            }

            return info;
        } catch (...) {
            return std::unexpected(SystemError::Unknown);
        }
    }

    uint64_t get_uptime_seconds() const override {
        return GetTickCount64() / 1000;
    }

    double get_memory_usage_percentage() const override {
        MEMORYSTATUSEX memStatus{};
        memStatus.dwLength = sizeof(memStatus);
        if (GlobalMemoryStatusEx(&memStatus)) {
            return (1.0 - (static_cast<double>(memStatus.ullAvailPhys) /
                          static_cast<double>(memStatus.ullTotalPhys))) * 100.0;
        }
        return 0.0;
    }
};

// system plugin implementation
class SystemPlugin {
private:
    // atomic state
    std::atomic<bool> initialized_{false};
    mutable std::atomic<uint32_t> request_count_{0};

    // thread management
    std::jthread monitoring_thread_;
    mutable std::shared_mutex metrics_mutex_;
    SystemMetrics cached_metrics_;

    // platform interface
    std::unique_ptr<PlatformInterface> platform_;

    // configuration
    static constexpr std::chrono::seconds UPDATE_INTERVAL{5};
    static constexpr std::chrono::milliseconds COMMAND_TIMEOUT{30000};

public:
    // constructors
    SystemPlugin() : platform_{std::make_unique<WindowsPlatform>()} {}

    // Non-copyable, movable
    SystemPlugin(const SystemPlugin&) = delete;
    SystemPlugin& operator=(const SystemPlugin&) = delete;
    SystemPlugin(SystemPlugin&&) noexcept = delete;
    SystemPlugin& operator=(SystemPlugin&&) noexcept = delete;

    // destructor
    ~SystemPlugin() noexcept {
        cleanup();
    }

    // Plugin interface
    [[nodiscard]] constexpr auto get_name() const noexcept -> StringView {
        return "_system_plugin_v3";
    }

    [[nodiscard]] constexpr auto get_version() const noexcept -> StringView {
        return "3.0.0";
    }

    [[nodiscard]] constexpr auto get_description() const noexcept -> StringView {
        return "C++23 system plugin with advanced features";
    }

    auto initialize() -> SystemResult<bool> {
        if (initialized_.load(std::memory_order_acquire)) {
            return std::unexpected(SystemError::InvalidArgument);
        }

        try {
            // Start monitoring thread with jthread
            monitoring_thread_ = std::jthread([this](std::stop_token stop_token) {
                this->monitoring_loop(std::move(stop_token));
            });

            initialized_.store(true, std::memory_order_release);
            return true;
        } catch (const std::system_error& e) {
            std::cerr << "System error in init: " << e.what() << " (code: " << e.code() << ")\n";
            return std::unexpected(SystemError::SystemCallFailed);
        } catch (...) {
            std::cerr << "Unknown error in init\n";
            return std::unexpected(SystemError::Unknown);
        }
    }

    auto cleanup() noexcept -> void {
        if (!initialized_.load(std::memory_order_acquire)) {
            return;
        }

        try {
            // Stop monitoring thread
            monitoring_thread_.request_stop();
            if (monitoring_thread_.joinable()) {
                monitoring_thread_.join();
            }

            initialized_.store(false, std::memory_order_release);
        } catch (...) {
            // Ignore exceptions in destructor
        }
    }

    // system metrics collection
    [[nodiscard]] auto get_system_metrics() const -> SystemResult<SystemMetrics> {
        if (!initialized_.load(std::memory_order_acquire)) {
            return std::unexpected(SystemError::InvalidArgument);
        }

        std::shared_lock lock{metrics_mutex_);
        return cached_metrics_;
    }

    // process enumeration with ranges
    [[nodiscard]] auto get_processes() const -> SystemResult<std::vector<ProcessInfo>> {
        if (!initialized_.load(std::memory_order_acquire)) {
            return std::unexpected(SystemError::InvalidArgument);
        }

        return platform_->get_processes();
    }

    // command execution with expected and timeout
    [[nodiscard]] auto execute_command(StringView command) const -> SystemResult<CommandResult> {
        if (!initialized_.load(std::memory_order_acquire)) {
            return std::unexpected(SystemError::InvalidArgument);
        }

        if (command.empty()) {
            return std::unexpected(SystemError::InvalidArgument);
        }

        // Increment request counter atomically
        request_count_.fetch_add(1, std::memory_order_relaxed);

        return platform_->execute_command(command);
    }

    // System information gathering with RAII
    [[nodiscard]] auto read_file(StringView path) const -> SystemResult<FileContent> {
        if (!initialized_.load(std::memory_order_acquire)) {
            return std::unexpected(SystemError::InvalidArgument);
        }

        if (path.empty()) {
            return std::unexpected(SystemError::InvalidArgument);
        }

        try {
            std::filesystem::path file_path(path);

            if (!std::filesystem::exists(file_path)) {
                return std::unexpected(SystemError::NotFound);
            }

            if (!std::filesystem::is_regular_file(file_path)) {
                return std::unexpected(SystemError::InvalidArgument);
            }

            // Get file size and last modified time
            auto file_size = std::filesystem::file_size(file_path);
            auto last_modified = std::filesystem::last_write_time(file_path);
            auto last_modified_time = std::chrono::system_clock::now(); // Default initialization

            try {
                last_modified_time = std::chrono::time_point_cast<std::chrono::system_clock::duration>(
                    last_modified - std::filesystem::file_time_type::clock::now() + std::chrono::system_clock::now());
            } catch (...) {
                // Use current time as fallback
            }

            // Read file with RAII
            std::ifstream file(file_path, std::ios::binary);
            if (!file) {
                return std::unexpected(SystemError::PermissionDenied);
            }

            std::vector<uint8_t> data(file_size);
            file.read(reinterpret_cast<char*>(data.data()), static_cast<std::streamsize>(file_size));

            if (!file) {
                return std::unexpected(SystemError::SystemCallFailed);
            }

            return FileContent{
                std::move(data),
                std::move(file_path),
                last_modified_time
            };
        } catch (const std::filesystem::filesystem_error& e) {
            if (e.code() == std::errc::no_such_file_or_directory) {
                return std::unexpected(SystemError::NotFound);
            } else if (e.code() == std::errc::permission_denied) {
                return std::unexpected(SystemError::PermissionDenied);
            } else {
                return std::unexpected(SystemError::SystemCallFailed);
            }
        } catch (...) {
            return std::unexpected(SystemError::Unknown);
        }
    }

    // system information collection
    [[nodiscard]] auto get_system_info() const -> SystemResult<SystemInfo> {
        if (!initialized_.load(std::memory_order_acquire)) {
            return std::unexpected(SystemError::InvalidArgument);
        }

        return platform_->get_system_info();
    }

    // statistics
    [[nodiscard]] auto get_request_count() const noexcept -> uint32_t {
        return request_count_.load(std::memory_order_relaxed);
    }

private:
    // monitoring loop with stop token
    auto monitoring_loop(std::stop_token stop_token) -> void {
        while (!stop_token.stop_requested()) {
            try {
                update_metrics();
            } catch (...) {
                // Log error but continue monitoring
            }

            // sleep with stop token support
            std::this_thread::sleep_for(UPDATE_INTERVAL);
        }
    }

    // metrics update
    auto update_metrics() -> void {
        SystemMetrics metrics{};

        // Get uptime
        metrics.uptime_seconds = platform_->get_uptime_seconds();

        // Get CPU usage
        metrics.cpu_usage = get_cpu_usage_percentage();

        // Get memory usage
        metrics.memory_usage = platform_->get_memory_usage_percentage();

        // Get disk usage
        metrics.disk_usage = get_disk_usage_percentage();

        // Update cached metrics
        std::unique_lock lock{metrics_mutex_);
        cached_metrics_ = SystemMetrics{
            metrics.cpu_usage,
            metrics.memory_usage,
            metrics.disk_usage,
            metrics.uptime_seconds,
            std::chrono::system_clock::now()
        };
    }

    // Platform-specific implementations
    [[nodiscard]] static auto get_cpu_usage_percentage() -> double {
        // Simplified CPU usage calculation
        // In production, implement proper CPU usage tracking
        return 25.0; // Placeholder
    }

    [[nodiscard]] static auto get_disk_usage_percentage() -> double {
        try {
            auto space = std::filesystem::space(std::filesystem::current_path());
            return (1.0 - (static_cast<double>(space.available) /
                          static_cast<double>(space.capacity))) * 100.0;
        } catch (...) {
            return 0.0;
        }
    }
};

// Plugin factory with concepts
template<typename T>
requires SystemPluginConcept<T>
auto create_system_plugin() -> std::unique_ptr<T> {
    return std::make_unique<T>();
}

// C++23 exported functions using extern "C"
extern "C" {
    // Plugin information
    [[nodiscard]] const char* get_plugin_info() {
        static constexpr const char* info = "_system_plugin_v3:3.0.0:C++23 system plugin";
        return info;
    }

    // Plugin initialization
    [[nodiscard]] bool plugin_initialize() {
        return true; // Simplified initialization
    }

    // Plugin cleanup
    void plugin_cleanup() noexcept {
        // Cleanup handled by destructor
    }

    // Plugin interface getter for Rust agent
    [[nodiscard]] PluginInterface* get_plugin_interface() {
        static PluginInterface plugin_interface;
        static bool initialized = false;

        if (!initialized) {
            // Initialize function pointers only once
            plugin_interface.get_plugin_info = nullptr;
            plugin_interface.init = nullptr;
            plugin_interface.cleanup = nullptr;

            // Set other functions to nullptr for now
            plugin_interface.get_system_metrics = nullptr;
            plugin_interface.get_processes = nullptr;
            plugin_interface.execute_command = nullptr;
            plugin_interface.read_file = nullptr;
            plugin_interface.get_system_info = nullptr;
            plugin_interface.get_directory_info_data = nullptr;
            plugin_interface.free_directory_info_data = nullptr;
            plugin_interface.get_file_signature_data = nullptr;
            plugin_interface.free_file_signature_data = nullptr;
            plugin_interface.get_root_directory_info = nullptr;
            plugin_interface.scan_directory = nullptr;
            plugin_interface.free_scan_result = nullptr;
            plugin_interface.create_folder_watcher = nullptr;
            plugin_interface.destroy_folder_watcher = nullptr;
            plugin_interface.create_file_listener = nullptr;
            plugin_interface.destroy_file_listener = nullptr;
            plugin_interface.get_watcher_events = nullptr;
            plugin_interface.free_watcher_events = nullptr;

            initialized = true;
        }

        return &plugin_interface;
    }
}

// DLL entry point
BOOL APIENTRY DllMain(HMODULE hModule, DWORD ul_reason_for_call, LPVOID) noexcept {
    switch (ul_reason_for_call) {
        case DLL_PROCESS_ATTACH:
            // initialization
            DisableThreadLibraryCalls(hModule);
            break;
        case DLL_THREAD_ATTACH:
        case DLL_THREAD_DETACH:
        case DLL_PROCESS_DETACH:
            break;
    }
    return TRUE;
}