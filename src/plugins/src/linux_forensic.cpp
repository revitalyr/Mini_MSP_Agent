#include <iostream>
#include <string>
#include <vector>
#include <map>
#include <chrono>
#include <sstream>
#include <fstream>
#include <cstdlib>
#include <memory>
#include <cstdio>
#include <array>
#include <dirent.h>
#include <sys/stat.h>
#include <unistd.h>

// Plugin interface structures
struct PluginInfo {
    std::string name;
    std::string version;
    std::string platform;
    std::string description;
};

struct ForensicData {
    std::string category;
    std::map<std::string, std::string> data;
    std::vector<std::map<std::string, std::string>> array_data;
};

// Linux-specific forensic data collector
class LinuxForensicCollector {
public:
    // /proc/ and /sys/ filesystems
    static ForensicData collectProcSys() {
        ForensicData data;
        data.category = "Proc & Sys Filesystems";
        
        // Read /proc/version
        std::ifstream version_file("/proc/version");
        if (version_file.is_open()) {
            std::string version;
            std::getline(version_file, version);
            data.data["kernel_version"] = version;
        }
        
        // Read /proc/sys/kernel/ip_forward
        std::ifstream ip_forward_file("/proc/sys/kernel/ip_forward");
        if (ip_forward_file.is_open()) {
            std::string ip_forward;
            std::getline(ip_forward_file, ip_forward);
            data.data["ip_forward"] = ip_forward;
        }
        
        // Read loaded kernel modules
        std::array<char, 128> buffer;
        std::string result;
        std::shared_ptr<FILE> pipe(popen("cat /proc/modules", "r"), pclose);
        if (pipe) {
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                result += buffer.data();
            }
            
            // Parse modules
            std::istringstream iss(result);
            std::string line;
            while (std::getline(iss, line)) {
                std::istringstream line_stream(line);
                std::string name, size, used_count, used_by;
                
                if (line_stream >> name >> size >> used_count >> used_by) {
                    std::map<std::string, std::string> module_info;
                    module_info["name"] = name;
                    module_info["size"] = size;
                    module_info["used_count"] = used_count;
                    module_info["used_by"] = used_by;
                    module_info["type"] = "kernel_module";
                    
                    data.array_data.push_back(module_info);
                }
            }
        }
        
