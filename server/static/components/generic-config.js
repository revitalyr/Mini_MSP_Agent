// Generic Plugin Configuration Component
const GenericConfig = {
    props: ['plugin', 'agentId'],
    emits: ['update'],
    data() {
        return {
            config: { ...this.plugin }
        };
    },
    mounted() {
        this.loadPlatformDefaults();
    },
    methods: {
        loadPlatformDefaults() {
            const pluginName = this.plugin.name;
            if (typeof window !== 'undefined' && window.platformManager) {
                const platformConfig = window.platformManager.getPluginConfig(pluginName);
                
                // Load platform-specific defaults based on plugin type
                if (pluginName === 'file_signature') {
                    if (!this.config.targetPath) {
                        this.config.targetPath = platformConfig.targetPath || '';
                    }
                    if (!this.config.hashAlgorithm) {
                        this.config.hashAlgorithm = platformConfig.hashAlgorithm || 'sha256';
                    }
                } else if (pluginName === 'folder_watcher') {
                    if (!this.config.targetPath) {
                        this.config.targetPath = platformConfig.targetPath || '';
                    }
                } else if (pluginName === 'plugin_registry') {
                    if (!this.config.pluginDirectory) {
                        this.config.pluginDirectory = platformConfig.pluginDirectory || '';
                    }
                    if (!this.config.scanInterval) {
                        this.config.scanInterval = platformConfig.scanInterval || 30;
                    }
                }
            }
        },
        
        async browseDirectory() {
            const res = await fetch('/api/browse/directory', { method: 'POST' });
            const data = await res.json();
            if (data.path) {
                // Set path based on plugin type
                if (this.plugin.name === 'directory_info') {
                    this.config.targetPath = data.path;
                } else if (this.plugin.name === 'file_signature') {
                    this.config.targetPath = data.path;
                } else if (this.plugin.name === 'folder_watcher') {
                    this.config.targetPath = data.path;
                } else if (this.plugin.name === 'plugin_registry') {
                    this.config.pluginDirectory = data.path;
                }
                this.emitUpdate();
            }
        },
        
        emitUpdate() {
            this.$emit('update', this.config);
        },
        
        getHint(key) {
            return (typeof window !== 'undefined' && window.platformManager) 
                ? window.platformManager.getHint(key) 
                : '';
        }
    },
    template: `
        <div class="config-section">
            <h4>⚙️ {{ plugin.displayName }} Settings</h4>
            
            <template v-if="plugin.name === 'file_signature'">
                <div class="config-item">
                    <label>Target File/Directory:</label>
                    <div class="path-input-group">
                        <input type="text" v-model="config.targetPath" 
                               placeholder="Click Browse to select file or directory"
                               @input="emitUpdate">
                        <button class="btn btn-small btn-secondary" @click="browseDirectory">Browse</button>
                    </div>
                    <small class="hint">{{ getHint('targetDirectory') }}</small>
                </div>
                <div class="config-item">
                    <label>Hash Algorithm:</label>
                    <select v-model="config.hashAlgorithm" @change="emitUpdate">
                        <option value="sha256">SHA-256 (Recommended)</option>
                        <option value="sha512">SHA-512</option>
                        <option value="md5">MD5 (Legacy)</option>
                        <option value="sha1">SHA-1 (Legacy)</option>
                    </select>
                    <small class="hint">Cryptographic hash algorithm for file signatures</small>
                </div>
                <div class="config-item">
                    <label>Include Metadata:</label>
                    <input type="checkbox" v-model="config.includeMetadata" @change="emitUpdate">
                    <small class="hint">Include file metadata (timestamps, size, etc.)</small>
                </div>
            </template>
            
            <!-- Folder Watcher Specific -->
            <template v-else-if="plugin.name === 'folder_watcher'">
                <div class="config-item">
                    <label>Target Directory:</label>
                    <div class="path-input-group">
                        <input type="text" v-model="config.targetPath" 
                               placeholder="Click Browse to select directory"
                               @input="emitUpdate">
                        <button class="btn btn-small btn-secondary" @click="browseDirectory">Browse</button>
                    </div>
                    <small class="hint">{{ getHint('targetDirectory') }}</small>
                </div>
                <div class="config-item">
                    <label>Recursive:</label>
                    <input type="checkbox" v-model="config.recursive" @change="emitUpdate">
                    <small class="hint">Watch subdirectories recursively</small>
                </div>
            </template>
            
            <!-- Plugin Registry Specific -->
            <template v-else-if="plugin.name === 'plugin_registry'">
                <div class="config-item">
                    <label>Plugin Directory:</label>
                    <div class="path-input-group">
                        <input type="text" v-model="config.pluginDirectory" 
                               placeholder="Click Browse to select plugin directory"
                               @input="emitUpdate">
                        <button class="btn btn-small btn-secondary" @click="browseDirectory">Browse</button>
                    </div>
                    <small class="hint">{{ getHint('pluginDirectory') }}</small>
                </div>
                <div class="config-item">
                    <label>Auto Scan:</label>
                    <input type="checkbox" v-model="config.autoScan" @change="emitUpdate">
                    <small class="hint">Automatically scan for new plugins</small>
                </div>
                <div class="config-item">
                    <label>Scan Interval (seconds):</label>
                    <input type="number" v-model="config.scanInterval" min="1" max="3600" @input="emitUpdate">
                    <small class="hint">{{ getHint('scanInterval') }}</small>
                </div>
                <div class="config-item">
                    <label>Enable Hot Reload:</label>
                    <input type="checkbox" v-model="config.hotReload" @change="emitUpdate">
                    <small class="hint">Reload plugins automatically when files change</small>
                </div>
            </template>
            
            <!-- Generic Settings for all plugins -->
            <div class="config-item">
                <label>Plugin Name:</label>
                <input type="text" v-model="config.name" readonly>
            </div>
            <div class="config-item">
                <label>Status:</label>
                <select v-model="config.status" @change="emitUpdate">
                    <option value="active">Active</option>
                    <option value="inactive">Inactive</option>
                    <option value="error">Error</option>
                </select>
            </div>
            <div class="config-item">
                <label>Auto-start:</label>
                <input type="checkbox" v-model="config.autoStart" @change="emitUpdate">
            </div>
            <div class="config-item">
                <label>Log Level:</label>
                <select v-model="config.logLevel" @change="emitUpdate">
                    <option value="debug">Debug</option>
                    <option value="info">Info</option>
                    <option value="warning">Warning</option>
                    <option value="error">Error</option>
                </select>
            </div>
        </div>
        
        <!-- Common Performance Settings -->
        <div class="config-section">
            <h4>📊 Performance Settings</h4>
            <div class="config-item">
                <label>Max Calls per Minute:</label>
                <input type="number" v-model="config.maxCallsPerMinute" min="1" max="1000" @input="emitUpdate">
            </div>
            <div class="config-item">
                <label>Timeout (seconds):</label>
                <input type="number" v-model="config.timeout" min="1" max="300" @input="emitUpdate">
                <small class="hint">{{ getHint('timeout') }}</small>
            </div>
        </div>

        <!-- Context Specific Monitors -->
        <template v-if="plugin.name === 'plugin_registry'">
            <hr>
            <div class="data-display-section">
                <h4>🔌 Plugin Registry</h4>
                <p>Plugin registry monitoring active</p>
                <p><small>Plugin list will appear here when agent is connected</small></p>
            </div>
        </template>
    `
};

// Export for use in other modules
if (typeof window !== 'undefined') {
    window.GenericConfig = GenericConfig;
} else if (typeof module !== 'undefined' && module.exports) {
    module.exports = GenericConfig;
}
