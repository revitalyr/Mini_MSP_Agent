//! Modern C++23 Directory Info Plugin for Mini MSP Agent
//! 
//! Complete rewrite of the C directory info plugin using modern C++23:
//! - std::expected for error handling
//! - std::span for safe array access
//! - std::ranges for directory traversal
//! - std::format for string formatting
//! - RAII for resource management
//! - Modern filesystem operations
//! - Async directory scanning
//! - Memory-efficient large directory handling
//! - Cross-platform path handling

#include <memory>
#include <string>
#include <format>
#include <expected>
#include <span>
#include <ranges>
#include <vector>
#include <filesystem>
#include <chrono>
#include <thread>
#include <mutex>
#include <shared_mutex>
#include <atomic>
#include <unordered_map>
#include <regex>
#include <concepts>
#include <type_traits>
#include <system_error>
#include <iostream>
#include <fstream>
#include <algorithm>
#include <future>

#ifdef _WIN32
#include <windows.h>
#include <fileapi.h>
#include <handleapi.h>
#include <winbase.h>
#else
#include <unistd.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <dirent.h>
#include <fcntl.h>
#endif

// Modern error handling for directory operations
enum class DirectoryError : int {
    Success = 0,
    DirectoryNotFound = 1,
    PermissionDenied = 2,
    InvalidPath = 3,
    IoError = 4,
    OutOfMemory = 5,
    ScanTimeout = 6,
    TooManyFiles = 7,
    Unknown = 99
};

// Modern result type
template<typename T>
using DirectoryResult = std::expected<T, DirectoryError>;

// Modern string view wrapper
using StringView = std::string_view;

// Modern span wrapper
template<typename T>
using Span = std::span<T>;

// Modern file entry information
struct FileEntry {
    std::filesystem::path path{};
    std::string name{};
    bool is_directory{false};
    bool is_symlink{false};
    bool is_hidden{false};
    uint64_t size{0};
    std::chrono::system_clock::time_point modified_time{};
    std::chrono::system_clock::time_point created_time{};
    std::string file_type{};
    std::string permissions{};
    
    constexpr FileEntry() noexcept = default;
    
    FileEntry(
        std::filesystem::path entry_path,
        std::string entry_name,
        bool dir,
        bool symlink,
        bool hidden,
        uint64_t file_size,
        std::chrono::system_clock::time_point modified,
        std::chrono::system_clock::time_point created,
        std::string type,
        std::string perms
    ) noexcept 
        : path{std::move(entry_path)}, name{std::move(entry_name)}, is_directory{dir},
          is_symlink{symlink}, is_hidden{hidden}, size{file_size},
          modified_time{modified}, created_time{created},
          file_type{std::move(type)}, permissions{std::move(perms)} {}
    
    // Three-way comparison operator
    auto operator<=>(const FileEntry& other) const noexcept -> std::strong_ordering {
        if (auto cmp = is_directory <=> other.is_directory; cmp != 0) return cmp;
        if (auto cmp = name <=> other.name; cmp != 0) return cmp;
        return size <=> other.size;
    }
    
    // Modern formatting
    friend auto operator<<(std::ostream& os, const FileEntry& entry) -> std::ostream& {
        os << std::format("Entry: {} ({} bytes, {})", 
                       entry.name, entry.size, 
                       entry.is_directory ? "dir" : "file");
        return os;
    }
};

// Modern directory statistics
struct DirectoryStats {
    uint64_t total_files{0};
    uint64_t total_directories{0};
    uint64_t total_size{0};
    uint64_t hidden_files{0};
    uint64_t hidden_directories{0};
    uint64_t symlinks{0};
    std::chrono::system_clock::time_point scan_time{};
    std::chrono::milliseconds scan_duration{0};
    std::filesystem::path scanned_path{};
    
    constexpr DirectoryStats() noexcept = default;
    
    DirectoryStats(
        uint64_t files,
        uint64_t dirs,
        uint64_t total_sz,
        uint64_t hidden_files_count,
        uint64_t hidden_dirs_count,
        uint64_t symlink_count,
        std::chrono::system_clock::time_point scan,
        std::chrono::milliseconds duration,
        std::filesystem::path path
    ) noexcept 
        : total_files{files}, total_directories{dirs}, total_size{total_sz},
          hidden_files{hidden_files_count}, hidden_directories{hidden_dirs_count},
          symlinks{symlink_count}, scan_time{scan}, scan_duration{duration},
          scanned_path{std::move(path)} {}
    
