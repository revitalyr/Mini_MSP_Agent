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

#ifdef _WIN32
#include <windows.h>
#include <winreg.h>
#include <psapi.h>
#include <tlhelp32.h>
#include <sddl.h>
#pragma comment(lib, "advapi32.lib")
#pragma comment(lib, "psapi.lib")
#endif

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

// Windows-specific forensic data collector
class WindowsForensicCollector {
public:
#ifdef _WIN32
    // Registry persistence points
    static ForensicData collectRegistryPersistence() {
        ForensicData data;
        data.category = "Registry Persistence";
        
        // Check Run keys
        std::vector<std::string> run_keys = {
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\RunOnce",
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\Explorer\\Run",
            "SOFTWARE\\Wow6432Node\\Microsoft\\Windows\\CurrentVersion\\Run",
            "SOFTWARE\\Wow6432Node\\Microsoft\\Windows\\CurrentVersion\\RunOnce"
        };
        
        for (const auto& key_path : run_keys) {
            HKEY hKey;
            if (RegOpenKeyEx(HKEY_LOCAL_MACHINE, key_path.c_str(), 0, KEY_READ, &hKey) == ERROR_SUCCESS) {
                DWORD index = 0;
                char value_name[256];
                DWORD value_name_size = sizeof(value_name);
                DWORD value_type;
                BYTE value_data[1024];
                DWORD value_data_size = sizeof(value_data);
                
                while (RegEnumValue(hKey, index, value_name, &value_name_size, NULL, &value_type, value_data, &value_data_size) == ERROR_SUCCESS) {
                    std::map<std::string, std::string> entry_info;
                    entry_info["key_path"] = key_path;
                    entry_info["value_name"] = value_name;
                    
                    if (value_type == REG_SZ) {
                        entry_info["value_data"] = std::string(reinterpret_cast<char*>(value_data));
                        entry_info["value_type"] = "REG_SZ";
                    } else if (value_type == REG_EXPAND_SZ) {
                        entry_info["value_data"] = std::string(reinterpret_cast<char*>(value_data));
                        entry_info["value_type"] = "REG_EXPAND_SZ";
                    }
                    
                    entry_info["type"] = "registry_persistence";
                    data.array_data.push_back(entry_info);
                    
                    value_name_size = sizeof(value_name);
                    value_data_size = sizeof(value_data);
                    index++;
                }
                RegCloseKey(hKey);
            }
        }
        
        // Check Winlogon shell
        HKEY hKey;
        if (RegOpenKeyEx(HKEY_LOCAL_MACHINE, "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon", 0, KEY_READ, &hKey) == ERROR_SUCCESS) {
            char shell_value[256];
            DWORD shell_size = sizeof(shell_value);
            if (RegQueryValueEx(hKey, "Shell", NULL, NULL, (LPBYTE)shell_value, &shell_size) == ERROR_SUCCESS) {
                data.data["winlogon_shell"] = shell_value;
            }
            RegCloseKey(hKey);
        }
        
        data.data["total_persistence_entries"] = std::to_string(data.array_data.size());
        return data;
    }
    
