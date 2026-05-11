//! macOS-specific implementation of System Plugin v3

#include "../common/system_plugin_common.h"
#include <unistd.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <signal.h>
#include <fcntl.h>
#include <sys/sysctl.h>
#include <sys/utsname.h>
#include <mach/mach.h>
#include <mach/vm_statistics.h>
#include <libproc.h>

// macOS-specific PlatformInterface implementation
class MacOSPlatform : public PlatformInterface {
public:
    SystemResult<std::vector<ProcessInfo>> get_processes() const override {
        try {
            std::vector<ProcessInfo> processes;

            // macOS using libproc
            // This is a simplified implementation
            // In production, use libproc.h for proper process enumeration
            int pid_count = proc_listpids(PROC_ALL_PIDS, 0, nullptr, 0);
            if (pid_count <= 0) {
                return std::unexpected(SystemError::SystemCallFailed);
            }

            std::vector<pid_t> pids(pid_count);
            pid_count = proc_listpids(PROC_ALL_PIDS, 0, pids.data(),
                                    static_cast<int>(pids.size() * sizeof(pid_t)));

            for (int i = 0; i < pid_count; ++i) {
                if (pids[i] == 0) continue;

                char name_buffer[1024];
                proc_name(pids[i], name_buffer, sizeof(name_buffer));

                processes.emplace_back(
                    static_cast<uint32_t>(pids[i]),
                    name_buffer,
                    0.0, // TODO: Calculate CPU usage
                    0,    // TODO: Get memory usage
                    std::chrono::system_clock::now()
                );
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

            // Unix/macOS process execution
            // Simple pipe management
            struct PipeGuard {
                int fds[2];
                PipeGuard() { fds[0] = fds[1] = -1; }
                ~PipeGuard() { if (fds[0] != -1) close(fds[0]); if (fds[1] != -1) close(fds[1]); }
            } pipe_guard;

            if (pipe(pipe_guard.fds) == -1) {
                return std::unexpected(SystemError::SystemCallFailed);
            }

            pid_t pid = fork();
            if (pid == -1) {
                return std::unexpected(SystemError::SystemCallFailed);
            }

            if (pid == 0) {
                // Child process
                close(pipe_guard.fds[0]);
                dup2(pipe_guard.fds[1], STDOUT_FILENO);
                dup2(pipe_guard.fds[1], STDERR_FILENO);
                close(pipe_guard.fds[1]);

                execl("/bin/sh", "sh", "-c", std::string(command).c_str(), nullptr);
                _exit(127);
            }

            // Parent process
            close(pipe_guard.fds[1]);

            // Read output with timeout
            std::string output;
            char buffer[4096];
            ssize_t bytes_read;

            auto deadline = std::chrono::steady_clock::now() + std::chrono::milliseconds{30000};

            while ((bytes_read = read(pipe_guard.fds[0], buffer, sizeof(buffer))) > 0) {
                output.append(buffer, static_cast<size_t>(bytes_read));

                if (std::chrono::steady_clock::now() > deadline) {
                    kill(pid, SIGKILL);
                    return std::unexpected(SystemError::SystemCallFailed);
                }
            }

            close(pipe_guard.fds[0]);

            // Wait for process
            int status;
            pid_t result = waitpid(pid, &status, 0);

            auto execution_time = std::chrono::duration_cast<std::chrono::milliseconds>(
                std::chrono::steady_clock::now() - start_time);

            if (result == -1) {
                return std::unexpected(SystemError::SystemCallFailed);
            }

            return CommandResult{
                WIFEXITED(status) ? WEXITSTATUS(status) : -1,
                output,
                "", // Combined stdout/stderr
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

            // macOS system information
            struct utsname uts{};
            if (uname(&uts) == 0) {
                info.hostname = uts.nodename;
                info.os_name = uts.sysname;
                info.os_version = uts.release;
                info.architecture = uts.machine;
            }

            // Memory information using sysctl
            int mib[2] = {CTL_HW, HW_MEMSIZE};
            uint64_t memsize = 0;
            size_t len = sizeof(memsize);
            if (sysctl(mib, 2, &memsize, &len, nullptr, 0) == 0) {
                info.total_memory = memsize;
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
        // macOS uptime
        struct timeval boottime{};
        size_t len = sizeof(boottime);
        int mib[2] = {CTL_KERN, KERN_BOOTTIME};
        if (sysctl(mib, 2, &boottime, &len, nullptr, 0) == 0) {
            auto now = std::chrono::system_clock::now();
            auto boot = std::chrono::system_clock::from_time_t(boottime.tv_sec);
            return std::chrono::duration_cast<std::chrono::seconds>(now - boot).count();
        }
        return 0;
    }

    double get_memory_usage_percentage() const override {
        // macOS memory usage
        vm_statistics64_data_t vm_stat{};
        mach_msg_type_number_t count = HOST_VM_INFO64_COUNT;
        if (host_statistics64(mach_host_self(), HOST_VM_INFO64,
                              reinterpret_cast<host_info64_t>(&vm_stat), &count) == KERN_SUCCESS) {
            uint64_t total = vm_stat.wire_count + vm_stat.active_count +
                           vm_stat.inactive_count + vm_stat.free_count;
            uint64_t used = vm_stat.wire_count + vm_stat.active_count;
            return (static_cast<double>(used) / static_cast<double>(total)) * 100.0;
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
    SystemPlugin() : platform_{std::make_unique<MacOSPlatform>()} {}

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