    // Modern formatting
    friend auto operator<<(std::ostream& os, const DirectoryStats& stats) -> std::ostream& {
        os << std::format("Stats: {} files, {} dirs, {} bytes, scan time: {}ms", 
                       stats.total_files, stats.total_directories, stats.total_size,
                       stats.scan_duration.count());
        return os;
    }
};

// Modern scan configuration
struct ScanConfig {
    bool include_hidden{false};
    bool follow_symlinks{false};
    bool recursive{true};
    uint32_t max_depth{100};
    uint64_t max_files{1000000};
    std::chrono::seconds timeout{300};
    std::vector<std::string> exclude_patterns{};
    
    constexpr ScanConfig() noexcept = default;
    
    ScanConfig(
        bool hidden,
        bool symlinks,
        bool recurse,
        uint32_t depth,
        uint64_t max_f,
        std::chrono::seconds time_limit,
        std::vector<std::string> patterns
    ) noexcept 
        : include_hidden{hidden}, follow_symlinks{symlinks}, recursive{recurse},
          max_depth{depth}, max_files{max_f}, timeout{time_limit},
          exclude_patterns{std::move(patterns)} {}
};

// Modern directory scanner interface with concepts
template<typename T>
concept DirectoryScanner = requires(T scanner, StringView path, const ScanConfig& config) {
    { scanner.scan_directory(path, config) } -> std::convertible_to<DirectoryResult<std::vector<FileEntry>>>;
    { scanner.get_stats(path, config) } -> std::convertible_to<DirectoryResult<DirectoryStats>>;
    { scanner.scan_async(path, config) } -> std::convertible_to<std::future<DirectoryResult<std::vector<FileEntry>>>>;
};

// Modern directory scanner implementation
class ModernDirectoryScanner {
private:
    // Modern atomic state
    mutable std::atomic<bool> scanning_{false};
    mutable std::atomic<uint32_t> scans_completed_{0};
    mutable std::atomic<uint64_t> files_scanned_{0};
    mutable std::shared_mutex cache_mutex_;
    std::unordered_map<std::filesystem::path, std::vector<FileEntry>> scan_cache_;
    std::unordered_map<std::filesystem::path, DirectoryStats> stats_cache_;
    
    // Modern configuration
    static constexpr size_t MAX_CACHE_SIZE = 1000;
    static constexpr std::chrono::minutes CACHE_TTL{5};
    
public:
    // Modern constructors
    constexpr ModernDirectoryScanner() noexcept = default;
    
    // Non-copyable, movable
    ModernDirectoryScanner(const ModernDirectoryScanner&) = delete;
    ModernDirectoryScanner& operator=(const ModernDirectoryScanner&) = delete;
    ModernDirectoryScanner(ModernDirectoryScanner&&) noexcept = default;
    ModernDirectoryScanner& operator=(ModernDirectoryScanner&&) noexcept = default;
    
    // Modern destructor
    ~ModernDirectoryScanner() noexcept = default;
    
    // Main directory scanning function
    [[nodiscard]] auto scan_directory(StringView path, const ScanConfig& config = {}) const -> DirectoryResult<std::vector<FileEntry>> {
        try {
            std::filesystem::path dir_path(path);
            
            // Validate path
            if (!validate_directory_path(dir_path)) {
                return std::unexpected(DirectoryError::DirectoryNotFound);
            }
            
            // Check if already scanning
            if (scanning_.load(std::memory_order_acquire)) {
                return std::unexpected(DirectoryError::ScanTimeout);
            }
            
            scanning_.store(true, std::memory_order_release);
            // Create a non-const copy for the guard
            auto* non_const_this = const_cast<ModernDirectoryScanner*>(this);
            auto scan_guard = std::unique_ptr<ModernDirectoryScanner, void(*)(ModernDirectoryScanner*)>(
                non_const_this, [](ModernDirectoryScanner* ptr) { ptr->stop_scanning(); });
            
            auto start_time = std::chrono::steady_clock::now();
            
            // Perform scan
            auto entries_result = scan_directory_internal(dir_path, config, 0, start_time);
            if (!entries_result) {
                return std::unexpected(entries_result.error());
            }
            
            // Update statistics
            scans_completed_.fetch_add(1, std::memory_order_relaxed);
            files_scanned_.fetch_add(entries_result->size(), std::memory_order_relaxed);
            
            return std::move(*entries_result);
        } catch (const std::filesystem::filesystem_error& e) {
            return std::unexpected(map_filesystem_error(e));
        } catch (const std::bad_alloc&) {
            return std::unexpected(DirectoryError::OutOfMemory);
        } catch (...) {
            return std::unexpected(DirectoryError::Unknown);
        }
    }
    
