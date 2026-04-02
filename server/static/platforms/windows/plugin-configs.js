// Windows-specific plugin configurations
const windowsPluginConfigs = {
    directory_info: {
        targetPath: 'C:\\Users\\Public\\Documents',
        defaultPaths: [
            'C:\\Users\\Public\\Documents',
            'C:\\Users\\Public\\Downloads',
            'C:\\Program Files',
            'C:\\Program Files (x86)',
            'C:\\Windows\\System32',
            'C:\\Windows\\Temp',
            'D:\\'
        ]
    },
    event_data: {
        monitorPath: 'C:\\Windows\\System32\\LogFiles',
        defaultPaths: [
            'C:\\Windows\\System32\\LogFiles',
            'C:\\Windows\\Temp',
            'C:\\ProgramData',
            'C:\\Users\\Public\\Documents',
            'C:\\Windows\\System32\\winevt\\Logs'
        ]
    },
    file_reader: {
        targetFile: 'C:\\Users\\Public\\Documents\\config.txt',
        defaultFiles: [
            'C:\\Users\\Public\\Documents\\config.txt',
            'C:\\Users\\Public\\Documents\\data.csv',
            'C:\\Windows\\System32\\drivers\\etc\\hosts',
            'C:\\ProgramData\\log.txt',
            'C:\\Users\\Public\\Desktop\\readme.txt'
        ]
    },
    file_signature: {
        targetPath: 'C:\\Users\\Public\\Documents',
        defaultPaths: [
            'C:\\Users\\Public\\Documents',
            'C:\\Users\\Public\\Downloads',
            'C:\\Program Files',
            'C:\\Windows\\System32'
        ]
    },
    folder_watcher: {
        targetPath: 'C:\\Users\\Public\\Documents',
        defaultPaths: [
            'C:\\Users\\Public\\Documents',
            'C:\\Users\\Public\\Downloads',
            'C:\\Windows\\Temp',
            'C:\\ProgramData'
        ]
    },
    watchers_manager: {
        watchPaths: ['C:\\Users\\Public\\Documents', 'C:\\Users\\Public\\Downloads'],
        defaultPaths: [
            'C:\\Users\\Public\\Documents',
            'C:\\Users\\Public\\Downloads',
            'C:\\Program Files',
            'C:\\Windows\\Temp',
            'C:\\ProgramData'
        ]
    },
    plugin_registry: {
        pluginDirectory: 'C:\\Program Files\\Mini MSP Agent\\plugins',
        defaultPaths: [
            'C:\\Program Files\\Mini MSP Agent\\plugins',
            'C:\\Program Files (x86)\\Mini MSP Agent\\plugins',
            'C:\\Users\\Public\\Documents\\Mini MSP Agent\\plugins',
            'D:\\Mini MSP Agent\\plugins'
        ]
    }
};

// Windows-specific system events
const windowsSystemEvents = {
    fileEvents: [
        { key: 'fileCreated', label: '📄 File Created', enabled: true },
        { key: 'fileDeleted', label: '🗑️ File Deleted', enabled: true },
        { key: 'fileModified', label: '✏️ File Modified', enabled: true },
        { key: 'fileRenamed', label: '🏷️ File Renamed', enabled: false },
        { key: 'fileAccessed', label: '👁️ File Accessed', enabled: false },
        { key: 'filePermissionsChanged', label: '🔐 Permissions Changed', enabled: true },
        { key: 'fileAttributesChanged', label: '📝 Attributes Changed', enabled: false },
        { key: 'fileSecurityChanged', label: '🛡️ Security Changed', enabled: false }
    ],
    directoryEvents: [
        { key: 'directoryCreated', label: '📁 Directory Created', enabled: false },
        { key: 'directoryDeleted', label: '🗂️ Directory Deleted', enabled: false },
        { key: 'directoryModified', label: '📝 Directory Modified', enabled: false },
        { key: 'directoryAccessed', label: '👁️ Directory Accessed', enabled: false }
    ],
    systemEvents: [
        { key: 'volumeMounted', label: '💾 Volume Mounted', enabled: true },
        { key: 'volumeUnmounted', label: '💿 Volume Unmounted', enabled: true },
        { key: 'diskSpaceLow', label: '⚠️ Disk Space Low', enabled: true },
        { key: 'networkDriveConnected', label: '🌐 Network Drive Connected', enabled: false },
        { key: 'networkDriveDisconnected', label: '📡 Network Drive Disconnected', enabled: false },
        { key: 'usbDriveConnected', label: '🔌 USB Drive Connected', enabled: true },
        { key: 'usbDriveDisconnected', label: '🔌 USB Drive Disconnected', enabled: true },
        { key: 'serviceStarted', label: '🚀 Service Started', enabled: false },
        { key: 'serviceStopped', label: '🛑 Service Stopped', enabled: false }
    ]
};

// Windows-specific encodings
const windowsEncodings = [
    { value: 'utf-8', label: 'UTF-8 (Recommended)' },
    { value: 'utf-16', label: 'UTF-16' },
    { value: 'ascii', label: 'ASCII' },
    { value: 'latin1', label: 'Latin-1' },
    { value: 'cp1252', label: 'Windows-1252' },
    { value: 'cp437', label: 'IBM PC (Code Page 437)' },
    { value: 'cp850', label: 'Code Page 850 (Multilingual)' }
];

// Windows-specific file system hints
const windowsHints = {
    targetDirectory: 'Select a Windows directory to analyze its contents and statistics',
    targetFile: 'Select a Windows file to read and monitor',
    monitorPath: 'Windows directory to monitor for system events',
    pluginDirectory: 'Windows directory containing plugin files',
    includeSubdirs: 'Analyze all subdirectories recursively',
    showHidden: 'Include hidden and system files in analysis',
    maxDepth: 'Maximum directory depth to analyze (1-10)',
    bufferSize: 'Number of Windows events to keep in memory (100-10000)',
    scanInterval: 'How often to scan for new plugins (1-3600 seconds)',
    timeout: 'Maximum time to wait for Windows plugin response (1-300 seconds)'
};