        data.data["total_modules"] = std::to_string(data.array_data.size());
        return data;
    }
    
    // Process environment variables
    static ForensicData collectProcessEnviron() {
        ForensicData data;
        data.category = "Process Environment";
        
        // Get current process environment
        std::array<char, 128> buffer;
        std::string result;
        std::shared_ptr<FILE> pipe(popen("cat /proc/self/environ", "r"), pclose);
        
        if (pipe) {
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                std::string env_var(buffer.data());
                if (!env_var.empty() && env_var.back() == '\0') {
                    env_var.pop_back(); // Remove null terminator
                }
                
                if (!env_var.empty()) {
                    size_t pos = env_var.find('=');
                    if (pos != std::string::npos) {
                        std::string key = env_var.substr(0, pos);
                        std::string value = env_var.substr(pos + 1);
                        
                        std::map<std::string, std::string> env_info;
                        env_info["key"] = key;
                        env_info["value"] = value;
                        env_info["type"] = "environment_variable";
                        
                        data.array_data.push_back(env_info);
                    }
                }
            }
        }
        
        data.data["total_env_vars"] = std::to_string(data.array_data.size());
        return data;
    }
    
    // Systemd services
    static ForensicData collectSystemdServices() {
        ForensicData data;
        data.category = "Systemd Services";
        
        // Get running systemd services
        std::array<char, 128> buffer;
        std::string result;
        std::shared_ptr<FILE> pipe(popen("systemctl list-units --type=service --state=running", "r"), pclose);
        
        if (pipe) {
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                result += buffer.data();
            }
            
            // Parse systemd output
            std::istringstream iss(result);
            std::string line;
            bool header_skipped = false;
            
            while (std::getline(iss, line)) {
                if (!header_skipped) {
                    header_skipped = true;
                    continue;
                }
                
                if (line.empty() || line[0] == ' ') continue;
                
                std::istringstream line_stream(line);
                std::string unit, load, active, sub, description;
                
                if (line_stream >> unit >> load >> active >> sub) {
                    std::getline(line_stream, description);
                    if (!description.empty() && description[0] == ' ') {
                        description = description.substr(1);
                    }
                    
                    std::map<std::string, std::string> service_info;
                    service_info["unit"] = unit;
                    service_info["load"] = load;
                    service_info["active"] = active;
                    service_info["sub"] = sub;
                    service_info["description"] = description;
                    service_info["type"] = "systemd_service";
                    
                    data.array_data.push_back(service_info);
                }
            }
        }
        
        // Get systemd security analysis
        pipe = std::shared_ptr<FILE>(popen("systemd-analyze security", "r"), pclose);
        if (pipe) {
            result.clear();
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                result += buffer.data();
            }
            data.data["security_analysis"] = result;
        }
        
        data.data["total_services"] = std::to_string(data.array_data.size());
        return data;
    }
    
    // Cron jobs
    static ForensicData collectCronJobs() {
        ForensicData data;
        data.category = "Cron Jobs";
        
        // System crontabs
        std::vector<std::string> cron_paths = {
            "/etc/crontab",
            "/etc/cron.d/",
            "/var/spool/cron/crontabs/"
        };
        
        for (const auto& path : cron_paths) {
            struct stat st;
            if (stat(path.c_str(), &st) == 0) {
                if (S_ISDIR(st.st_mode)) {
                    // It's a directory, scan for files
                    DIR *dir = opendir(path.c_str());
                    if (dir) {
                        struct dirent *entry;
                        while ((entry = readdir(dir)) != nullptr) {
                            if (entry->d_type == DT_REG) {
                                std::string full_path = path + entry->d_name;
                                std::ifstream file(full_path);
                                if (file.is_open()) {
                                    std::string content((std::istreambuf_iterator<char>(file)),
                                                      std::istreambuf_iterator<char>());
                                    
                                    std::map<std::string, std::string> cron_info;
                                    cron_info["path"] = full_path;
                                    cron_info["content"] = content;
                                    cron_info["type"] = "cron_file";
                                    
                                    data.array_data.push_back(cron_info);
                                }
                            }
                        }
                        closedir(dir);
                    }
                } else {
                    // It's a file
                    std::ifstream file(path);
                    if (file.is_open()) {
                        std::string content((std::istreambuf_iterator<char>(file)),
                                          std::istreambuf_iterator<char>());
                        
                        std::map<std::string, std::string> cron_info;
                        cron_info["path"] = path;
                        cron_info["content"] = content;
                        cron_info["type"] = "cron_file";
                        
                        data.array_data.push_back(cron_info);
                    }
                }
            }
        }
        
        // User crontabs
        std::array<char, 128> buffer;
        std::string result;
        std::shared_ptr<FILE> pipe(popen("ls /var/spool/cron/crontabs/", "r"), pclose);
        if (pipe) {
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                std::string username(buffer.data());
                if (!username.empty() && username.back() == '\n') {
                    username.pop_back();
                }
                
                if (!username.empty() && username != "." && username != "..") {
                    std::string user_cron_path = "/var/spool/cron/crontabs/" + username;
                    std::ifstream file(user_cron_path);
                    if (file.is_open()) {
                        std::string content((std::istreambuf_iterator<char>(file)),
                                          std::istreambuf_iterator<char>());
                        
                        std::map<std::string, std::string> cron_info;
                        cron_info["path"] = user_cron_path;
                        cron_info["user"] = username;
                        cron_info["content"] = content;
                        cron_info["type"] = "user_cron";
                        
                        data.array_data.push_back(cron_info);
                    }
                }
            }
        }
        
        data.data["total_cron_jobs"] = std::to_string(data.array_data.size());
        return data;
    }
    
    // Package manager verification
    static ForensicData collectPackageManager() {
        ForensicData data;
        data.category = "Package Manager";
        
        // Check for dpkg (Debian/Ubuntu)
        std::array<char, 128> buffer;
        std::string result;
        std::shared_ptr<FILE> pipe(popen("which dpkg", "r"), pclose);
        
        if (pipe) {
            result.clear();
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                result += buffer.data();
            }
            
            if (!result.empty()) {
                // dpkg found, verify packages
                pipe = std::shared_ptr<FILE>(popen("dpkg --verify 2>&1 | head -20", "r"), pclose);
                if (pipe) {
                    result.clear();
                    while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                        result += buffer.data();
                    }
                    data.data["package_manager"] = "dpkg";
                    data.data["verification_results"] = result;
                }
            }
        }
        
        // Check for rpm (RedHat/CentOS)
        pipe = std::shared_ptr<FILE>(popen("which rpm", "r"), pclose);
        if (pipe) {
            result.clear();
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                result += buffer.data();
            }
            
            if (!result.empty()) {
                // rpm found, verify packages
                pipe = std::shared_ptr<FILE>(popen("rpm -Va 2>&1 | head -20", "r"), pclose);
                if (pipe) {
                    result.clear();
                    while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                        result += buffer.data();
                    }
                    data.data["package_manager"] = "rpm";
                    data.data["verification_results"] = result;
                }
            }
        }
        
        return data;
    }
    
    // Security attributes (SELinux/AppArmor)
    static ForensicData collectSecurityAttributes() {
        ForensicData data;
        data.category = "Security Attributes";
        
        // Check SELinux
        std::array<char, 128> buffer;
        std::string result;
        std::shared_ptr<FILE> pipe(popen("which getenforce", "r"), pclose);
        
        if (pipe) {
            result.clear();
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                result += buffer.data();
            }
            
            if (!result.empty()) {
                pipe = std::shared_ptr<FILE>(popen("getenforce", "r"), pclose);
                if (pipe) {
                    result.clear();
                    while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                        result += buffer.data();
                    }
                    data.data["selinux_status"] = result;
                }
            }
        }
        
        // Check AppArmor
        pipe = std::shared_ptr<FILE>(popen("which aa-status", "r"), pclose);
        if (pipe) {
            result.clear();
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                result += buffer.data();
            }
            
            if (!result.empty()) {
                pipe = std::shared_ptr<FILE>(popen("aa-status", "r"), pclose);
                if (pipe) {
                    result.clear();
                    while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                        result += buffer.data();
                    }
                    data.data["apparmor_status"] = result;
                }
            }
        }
        
        return data;
    }
    
    // System Information
    static ForensicData collectSystemInfo() {
        ForensicData data;
        data.category = "System Information";
        
        // Get distribution info
        std::ifstream os_release("/etc/os-release");
        if (os_release.is_open()) {
            std::string line;
            while (std::getline(os_release, line)) {
                if (line.find("PRETTY_NAME=") == 0) {
                    data.data["distribution"] = line.substr(12); // Remove PRETTY_NAME=
                    if (data.data["distribution"].front() == '"' && data.data["distribution"].back() == '"') {
                        data.data["distribution"] = data.data["distribution"].substr(1, data.data["distribution"].length() - 2);
                    }
                    break;
                }
            }
        }
        
        // Get hostname
        char hostname[256];
        if (gethostname(hostname, sizeof(hostname)) == 0) {
            data.data["hostname"] = hostname;
        }
        
        // Get current user
        struct passwd *pw = getpwuid(getuid());
        if (pw) {
            data.data["current_user"] = pw->pw_name;
            data.data["current_user_home"] = pw->pw_dir;
        }
        
        return data;
    }
};