    // Get directory statistics
    [[nodiscard]] auto get_stats(StringView path, const ScanConfig& config = {}) const -> DirectoryResult<DirectoryStats> {
        try {
            std::filesystem::path dir_path(path);
            
            if (!validate_directory_path(dir_path)) {
                return std::unexpected(DirectoryError::DirectoryNotFound);
            }
            
            auto start_time = std::chrono::steady_clock::now();
            
            DirectoryStats stats{};
            
            // Scan directory and collect statistics
            auto entries_result = scan_directory_internal(dir_path, config, 0, start_time);
            if (!entries_result) {
                return std::unexpected(entries_result.error());
            }
            
            auto& entries = *entries_result;
            
            // Calculate statistics
            uint64_t total_files = 0;
            uint64_t total_directories = 0;
            uint64_t total_size = 0;
            uint64_t hidden_files = 0;
            uint64_t hidden_directories = 0;
            uint64_t symlinks = 0;
            
            for (const auto& entry : entries) {
                if (entry.is_directory) {
                    total_directories++;
                    if (entry.is_hidden) {
                        hidden_directories++;
                    }
                } else {
                    total_files++;
                    total_size += entry.size;
                    if (entry.is_hidden) {
                        hidden_files++;
                    }
                }
                
                if (entry.is_symlink) {
                    symlinks++;
                }
            }
            
            auto scan_duration = std::chrono::duration_cast<std::chrono::milliseconds>(
                std::chrono::steady_clock::now() - start_time);
            
            return DirectoryStats{
                total_files,
                total_directories,
                total_size,
                hidden_files,
                hidden_directories,
                symlinks,
                std::chrono::system_clock::now(),
                scan_duration,
                dir_path
            };
        } catch (const std::filesystem::filesystem_error& e) {
            return std::unexpected(map_filesystem_error(e));
        } catch (...) {
            return std::unexpected(DirectoryError::Unknown);
        }
    }
    
    // Async directory scanning
    [[nodiscard]] auto scan_async(StringView path, const ScanConfig& config = {}) const -> std::future<DirectoryResult<std::vector<FileEntry>>> {
        return std::async(std::launch::async, [this, path, config]() {
            return scan_directory(path, config);
        });
    }
    
    // Modern statistics
    [[nodiscard]] auto get_scans_completed() const noexcept -> uint32_t {
        return scans_completed_.load(std::memory_order_relaxed);
    }
    
    [[nodiscard]] auto get_files_scanned() const noexcept -> uint64_t {
        return files_scanned_.load(std::memory_order_relaxed);
    }
    
    // Clear cache
    auto clear_cache() -> void {
        std::unique_lock lock{cache_mutex_};
        scan_cache_.clear();
        stats_cache_.clear();
    }

private:
    // Stop scanning
    auto stop_scanning() -> void {
        scanning_.store(false, std::memory_order_release);
    }
    
    // Validate directory path
    [[nodiscard]] static auto validate_directory_path(const std::filesystem::path& path) -> bool {
        try {
            return std::filesystem::exists(path) && 
                   std::filesystem::is_directory(path);
        } catch (...) {
            return false;
        }
    }
    
    // Internal directory scanning
    [[nodiscard]] auto scan_directory_internal(
        const std::filesystem::path& path,
        const ScanConfig& config,
        uint32_t current_depth,
        std::chrono::steady_clock::time_point start_time
    ) const -> DirectoryResult<std::vector<FileEntry>> {
        
        // Check timeout
        auto elapsed = std::chrono::steady_clock::now() - start_time;
        if (elapsed > config.timeout) {
            return std::unexpected(DirectoryError::ScanTimeout);
        }
        
        // Check depth limit
        if (current_depth > config.max_depth) {
            return std::vector<FileEntry>{};
        }
        
        std::vector<FileEntry> entries;
        
        try {
            // Check file count limit
            if (files_scanned_.load(std::memory_order_relaxed) >= config.max_files) {
                return std::unexpected(DirectoryError::TooManyFiles);
            }
            
            // Iterate directory
            for (const auto& entry : std::filesystem::directory_iterator(path)) {
                // Check if scanning was stopped
                if (!scanning_.load(std::memory_order_acquire)) {
                    return std::unexpected(DirectoryError::ScanTimeout);
                }
                
                try {
                    auto file_entry = create_file_entry(entry.path(), config);
                    if (file_entry) {
                        entries.push_back(std::move(*file_entry));
                        
                        // Recursively scan subdirectories
                        if (config.recursive && file_entry->is_directory && !file_entry->is_symlink) {
                            auto sub_entries_result = scan_directory_internal(
                                entry.path(), config, current_depth + 1, start_time);
                            
                            if (sub_entries_result) {
                                auto sub_entries = std::move(*sub_entries_result);
                                entries.insert(entries.end(), 
                                           std::make_move_iterator(sub_entries.begin()),
                                           std::make_move_iterator(sub_entries.end()));
                            }
                        }
                    }
                } catch (...) {
                    // Skip problematic entries
                    continue;
                }
            }
            
            // Sort entries
            std::ranges::sort(entries, [](const auto& a, const auto& b) {
                if (a.is_directory != b.is_directory) {
                    return a.is_directory > b.is_directory; // Directories first
                }
                return a.name < b.name;
            });
            
            return entries;
            
        } catch (const std::filesystem::filesystem_error& e) {
            return std::unexpected(map_filesystem_error(e));
        } catch (...) {
            return std::unexpected(DirectoryError::Unknown);
        }
    }
    