    // Services information
    static ForensicData collectServices() {
        ForensicData data;
        data.category = "Services";
        
        // Get service list
        SC_HANDLE hSCManager = OpenSCManager(NULL, NULL, SC_MANAGER_ENUMERATE_SERVICE);
        if (hSCManager) {
            DWORD bytesNeeded, servicesReturned, resumeHandle = 0;
            
            if (EnumServicesStatusEx(hSCManager, SC_MANAGER_ENUMERATE_SERVICE, SERVICE_WIN32_OWN_PROCESS | SERVICE_WIN32_SHARE_PROCESS,
                                   SERVICE_STATE_ALL, NULL, 0, &bytesNeeded, &servicesReturned, &resumeHandle, NULL) == ERROR_MORE_DATA) {
                
                std::vector<BYTE> buffer(bytesNeeded);
                if (EnumServicesStatusEx(hSCManager, SC_MANAGER_ENUMERATE_SERVICE, SERVICE_WIN32_OWN_PROCESS | SERVICE_WIN32_SHARE_PROCESS,
                                       SERVICE_STATE_ALL, buffer.data(), bytesNeeded, &bytesNeeded, &servicesReturned, &resumeHandle, NULL) == ERROR_SUCCESS) {
                    
                    ENUM_SERVICE_STATUS_PROCESS* services = reinterpret_cast<ENUM_SERVICE_STATUS_PROCESS*>(buffer.data());
                    
                    for (DWORD i = 0; i < servicesReturned; i++) {
                        std::map<std::string, std::string> service_info;
                        service_info["service_name"] = services[i].lpServiceName;
                        service_info["display_name"] = services[i].lpDisplayName;
                        service_info["service_type"] = std::to_string(services[i].ServiceStatusProcess.dwServiceType);
                        service_info["current_state"] = std::to_string(services[i].ServiceStatusProcess.dwCurrentState);
                        service_info["controls_accepted"] = std::to_string(services[i].ServiceStatusProcess.dwControlsAccepted);
                        service_info["win32_exit_code"] = std::to_string(services[i].ServiceStatusProcess.dwWin32ExitCode);
                        service_info["service_specific_exit_code"] = std::to_string(services[i].ServiceStatusProcess.dwServiceSpecificExitCode);
                        service_info["check_point"] = std::to_string(services[i].ServiceStatusProcess.dwCheckPoint);
                        service_info["wait_hint"] = std::to_string(services[i].ServiceStatusProcess.dwWaitHint);
                        service_info["process_id"] = std::to_string(services[i].ServiceStatusProcess.dwProcessId);
                        service_info["service_flags"] = std::to_string(services[i].ServiceStatusProcess.dwServiceFlags);
                        
                        // Check if binary exists
                        if (services[i].lpServiceName) {
                            SC_HANDLE hService = OpenService(hSCManager, services[i].lpServiceName, SERVICE_QUERY_CONFIG);
                            if (hService) {
                                QUERY_SERVICE_CONFIG* config = nullptr;
                                DWORD configSize = 0;
                                
                                QueryServiceConfig(hService, nullptr, 0, &configSize);
                                if (configSize > 0) {
                                    std::vector<BYTE> configBuffer(configSize);
                                    if (QueryServiceConfig(hService, reinterpret_cast<QUERY_SERVICE_CONFIG*>(configBuffer.data()), configSize, &configSize)) {
                                        QUERY_SERVICE_CONFIG* serviceConfig = reinterpret_cast<QUERY_SERVICE_CONFIG*>(configBuffer.data());
                                        if (serviceConfig->lpBinaryPathName) {
                                            service_info["binary_path"] = serviceConfig->lpBinaryPathName;
                                            
                                            // Check if file exists
                                            DWORD fileAttribs = GetFileAttributes(serviceConfig->lpBinaryPathName);
                                            service_info["binary_exists"] = (fileAttribs != INVALID_FILE_ATTRIBUTES) ? "true" : "false";
                                        }
                                    }
                                }
                                CloseServiceHandle(hService);
                            }
                        }
                        
                        service_info["type"] = "windows_service";
                        data.array_data.push_back(service_info);
                    }
                }
            }
            CloseServiceHandle(hSCManager);
        }
        
        data.data["total_services"] = std::to_string(data.array_data.size());
        return data;
    }
    
    // Critical registry keys
    static ForensicData collectCriticalRegistry() {
        ForensicData data;
        data.category = "Critical Registry";
        
        // Check LSA authentication packages
        HKEY hKey;
        if (RegOpenKeyEx(HKEY_LOCAL_MACHINE, "SYSTEM\\CurrentControlSet\\Control\\Lsa", 0, KEY_READ, &hKey) == ERROR_SUCCESS) {
            char auth_packages[1024];
            DWORD auth_size = sizeof(auth_packages);
            if (RegQueryValueEx(hKey, "Authentication Packages", NULL, NULL, (LPBYTE)auth_packages, &auth_size) == ERROR_SUCCESS) {
                data.data["lsa_authentication_packages"] = auth_packages;
            }
            RegCloseKey(hKey);
        }
        
        // Check security packages
        if (RegOpenKeyEx(HKEY_LOCAL_MACHINE, "SYSTEM\\CurrentControlSet\\Control\\Lsa", 0, KEY_READ, &hKey) == ERROR_SUCCESS) {
            char sec_packages[1024];
            DWORD sec_size = sizeof(sec_packages);
            if (RegQueryValueEx(hKey, "Security Packages", NULL, NULL, (LPBYTE)sec_packages, &sec_size) == ERROR_SUCCESS) {
                data.data["lsa_security_packages"] = sec_packages;
            }
            RegCloseKey(hKey);
        }
        
        // Check Image File Execution Options
        if (RegOpenKeyEx(HKEY_LOCAL_MACHINE, "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Image File Execution Options", 0, KEY_READ, &hKey) == ERROR_SUCCESS) {
            DWORD index = 0;
            char value_name[256];
            DWORD value_name_size = sizeof(value_name);
            
            while (RegEnumKeyEx(hKey, index, value_name, &value_name_size, NULL, NULL, NULL, NULL) == ERROR_SUCCESS) {
                std::map<std::string, std::string> ifeo_info;
                ifeo_info["executable"] = value_name;
                ifeo_info["type"] = "image_file_execution_option";
                data.array_data.push_back(ifeo_info);
                
                value_name_size = sizeof(value_name);
                index++;
            }
            RegCloseKey(hKey);
        }
        
        data.data["total_ifeo_entries"] = std::to_string(data.array_data.size());
        return data;
    }
    