// Plugin implementation
extern "C" {
    const char* get_plugin_info() {
        snprintf(plugin_info_buffer, sizeof(plugin_info_buffer),
                 "%s:%s:%s:%s", "linux_forensic", "1.0.0", "Linux",
                 "Linux-specific forensic data collection plugin");
        return plugin_info_buffer;
    }
    
    bool plugin_initialize() {
        // Initialize plugin
        return true;
    }
    
    void plugin_cleanup() {
        // Cleanup plugin
    }
    
    std::vector<ForensicData> collect_linux_forensic_data() {
        std::vector<ForensicData> forensic_data;
        
        // Collect Linux-specific forensic data
        forensic_data.push_back(LinuxForensicCollector::collectSystemInfo());
        forensic_data.push_back(LinuxForensicCollector::collectProcSys());
        forensic_data.push_back(LinuxForensicCollector::collectProcessEnviron());
        forensic_data.push_back(LinuxForensicCollector::collectSystemdServices());
        forensic_data.push_back(LinuxForensicCollector::collectCronJobs());
        forensic_data.push_back(LinuxForensicCollector::collectPackageManager());
        forensic_data.push_back(LinuxForensicCollector::collectSecurityAttributes());
        
        return forensic_data;
    }
    
    std::string get_linux_forensic_report() {
        auto forensic_data = collect_linux_forensic_data();
        std::stringstream report;
        
        report << "{\n";
        report << "  \"linux_forensic_report\": {\n";
        report << "    \"collection_time\": \"" << std::chrono::duration_cast<std::chrono::seconds>(
            std::chrono::system_clock::now().time_since_epoch()).count() << "\",\n";
        report << "    \"categories\": [\n";
        
        for (size_t i = 0; i < forensic_data.size(); i++) {
            const auto& category = forensic_data[i];
            report << "      {\n";
            report << "        \"name\": \"" << category.category << "\",\n";
            report << "        \"data\": {\n";
            
            for (const auto& pair : category.data) {
                report << "          \"" << pair.first << "\": \"" << pair.second << "\"";
                if (&pair != &*category.data.rbegin()) report << ",";
                report << "\n";
            }
            
            report << "        },\n";
            report << "        \"array_data\": [\n";
            
            for (size_t j = 0; j < category.array_data.size(); j++) {
                const auto& item = category.array_data[j];
                report << "          {\n";
                
                for (const auto& pair : item) {
                    report << "            \"" << pair.first << "\": \"" << pair.second << "\"";
                    if (&pair != &*item.rbegin()) report << ",";
                    report << "\n";
                }
                
                report << "          }";
                if (j < category.array_data.size() - 1) report << ",";
                report << "\n";
            }
            
            report << "        ]\n";
            report << "      }";
            if (i < forensic_data.size() - 1) report << ",";
            report << "\n";
        }
        
        report << "    ]\n";
        report << "  }\n";
        report << "}\n";
        
        return report.str();
    }
}