    // Create file entry from path
    [[nodiscard]] static auto create_file_entry(
        const std::filesystem::path& path,
        const ScanConfig& config
    ) -> DirectoryResult<FileEntry> {
        
        try {
            // Check if should be excluded
            if (should_exclude(path, config.exclude_patterns)) {
                return std::unexpected(DirectoryError::InvalidPath);
            }
            
            auto filename = path.filename().string();
            
            // Check if hidden
            bool is_hidden = is_file_hidden(path, filename);
            
            // Skip hidden files if not requested
            if (!config.include_hidden && is_hidden) {
                return std::unexpected(DirectoryError::InvalidPath);
            }
            
            // Get file status
            auto status = std::filesystem::status(path);
            bool is_directory = std::filesystem::is_directory(status);
            bool is_symlink = std::filesystem::is_symlink(status);
            
            // Get file size (only for files)
            uint64_t size = 0;
            if (!is_directory) {
                try {
                    size = std::filesystem::file_size(path);
                } catch (...) {
                    // Skip if can't get size
                    size = 0;
                }
            }
            
            // Get timestamps
            auto modified_fs_time = std::filesystem::last_write_time(path);
            auto modified_time = std::chrono::system_clock::now(); // Default
            try {
                modified_time = std::chrono::time_point_cast<std::chrono::system_clock::duration>(
                    modified_fs_time - std::filesystem::file_time_type::clock::now() + std::chrono::system_clock::now());
            } catch (...) {
                // Use current time as fallback
            }
            
            // creation_time may not be available on all platforms, use modified_time as fallback
            auto created_time = modified_time;
            
            // Get file type
            std::string file_type = get_file_type(path, is_directory);
            
            // Get permissions
            std::string permissions = get_permissions_string(status.permissions());
            
            return FileEntry{
                path,
                std::move(filename),
                is_directory,
                is_symlink,
                is_hidden,
                size,
                modified_time,
                created_time,
                std::move(file_type),
                std::move(permissions)
            };
            
        } catch (const std::filesystem::filesystem_error& e) {
            return std::unexpected(map_filesystem_error(e));
        } catch (...) {
            return std::unexpected(DirectoryError::Unknown);
        }
    }
    
    // Check if file should be excluded
    [[nodiscard]] static auto should_exclude(
        const std::filesystem::path& path,
        const std::vector<std::string>& patterns
    ) -> bool {
        
        if (patterns.empty()) {
            return false;
        }
        
        auto path_str = path.string();
        
        for (const auto& pattern : patterns) {
            // Simple pattern matching (can be enhanced with regex)
            if (path_str.find(pattern) != std::string::npos) {
                return true;
            }
        }
        
        return false;
    }
    
    // Check if file is hidden
    [[nodiscard]] static auto is_file_hidden(
        const std::filesystem::path& path,
        const std::string& filename
    ) -> bool {
        
#ifdef _WIN32
        DWORD attributes = GetFileAttributesW(path.wstring().c_str());
        return (attributes & FILE_ATTRIBUTE_HIDDEN) != 0;
#else
        return filename.starts_with('.');
#endif
    }
    
