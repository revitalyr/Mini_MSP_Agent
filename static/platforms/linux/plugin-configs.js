// Linux-specific plugin configurations
export const linuxPluginConfigs = {
    directory_info: {
        targetPath: '/home/user/documents',
        defaultPaths: [
            '/home/user/documents',
            '/home/user/downloads',
            '/var/log',
            '/tmp',
            '/opt',
            '/usr/local',
            '/media'
        ]
    },
    event_data: {
        monitorPath: '/var/log',
        defaultPaths: [
            '/var/log',
            '/var/log/syslog',
            '/var/log/auth.log',
            '/home/user/documents',
            '/tmp',
            '/var/spool'
        ]
    },
    file_reader: {
        targetFile: '/home/user/documents/config.txt',
        defaultFiles: [
            '/home/user/documents/config.txt',
            '/home/user/documents/data.csv',
            '/etc/hosts',
            '/var/log/syslog',
            '/home/user/.bashrc',
            '/etc/fstab'
        ]
    },
    file_signature: {
        targetPath: '/home/user/documents',
        defaultPaths: [
            '/home/user/documents',
            '/home/user/downloads',
            '/var/log',
            '/etc',
            '/opt'
        ]
    },
    folder_watcher: {
        targetPath: '/home/user/documents',
        defaultPaths: [
            '/home/user/documents',
            '/home/user/downloads',
            '/tmp',
            '/var/tmp'
        ]
    },
    watchers_manager: {
        watchPaths: ['/home/user/documents', '/home/user/downloads'],
        defaultPaths: [
            '/home/user/documents',
            '/home/user/downloads',
            '/tmp',
            '/var/log',
            '/media'
        ]
    },
    plugin_registry: {
        pluginDirectory: '/opt/mini-msp-agent/plugins',
        defaultPaths: [
            '/opt/mini-msp-agent/plugins',
            '/usr/local/lib/mini-msp-agent/plugins',
            '/home/user/.local/share/mini-msp-agent/plugins',
            '/var/lib/mini-msp-agent/plugins'
        ]
    }
};

export const linuxSystemEvents = {
    fileEvents: [
        { key: 'fileCreated', label: '📄 File Created', enabled: true },
        { key: 'fileDeleted', label: '🗑️ File Deleted', enabled: true },
        { key: 'fileModified', label: '✏️ File Modified', enabled: true },
        { key: 'fileRenamed', label: '🏷️ File Renamed', enabled: false },
        { key: 'fileAccessed', label: '👁️ File Accessed', enabled: false },
        { key: 'filePermissionsChanged', label: '🔐 Permissions Changed', enabled: true },
        { key: 'fileAttributesChanged', label: '📝 Attributes Changed', enabled: false },
        { key: 'fileSymlinkCreated', label: '🔗 Symlink Created', enabled: true },
        { key: 'fileSymlinkDeleted', label: '🔗 Symlink Deleted', enabled: false }
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
        { key: 'cronJobExecuted', label: '⏰ Cron Job Executed', enabled: false },
        { key: 'packageInstalled', label: '📦 Package Installed', enabled: false },
        { key: 'packageRemoved', label: '📦 Package Removed', enabled: false }
    ]
};

export const linuxEncodings = [
    { value: 'utf-8', label: 'UTF-8 (Recommended)' },
    { value: 'utf-16', label: 'UTF-16' },
    { value: 'ascii', label: 'ASCII' },
    { value: 'latin1', label: 'Latin-1' },
    { value: 'iso-8859-15', label: 'ISO-8859-15' },
    { value: 'koi8-r', label: 'KOI8-R (Cyrillic)' },
    { value: 'koi8-u', label: 'KOI8-U (Ukrainian)' }
];

export const linuxHints = {
    targetDirectory: 'Select a Linux directory to analyze its contents and statistics',
    targetFile: 'Select a Linux file to read and monitor',
    monitorPath: 'Linux directory to monitor for system events',
    pluginDirectory: 'Linux directory containing plugin files',
    includeSubdirs: 'Analyze all subdirectories recursively',
    showHidden: 'Include hidden and system files in analysis',
    maxDepth: 'Maximum directory depth to analyze (1-10)',
    bufferSize: 'Number of Linux events to keep in memory (100-10000)',
    scanInterval: 'How often to scan for new plugins (1-3600 seconds)',
    timeout: 'Maximum time to wait for Linux plugin response (1-300 seconds)'
};
