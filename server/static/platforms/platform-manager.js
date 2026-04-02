// Platform detection and configuration manager
class PlatformManager {
    constructor() {
        this.platform = this.detectPlatform();
        this.configs = {};
        this.events = {};
        this.encodings = [];
        this.hints = {};
        
        console.log(`Platform detected: ${this.platform}`);
        this.loadPlatformConfig();
    }

    detectPlatform() {
        const userAgent = navigator.userAgent.toLowerCase();
        const platform = navigator.platform.toLowerCase();
        
        if (userAgent.includes('win') || platform.includes('win')) {
            return 'windows';
        } else if (userAgent.includes('mac') || platform.includes('mac')) {
            return 'macos';
        } else if (userAgent.includes('linux') || platform.includes('linux')) {
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
            
            // Safer approach: use eval to load configuration
            try {
                this.loadConfigFromText(configText);
                console.log(`Successfully loaded ${this.platform} configurations:`, {
                    configs: Object.keys(this.configs),
                    events: Object.keys(this.events),
                    encodings: this.encodings.length,
                    hints: Object.keys(this.hints)
                });
            } catch (evalError) {
                console.error('Failed to load configuration with eval:', evalError);
                this.loadFallbackConfig();
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
            // Remove export statements and evaluate
            const cleanText = configText
                .replace(/export\s+const\s+/g, 'const ')
                .replace(/export\s+{/g, '{');
            
            // Create a safe evaluation context
            const exports = {};
            const module = { exports };
            const evalInContext = eval;
            evalInContext(cleanText);
            
            // Extract configurations from module exports
            const platformKey = `${this.platform}PluginConfigs`;
            const eventsKey = `${this.platform}SystemEvents`;
            const encodingsKey = `${this.platform}Encodings`;
            const hintsKey = `${this.platform}Hints`;
            
            this.configs = module.exports[platformKey] || {};
            this.events = module.exports[eventsKey] || {};
            this.encodings = module.exports[encodingsKey] || [];
            this.hints = module.exports[hintsKey] || {};
            
            console.log(`Loaded ${this.platform} configuration from eval`);
        } catch (error) {
            console.error('Failed to evaluate configuration:', error);
            throw error;
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
        console.log(`Getting default paths for plugin: ${pluginName}`);
        console.log('Available configs:', Object.keys(this.configs));
        console.log('Config for plugin:', this.configs[pluginName]);
        
        const config = this.getPluginConfig(pluginName);
        const paths = config.defaultPaths || [];
        console.log(`Default paths for ${pluginName}:`, paths);
        return paths;
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
if (typeof window !== 'undefined') {
    window.platformManager = new PlatformManager();
}

// Export for use in other modules
if (typeof module !== 'undefined' && module.exports) {
    module.exports = PlatformManager;
}
