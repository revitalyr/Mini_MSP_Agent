// Platform detection and configuration manager
class PlatformManager {
    constructor() {
        this.platform = this.detectPlatform();
        this.configs = null;
        this.events = null;
        this.encodings = null;
        this.hints = null;
        this.loadPlatformConfig();
    }

    detectPlatform() {
        const userAgent = navigator.userAgent;
        const platform = navigator.platform;
        
        if (platform.includes('Win') || userAgent.includes('Windows')) {
            return 'windows';
        } else if (platform.includes('Mac') || userAgent.includes('Mac')) {
            return 'macos';
        } else if (platform.includes('Linux') || userAgent.includes('Linux')) {
            return 'linux';
        } else {
            // Fallback to Windows if detection fails
            console.warn('Platform detection failed, defaulting to Windows');
            return 'windows';
        }
    }

    async loadPlatformConfig() {
        try {
            const configPath = `platforms/${this.platform}/plugin-configs.js`;
            const response = await fetch(configPath);
            
            if (!response.ok) {
                throw new Error(`Failed to load config: ${response.status} ${response.statusText}`);
            }
            
            const configText = await response.text();
            
            // Safer approach: import the module dynamically
            try {
                const module = await import(`./platforms/${this.platform}/plugin-configs.js`);
                this.configs = module[`${this.platform}PluginConfigs`] || {};
                this.events = module[`${this.platform}SystemEvents`] || {};
                this.encodings = module[`${this.platform}Encodings`] || [];
                this.hints = module[`${this.platform}Hints`] || {};
            } catch (importError) {
                console.warn('Dynamic import failed, falling back to eval:', importError.message);
                this.loadConfigFromText(configText);
            }
            
            console.log(`Loaded ${this.platform} platform configuration`);
        } catch (error) {
            console.error('Failed to load platform configuration:', error);
            // Fallback to Windows configuration
            this.loadFallbackConfig();
        }
    }

    loadConfigFromText(configText) {
        try {
            // Create a safe evaluation context
            const safeEval = (text) => {
                const exports = {};
                const module = { exports };
                const evalInContext = eval;
                evalInContext(text);
                return module.exports;
            };
            
            const allConfigs = safeEval(configText);
            
            // Load platform-specific configurations
            const configKey = `${this.platform}PluginConfigs`;
            const eventsKey = `${this.platform}SystemEvents`;
            const encodingsKey = `${this.platform}Encodings`;
            const hintsKey = `${this.platform}Hints`;
            
            this.configs = allConfigs[configKey] || {};
            this.events = allConfigs[eventsKey] || {};
            this.encodings = allConfigs[encodingsKey] || [];
            this.hints = allConfigs[hintsKey] || {};
        } catch (error) {
            console.error('Failed to parse config text:', error);
            this.loadFallbackConfig();
        }
    }

    loadFallbackConfig() {
        // Basic fallback configuration
        this.configs = {
            directory_info: { targetPath: 'C:\\Users\\Public\\Documents', defaultPaths: ['C:\\Users\\Public\\Documents'] },
            event_data: { monitorPath: 'C:\\Windows\\System32\\LogFiles', defaultPaths: ['C:\\Windows\\System32\\LogFiles'] },
            file_reader: { targetFile: 'C:\\Users\\Public\\Documents\\config.txt', defaultFiles: ['C:\\Users\\Public\\Documents\\config.txt'] }
        };
        
        this.events = {
            fileEvents: [
                { key: 'fileCreated', label: '📄 File Created', enabled: true },
                { key: 'fileDeleted', label: '🗑️ File Deleted', enabled: true },
                { key: 'fileModified', label: '✏️ File Modified', enabled: true }
            ]
        };
        
        this.encodings = [
            { value: 'utf-8', label: 'UTF-8' },
            { value: 'ascii', label: 'ASCII' }
        ];
        
        this.hints = {
            targetDirectory: 'Select a directory to analyze',
            targetFile: 'Select a file to read',
            monitorPath: 'Directory to monitor for events'
        };
    }

    getPluginConfig(pluginName) {
        return this.configs[pluginName] || {};
    }

    getDefaultPaths(pluginName) {
        const config = this.getPluginConfig(pluginName);
        return config.defaultPaths || [];
    }

    getSystemEvents() {
        return this.events || {};
    }

    getEncodings() {
        return this.encodings || [];
    }

    getHint(key) {
        return this.hints[key] || '';
    }

    getPlatform() {
        return this.platform;
    }

    getPlatformInfo() {
        const platformNames = {
            windows: 'Windows',
            linux: 'Linux',
            macos: 'macOS'
        };
        
        const platformIcons = {
            windows: '🪟',
            linux: '🐧',
            macos: '🍎'
        };
        
        return {
            name: platformNames[this.platform] || 'Unknown',
            icon: platformIcons[this.platform] || '❓',
            platform: this.platform
        };
    }

    // Helper method to format paths for current platform
    formatPath(path) {
        if (this.platform === 'windows') {
            return path.replace(/\//g, '\\');
        } else {
            return path.replace(/\\/g, '/');
        }
    }

    // Helper method to get appropriate path separator
    getPathSeparator() {
        return this.platform === 'windows' ? '\\' : '/';
    }
}

// Global platform manager instance
window.platformManager = new PlatformManager();

// Export for use in other modules
if (typeof module !== 'undefined' && module.exports) {
    module.exports = PlatformManager;
}
