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

// macOS-specific includes
#include <sys/types.h>
#include <sys/sysctl.h>
#include <unistd.h>
#include <pwd.h>
#include <uuid/uuid.h>

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

// macOS-specific forensic data collector
class MacOSForensicCollector {
public:
    // Launch Agents and Daemons
    static ForensicData collectLaunchAgentsDaemons() {
        ForensicData data;
        data.category = "Launch Agents & Daemons";
        
        // Get launchctl list output
        std::array<char, 128> buffer;
        std::string result;
        std::shared_ptr<FILE> pipe(popen("launchctl list", "r"), pclose);
        if (!pipe) {
            data.data["error"] = "Failed to execute launchctl list";
            return data;
        }
        
        while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
            result += buffer.data();
        }
        
        // Parse launchctl output
        std::istringstream iss(result);
        std::string line;
        while (std::getline(iss, line)) {
            if (line.empty() || line[0] == '#') continue;
            
            std::istringstream line_stream(line);
            std::string pid_str, exit_code_str, label;
            
            if (line_stream >> pid_str >> exit_code_str >> std::ws) {
                std::getline(line_stream, label);
                
                std::map<std::string, std::string> service_info;
                service_info["pid"] = (pid_str == "-") ? "0" : pid_str;
                service_info["exit_code"] = exit_code_str;
                service_info["label"] = label;
                service_info["status"] = (pid_str == "-") ? "not_running" : "running";
                
                data.array_data.push_back(service_info);
            }
        }
        