    // Get file type string
    [[nodiscard]] static auto get_file_type(
        const std::filesystem::path& path,
        bool is_directory
    ) -> std::string {
        
        if (is_directory) {
            return "directory";
        }
        
        auto extension = path.extension().string();
        if (extension.empty()) {
            return "file";
        }
        
        // Convert to lowercase
        std::ranges::transform(extension, extension.begin(), ::tolower);
        
        static const std::unordered_map<std::string, std::string> file_types = {
            {".txt", "text"},
            {".html", "html"},
            {".htm", "html"},
            {".css", "css"},
            {".js", "javascript"},
            {".json", "json"},
            {".xml", "xml"},
            {".pdf", "pdf"},
            {".doc", "document"},
            {".docx", "document"},
            {".xls", "spreadsheet"},
            {".xlsx", "spreadsheet"},
            {".ppt", "presentation"},
            {".pptx", "presentation"},
            {".zip", "archive"},
            {".rar", "archive"},
            {".tar", "archive"},
            {".gz", "archive"},
            {".jpg", "image"},
            {".jpeg", "image"},
            {".png", "image"},
            {".gif", "image"},
            {".bmp", "image"},
            {".tiff", "image"},
            {".mp3", "audio"},
            {".wav", "audio"},
            {".mp4", "video"},
            {".avi", "video"},
            {".exe", "executable"},
            {".dll", "library"},
            {".so", "library"},
            {".dylib", "library"}
        };
        
        auto it = file_types.find(extension);
        if (it != file_types.end()) {
            return it->second;
        }
        
        return "file";
    }
    
    // Get permissions string
    [[nodiscard]] static auto get_permissions_string(std::filesystem::perms perms) -> std::string {
        std::string result = "rwxrwxrwx";
        
        // Convert permissions to string (simplified)
        if ((perms & std::filesystem::perms::owner_read) == std::filesystem::perms::none) {
            result[0] = '-';
        }
        if ((perms & std::filesystem::perms::owner_write) == std::filesystem::perms::none) {
            result[1] = '-';
        }
        if ((perms & std::filesystem::perms::owner_exec) == std::filesystem::perms::none) {
            result[2] = '-';
        }
        if ((perms & std::filesystem::perms::group_read) == std::filesystem::perms::none) {
            result[3] = '-';
        }
        if ((perms & std::filesystem::perms::group_write) == std::filesystem::perms::none) {
            result[4] = '-';
        }
        if ((perms & std::filesystem::perms::group_exec) == std::filesystem::perms::none) {
            result[5] = '-';
        }
        if ((perms & std::filesystem::perms::others_read) == std::filesystem::perms::none) {
            result[6] = '-';
        }
        if ((perms & std::filesystem::perms::others_write) == std::filesystem::perms::none) {
            result[7] = '-';
        }
        if ((perms & std::filesystem::perms::others_exec) == std::filesystem::perms::none) {
            result[8] = '-';
        }
        
        return result;
    }
    
    // Map filesystem error to DirectoryError
    [[nodiscard]] static auto map_filesystem_error(const std::filesystem::filesystem_error& e) -> DirectoryError {
        if (e.code() == std::errc::no_such_file_or_directory) {
            return DirectoryError::DirectoryNotFound;
        } else if (e.code() == std::errc::permission_denied) {
            return DirectoryError::PermissionDenied;
        } else if (e.code() == std::errc::filename_too_long) {
            return DirectoryError::InvalidPath;
        } else if (e.code() == std::errc::too_many_links) {
            return DirectoryError::TooManyFiles;
        } else {
            return DirectoryError::IoError;
        }
    }
};

// Modern plugin factory
template<typename T>
requires DirectoryScanner<T>
auto create_directory_scanner() -> std::unique_ptr<T> {
    return std::make_unique<T>();
}

// Modern C++23 exported functions
extern "C" {
    [[nodiscard]] const char* get_plugin_info() {
        static constexpr const char* info = "modern_directory_info_plugin:2.0.0:Modern C++23 directory scanner";
        return info;
    }
    
    [[nodiscard]] ModernDirectoryScanner* get_plugin_instance() {
        static auto scanner = std::make_unique<ModernDirectoryScanner>();
        return scanner.get();
    }
}

// Modern DLL entry point
#ifdef _WIN32
BOOL APIENTRY DllMain(HMODULE hModule, DWORD ul_reason_for_call, LPVOID lpReserved) noexcept {
    switch (ul_reason_for_call) {
        case DLL_PROCESS_ATTACH:
            DisableThreadLibraryCalls(hModule);
            break;
        case DLL_THREAD_ATTACH:
        case DLL_THREAD_DETACH:
        case DLL_PROCESS_DETACH:
            break;
    }
    return TRUE;
}
#endif
