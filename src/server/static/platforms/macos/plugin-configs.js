// macOS-specific plugin configurations
export const macosPluginConfigs = {
    directory_info: {
        targetPath: '/Users/Shared/Documents',
        defaultPaths: [
            '/Users/Shared/Documents',
            '/Users/Shared/Downloads',
            '/Users/Shared/Desktop',
            '/Applications',
            '/Library',
            '/System/Library',
            '/Volumes'
        ]
    },
    event_data: {
        monitorPath: '/var/log',
        defaultPaths: [
            '/var/log',
            '/var/log/system.log',
            '/var/log/install.log',
            '/Users/Shared/Documents',
            '/tmp',
            '/Library/Logs'
        ]
    },
    file_reader: {
        targetFile: '/Users/Shared/Documents/config.txt',
        defaultFiles: [
            '/Users/Shared/Documents/config.txt',
            '/Users/Shared/Documents/data.csv',
            '/etc/hosts',
            '/var/log/system.log',
            '/Users/Shared/.bash_profile',
            '/etc/fstab'
        ]
    },
    file_signature: {
        targetPath: '/Users/Shared/Documents',
        defaultPaths: [
            '/Users/Shared/Documents',
            '/Users/Shared/Downloads',
            '/Applications',
            '/Library',
            '/Volumes'
        ]
    },
    folder_watcher: {
        targetPath: '/Users/Shared/Documents',
        defaultPaths: [
            '/Users/Shared/Documents',
            '/Users/Shared/Downloads',
            '/tmp',
            '/Users/Shared/Desktop'
        ]
    },
    watchers_manager: {
        watchPaths: ['/Users/Shared/Documents', '/Users/Shared/Downloads'],
        defaultPaths: [
            '/Users/Shared/Documents',
            '/Users/Shared/Downloads',
            '/tmp',
            '/Library/Logs',
            '/Volumes'
        ]
    },
    plugin_registry: {
        pluginDirectory: '/Applications/Mini MSP Agent.app/Contents/PlugIns',
        defaultPaths: [
            '/Applications/Mini MSP Agent.app/Contents/PlugIns',
            '/Library/Application Support/Mini MSP Agent/plugins',
            '/Users/Shared/Library/Application Support/Mini MSP Agent/plugins',
            '/usr/local/lib/mini-msp-agent/plugins'
        ]
    }
};

export const macosSystemEvents = {
    fileEvents: [
        { key: 'fileCreated', label: '📄 File Created', enabled: true },
        { key: 'fileDeleted', label: '🗑️ File Deleted', enabled: true },
        { key: 'fileModified', label: '✏️ File Modified', enabled: true },
        { key: 'fileRenamed', label: '🏷️ File Renamed', enabled: false },
        { key: 'fileAccessed', label: '👁️ File Accessed', enabled: false },
        { key: 'filePermissionsChanged', label: '🔐 Permissions Changed', enabled: true },
        { key: 'fileAttributesChanged', label: '📝 Attributes Changed', enabled: false },
        { key: 'fileExtendedAttributesChanged', label: '🏷️ Extended Attributes Changed', enabled: true },
        { key: 'fileQuarantineChanged', label: '🛡️ Quarantine Status Changed', enabled: false }
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
        { key: 'serviceStopped', label: '🛑 Service Stopped', enabled: false },
        { key: 'applicationLaunched', label: '🚀 Application Launched', enabled: false },
        { key: 'applicationTerminated', label: '🛑 Application Terminated', enabled: false },
        { key: 'spotlightIndexChanged', label: '🔍 Spotlight Index Changed', enabled: false },
        { key: 'timeMachineBackup', label: '💾 Time Machine Backup', enabled: false },
        { key: 'systemSleep', label: '😴 System Sleep', enabled: false },
        { key: 'systemWake', label: '⏰ System Wake', enabled: false }
    ]
};

export const macosEncodings = [
    { value: 'utf-8', label: 'UTF-8 (Recommended)' },
    { value: 'utf-16', label: 'UTF-16' },
    { value: 'ascii', label: 'ASCII' },
    { value: 'latin1', label: 'Latin-1' },
    { value: 'iso-8859-1', label: 'ISO-8859-1' },
    { value: 'mac-roman', label: 'Mac Roman' },
    { value: 'utf-32', label: 'UTF-32' }
];

export const macosHints = {
    targetDirectory: 'Select a macOS directory to analyze its contents and statistics',
    targetFile: 'Select a macOS file to read and monitor',
    monitorPath: 'macOS directory to monitor for system events',
    pluginDirectory: 'macOS directory containing plugin files',
    includeSubdirs: 'Analyze all subdirectories recursively',
    showHidden: 'Include hidden and system files in analysis',
    maxDepth: 'Maximum directory depth to analyze (1-10)',
    bufferSize: 'Number of macOS events to keep in memory (100-10000)',
    scanInterval: 'How often to scan for new plugins (1-3600 seconds)',
    timeout: 'Maximum time to wait for macOS plugin response (1-300 seconds)'
};