    // AmCache and installed applications
    static ForensicData collectAmCache() {
        ForensicData data;
        data.category = "AmCache & Applications";
        
        // Check AmCache registry
        HKEY hKey;
        if (RegOpenKeyEx(HKEY_LOCAL_MACHINE, "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\AppModel\\StateRepository\\Cache", 0, KEY_READ, &hKey) == ERROR_SUCCESS) {
            DWORD index = 0;
            char subkey_name[256];
            DWORD subkey_size = sizeof(subkey_name);
            
            while (RegEnumKeyEx(hKey, index, subkey_name, &subkey_size, NULL, NULL, NULL, NULL) == ERROR_SUCCESS) {
                std::map<std::string, std::string> amcache_info;
                amcache_info["cache_id"] = subkey_name;
                amcache_info["type"] = "amcache_entry";
                data.array_data.push_back(amcache_info);
                
                subkey_size = sizeof(subkey_name);
                index++;
            }
            RegCloseKey(hKey);
        }
        
        // Get installed programs from registry
        std::vector<std::string> uninstall_keys = {
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
            "SOFTWARE\\Wow6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall"
        };
        
        for (const auto& key_path : uninstall_keys) {
            HKEY hUninstallKey;
            if (RegOpenKeyEx(HKEY_LOCAL_MACHINE, key_path.c_str(), 0, KEY_READ, &hUninstallKey) == ERROR_SUCCESS) {
                DWORD index = 0;
                char subkey_name[256];
                DWORD subkey_size = sizeof(subkey_name);
                
                while (RegEnumKeyEx(hUninstallKey, index, subkey_name, &subkey_size, NULL, NULL, NULL, NULL) == ERROR_SUCCESS) {
                    HKEY hProgramKey;
                    if (RegOpenKeyEx(hUninstallKey, subkey_name, 0, KEY_READ, &hProgramKey) == ERROR_SUCCESS) {
                        char display_name[256];
                        DWORD name_size = sizeof(display_name);
                        if (RegQueryValueEx(hProgramKey, "DisplayName", NULL, NULL, (LPBYTE)display_name, &name_size) == ERROR_SUCCESS) {
                            std::map<std::string, std::string> program_info;
                            program_info["display_name"] = display_name;
                            program_info["key_name"] = subkey_name;
                            program_info["type"] = "installed_program";
                            data.array_data.push_back(program_info);
                        }
                        RegCloseKey(hProgramKey);
                    }
                    
                    subkey_size = sizeof(subkey_name);
                    index++;
                }
                RegCloseKey(hUninstallKey);
            }
        }
        
        data.data["total_amcache_entries"] = std::to_string(data.array_data.size());
        return data;
    }
    
    // NetBIOS and SMB information
    static ForensicData collectNetBiosSMB() {
        ForensicData data;
        data.category = "NetBIOS & SMB";
        
        // Get NetBIOS cache
        std::array<char, 128> buffer;
        std::string result;
        std::shared_ptr<FILE> pipe(popen("nbtstat -c", "r"), pclose);
        
        if (pipe) {
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                result += buffer.data();
            }
            data.data["netbios_cache"] = result;
        }
        