        data.data["total_services"] = std::to_string(data.array_data.size());
        return data;
    }
    
    // Gatekeeper and Code Signing
    static ForensicData collectGatekeeperSigning() {
        ForensicData data;
        data.category = "Gatekeeper & Code Signing";
        
        // Get system integrity protection status
        std::array<char, 128> buffer;
        std::string result;
        std::shared_ptr<FILE> pipe(popen("csrutil status", "r"), pclose);
        if (pipe) {
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                result += buffer.data();
            }
            data.data["sip_status"] = result;
        }
        
        // Check Gatekeeper status
        pipe = std::shared_ptr<FILE>(popen("spctl --status", "r"), pclose);
        if (pipe) {
            result.clear();
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                result += buffer.data();
            }
            data.data["gatekeeper_status"] = result;
        }
        
        data.data["collection_method"] = "macOS native commands";
        return data;
    }
    
    // Quarantine Events
    static ForensicData collectQuarantineEvents() {
        ForensicData data;
        data.category = "Quarantine Events";
        
        // Try to read quarantine database
        std::string quarantine_db = std::getenv("HOME") + std::string("/Library/Preferences/com.apple.LaunchServices.QuarantineEventsV2");
        std::ifstream file(quarantine_db);
        
        if (file.is_open()) {
            data.data["quarantine_db_path"] = quarantine_db;
            data.data["quarantine_db_accessible"] = "true";
            
            // Read file size (binary file, so just report size)
            file.seekg(0, std::ios::end);
            size_t size = file.tellg();
            data.data["quarantine_db_size"] = std::to_string(size);
        } else {
            data.data["quarantine_db_accessible"] = "false";
            data.data["quarantine_db_path"] = quarantine_db;
        }
        
        return data;
    }
    
    // Kernel Extensions
    static ForensicData collectKernelExtensions() {
        ForensicData data;
        data.category = "Kernel Extensions";
        
        // Get kextstat output
        std::array<char, 128> buffer;
        std::string result;
        std::shared_ptr<FILE> pipe(popen("kextstat", "r"), pclose);
        if (!pipe) {
            data.data["error"] = "Failed to execute kextstat";
            return data;
        }
        
        while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
            result += buffer.data();
        }
        
        // Parse kextstat output
        std::istringstream iss(result);
        std::string line;
        bool header_skipped = false;
        
        while (std::getline(iss, line)) {
            if (!header_skipped) {
                header_skipped = true;
                continue;
            }
            
            std::istringstream line_stream(line);
            std::string index, ref_count, size, wired, name, version, address;
            
            if (line_stream >> index >> ref_count >> size >> wired >> address >> std::ws) {
                std::getline(line_stream, name);
                
                std::map<std::string, std::string> kext_info;
                kext_info["index"] = index;
                kext_info["ref_count"] = ref_count;
                kext_info["size"] = size;
                kext_info["wired"] = wired;
                kext_info["address"] = address;
                kext_info["name"] = name;
                kext_info["type"] = "kernel_extension";
                
                data.array_data.push_back(kext_info);
            }
        }
        
        data.data["total_kexts"] = std::to_string(data.array_data.size());
        return data;
    }
    
    // Managed Settings (Profiles)
    static ForensicData collectManagedSettings() {
        ForensicData data;
        data.category = "Managed Settings";
        
        // Get profiles list
        std::array<char, 128> buffer;
        std::string result;
        std::shared_ptr<FILE> pipe(popen("profiles -L", "r"), pclose);
        if (!pipe) {
            data.data["error"] = "Failed to execute profiles -L";
            return data;
        }
        
        while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
            result += buffer.data();
        }
        
        data.data["profiles_output"] = result;
        
        // Parse profiles
        std::istringstream iss(result);
        std::string line;
        std::string current_profile;
        int profile_count = 0;
        
        while (std::getline(iss, line)) {
            if (line.find("com.apple.") == 0 || line.find("profile.") == 0) {
                if (!current_profile.empty()) {
                    profile_count++;
                    std::map<std::string, std::string> profile_info;
                    profile_info["identifier"] = current_profile;
                    profile_info["type"] = "configuration_profile";
                    data.array_data.push_back(profile_info);
                }
                current_profile = line;
            }
        }
        
        if (!current_profile.empty()) {
            profile_count++;
            std::map<std::string, std::string> profile_info;
            profile_info["identifier"] = current_profile;
            profile_info["type"] = "configuration_profile";
            data.array_data.push_back(profile_info);
        }
        
        data.data["total_profiles"] = std::to_string(profile_count);
        return data;
    }
    
    // Unified Logging
    static ForensicData collectUnifiedLogging() {
        ForensicData data;
        data.category = "Unified Logging";
        
        // Get recent log entries
        std::array<char, 128> buffer;
        std::string result;
        std::shared_ptr<FILE> pipe(popen("log show --last 5m --predicate 'subsystem == \"com.apple.launchd\"'", "r"), pclose);
        
        if (pipe) {
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                result += buffer.data();
            }
            data.data["recent_launchd_logs"] = result;
        }
        
        // Get log stream info
        pipe = std::shared_ptr<FILE>(popen("log stream --predicate 'subsystem == \"com.apple.launchd\"' --info --debug", "r"), pclose);
        if (pipe) {
            result.clear();
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                result += buffer.data();
            }
            data.data["log_stream_info"] = result;
        }
        
        data.data["collection_time"] = std::to_string(std::chrono::duration_cast<std::chrono::seconds>(
            std::chrono::system_clock::now().time_since_epoch()).count());
        
        return data;
    }
    
    // System Information
    static ForensicData collectSystemInfo() {
        ForensicData data;
        data.category = "System Information";
        
        // Get macOS version
        std::array<char, 128> buffer;
        std::string result;
        std::shared_ptr<FILE> pipe(popen("sw_vers", "r"), pclose);
        if (pipe) {
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                result += buffer.data();
            }
            data.data["macos_version"] = result;
        }
        
        // Get hardware info
        pipe = std::shared_ptr<FILE>(popen("system_profiler SPHardwareDataType", "r"), pclose);
        if (pipe) {
            result.clear();
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                result += buffer.data();
            }
            data.data["hardware_info"] = result;
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
    PluginInfo get_plugin_info() {
        return {
            "macos_forensic",
            "1.0.0",
            "macOS",
            "macOS-specific forensic data collection plugin"
        };
    }
    
    bool plugin_initialize() {
        // Initialize plugin
        return true;
    }
    
    void plugin_cleanup() {
        // Cleanup plugin
    }
    
    std::vector<ForensicData> collect_macos_forensic_data() {
        std::vector<ForensicData> forensic_data;
        
        // Collect macOS-specific forensic data
        forensic_data.push_back(MacOSForensicCollector::collectSystemInfo());
        forensic_data.push_back(MacOSForensicCollector::collectLaunchAgentsDaemons());
        forensic_data.push_back(MacOSForensicCollector::collectGatekeeperSigning());
        forensic_data.push_back(MacOSForensicCollector::collectQuarantineEvents());
        forensic_data.push_back(MacOSForensicCollector::collectKernelExtensions());
        forensic_data.push_back(MacOSForensicCollector::collectManagedSettings());
        forensic_data.push_back(MacOSForensicCollector::collectUnifiedLogging());
        
        return forensic_data;
    }
    
    std::string get_macos_forensic_report() {
        auto forensic_data = collect_macos_forensic_data();
        std::stringstream report;
        
        report << "{\n";
        report << "  \"macos_forensic_report\": {\n";
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
