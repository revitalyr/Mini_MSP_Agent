// Platform detection and configuration manager
class PlatformManager {
    constructor() {
        this.platform = this.detectPlatform();
        this.configs = {};
        this.events = {};
        this.encodings = [];
        this.hints = {};
        this._ready = false;
        this._readyCallbacks = [];

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

            try {
                this.loadConfigFromText(configText);
                console.log(`Successfully loaded ${this.platform} configurations:`, {
                    configs: Object.keys(this.configs),
                    events: Object.keys(this.events),
                    encodings: this.encodings.length,
                    hints: Object.keys(this.hints)
                });
            } catch (evalError) {
                console.error('Failed to load configuration:', evalError);
                this.loadFallbackConfig();
            }

            console.log(`Loaded ${this.platform} platform configuration`);
        } catch (error) {
            console.error('Failed to load platform configuration:', error);
            this.loadFallbackConfig();
        } finally {
            this._ready = true;
            this._readyCallbacks.forEach(cb => cb());
            this._readyCallbacks = [];
        }
    }

    // Returns a Promise that resolves once config is fully loaded.
    // Components should await this before calling getDefaultPaths().
    ready() {
        if (this._ready) return Promise.resolve();
        return new Promise(resolve => this._readyCallbacks.push(resolve));
    }

    loadConfigFromText(configText) {
        try {
            const platformKey  = `${this.platform}PluginConfigs`;
            const eventsKey    = `${this.platform}SystemEvents`;
            const encodingsKey = `${this.platform}Encodings`;
            const hintsKey     = `${this.platform}Hints`;

            // Strip ES module export keywords so the code runs as plain JS
            const cleanText = configText.replace(/export\s+const\s+/g, 'var ');

            // Wrap in a Function so declared vars are accessible by name in the return
            const wrapper = new Function(`
                ${cleanText}
                return {
                    configs:   typeof ${platformKey}  !== 'undefined' ? ${platformKey}  : null,
                    events:    typeof ${eventsKey}     !== 'undefined' ? ${eventsKey}     : null,
                    encodings: typeof ${encodingsKey}  !== 'undefined' ? ${encodingsKey}  : null,
                    hints:     typeof ${hintsKey}      !== 'undefined' ? ${hintsKey}      : null
                };
            `);

            const result = wrapper();

            if (!result.configs) {
                throw new Error(`Variable ${platformKey} not found in config file`);
            }

            this.configs   = result.configs;
            this.events    = result.events    || {};
            this.encodings = result.encodings || [];
            this.hints     = result.hints     || {};

            console.log(`Loaded ${this.platform} configuration`);
        } catch (error) {
            console.error('Failed to evaluate configuration:', error);
            throw error;
        }
    }

    loadFallbackConfig() {
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
        const platformNames = { windows: 'Windows', linux: 'Linux', macos: 'macOS' };
        const platformIcons = { windows: '🪟', linux: '🐧', macos: '🍎' };

        return {
            name: platformNames[this.platform] || 'Unknown',
            icon: platformIcons[this.platform] || '❓',
            platform: this.platform
        };
    }

    formatPath(path) {
        if (this.platform === 'windows') {
            return path.replace(/\//g, '\\');
        } else {
            return path.replace(/\\/g, '/');
        }
    }

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