        // Get SMB sessions
        pipe = std::shared_ptr<FILE>(popen("net session", "r"), pclose);
        if (pipe) {
            result.clear();
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                result += buffer.data();
            }
            data.data["smb_sessions"] = result;
        }
        
        // Get SMB open files
        pipe = std::shared_ptr<FILE>(popen("net file", "r"), pclose);
        if (pipe) {
            result.clear();
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                result += buffer.data();
            }
            data.data["smb_open_files"] = result;
        }
        
        return data;
    }
    
    // Local policies and events
    static ForensicData collectLocalPolicies() {
        ForensicData data;
        data.category = "Local Policies & Events";
        
        // Get audit policy
        std::array<char, 128> buffer;
        std::string result;
        std::shared_ptr<FILE> pipe(popen("auditpol /get /category:*", "r"), pclose);
        
        if (pipe) {
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                result += buffer.data();
            }
            data.data["audit_policy"] = result;
        }
        
        // Get local policy settings
        pipe = std::shared_ptr<FILE>(popen("secedit /export /cfg temp_policy.cfg", "r"), pclose);
        if (pipe) {
            result.clear();
            while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
                result += buffer.data();
            }
            data.data["local_policy_export"] = result;
            
            // Clean up temp file
            system("del temp_policy.cfg 2>nul");
        }
        
        return data;
    }
    
    // System Information
    static ForensicData collectSystemInfo() {
        ForensicData data;
        data.category = "System Information";
        
        // Get Windows version
        OSVERSIONINFOEX osvi;
        osvi.dwOSVersionInfoSize = sizeof(OSVERSIONINFOEX);
        if (GetVersionEx((OSVERSIONINFO*)&osvi)) {
            data.data["windows_version"] = std::to_string(osvi.dwMajorVersion) + "." + std::to_string(osvi.dwMinorVersion) + "." + std::to_string(osvi.dwBuildNumber);
        }
        
        // Get computer name
        char computer_name[256];
        DWORD size = sizeof(computer_name);
        if (GetComputerName(computer_name, &size)) {
            data.data["computer_name"] = computer_name;
        }
        
        // Get current user
        DWORD username_size = UNLEN + 1;
        char username[UNLEN + 1];
        if (GetUserName(username, &username_size)) {
            data.data["current_user"] = username;
        }
        
        // Get system directory
        char system_dir[MAX_PATH];
        if (GetSystemDirectory(system_dir, MAX_PATH)) {
            data.data["system_directory"] = system_dir;
        }
        
        return data;
    }
#else
    // Stub implementations for non-Windows platforms
    static ForensicData collectRegistryPersistence() {
        ForensicData data;
        data.category = "Registry Persistence";
        data.data["error"] = "Not supported on this platform";
        return data;
    }
    
    static ForensicData collectServices() {
        ForensicData data;
        data.category = "Services";
        data.data["error"] = "Not supported on this platform";
        return data;
    }
    
    static ForensicData collectCriticalRegistry() {
        ForensicData data;
        data.category = "Critical Registry";
        data.data["error"] = "Not supported on this platform";
        return data;
    }
    
    static ForensicData collectAmCache() {
        ForensicData data;
        data.category = "AmCache & Applications";
        data.data["error"] = "Not supported on this platform";
        return data;
    }
    
    static ForensicData collectNetBiosSMB() {
        ForensicData data;
        data.category = "NetBIOS & SMB";
        data.data["error"] = "Not supported on this platform";
        return data;
    }
    
    static ForensicData collectLocalPolicies() {
        ForensicData data;
        data.category = "Local Policies & Events";
        data.data["error"] = "Not supported on this platform";
        return data;
    }
    
    static ForensicData collectSystemInfo() {
        ForensicData data;
        data.category = "System Information";
        data.data["error"] = "Not supported on this platform";
        return data;
    }
#endif
};

// Plugin implementation
extern "C" {
    PluginInfo get_plugin_info() {
        return {
            "windows_forensic",
            "1.0.0",
            "Windows",
            "Windows-specific forensic data collection plugin"
        };
    }
    
    bool plugin_initialize() {
        // Initialize plugin
        return true;
    }
    
    void plugin_cleanup() {
        // Cleanup plugin
    }
    
    std::vector<ForensicData> collect_windows_forensic_data() {
        std::vector<ForensicData> forensic_data;
        
        // Collect Windows-specific forensic data
        forensic_data.push_back(WindowsForensicCollector::collectSystemInfo());
        forensic_data.push_back(WindowsForensicCollector::collectRegistryPersistence());
        forensic_data.push_back(WindowsForensicCollector::collectServices());
        forensic_data.push_back(WindowsForensicCollector::collectCriticalRegistry());
        forensic_data.push_back(WindowsForensicCollector::collectAmCache());
        forensic_data.push_back(WindowsForensicCollector::collectNetBiosSMB());
        forensic_data.push_back(WindowsForensicCollector::collectLocalPolicies());
        
        return forensic_data;
    }
    
    std::string get_windows_forensic_report() {
        auto forensic_data = collect_windows_forensic_data();
        std::stringstream report;
        
        report << "{\n";
        report << "  \"windows_forensic_report\": {\n";
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
