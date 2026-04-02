// File Reader Configuration Component
const FileReaderConfig = {
    props: ['plugin'],
    emits: ['update'],
    data() {
        return {
            config: { ...this.plugin },
            encodings: []
        };
    },
    mounted() {
        this.loadPlatformDefaults();
        this.loadEncodings();
    },
    methods: {
        loadPlatformDefaults() {
            if (typeof window !== 'undefined' && window.platformManager) {
                const platformConfig = window.platformManager.getPluginConfig('file_reader');
                if (!this.config.targetFile) {
                    this.config.targetFile = platformConfig.targetFile || '';
                }
                if (!this.config.encoding) {
                    this.config.encoding = platformConfig.encoding || 'utf-8';
                }
            }
        },
        
        loadEncodings() {
            if (typeof window !== 'undefined' && window.platformManager) {
                this.encodings = window.platformManager.getEncodings();
            }
        },
        
        browseFile() {
            if (typeof window !== 'undefined' && window.platformManager) {
                const platformConfig = window.platformManager.getPluginConfig('file_reader');
                const defaultFiles = (platformConfig && platformConfig.defaultFiles) || [];
                const selectedFile = defaultFiles[0] || defaultFiles[1];
                
                if (selectedFile) {
                    this.config.targetFile = selectedFile;
                    this.emitUpdate();
                    console.log(`File selected: ${selectedFile}`);
                }
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
            <h4>📄 File Reader Settings</h4>
            <div class="config-item">
                <label>Target File:</label>
                <div class="path-input-group">
                    <input type="text" v-model="config.targetFile" 
                           placeholder="Click Browse to select file"
                           @input="emitUpdate">
                    <button class="btn btn-small btn-secondary" @click="browseFile">Browse</button>
                </div>
                <small class="hint">{{ getHint('targetFile') }}</small>
            </div>
            <div class="config-item">
                <label>File Encoding:</label>
                <select v-model="config.encoding" @change="emitUpdate">
                    <option v-for="encoding in encodings" :key="encoding.value" :value="encoding.value">
                        {{ encoding.label }}
                    </option>
                </select>
                <small class="hint">Character encoding for file reading</small>
            </div>
            <div class="config-item">
                <label>Read Mode:</label>
                <select v-model="config.readMode" @change="emitUpdate">
                    <option value="text">Text Mode</option>
                    <option value="binary">Binary Mode</option>
                    <option value="lines">Line by Line</option>
                    <option value="chunks">Fixed Size Chunks</option>
                </select>
                <small class="hint">How to read file content</small>
            </div>
            <div class="config-item">
                <label>Auto Reload:</label>
                <input type="checkbox" v-model="config.autoReload" @change="emitUpdate">
                <small class="hint">Automatically reload file when changes are detected</small>
            </div>
        </div>
    `
};

// Export for use in other modules
if (typeof window !== 'undefined') {
    window.FileReaderConfig = FileReaderConfig;
} else if (typeof module !== 'undefined' && module.exports) {
    module.exports = FileReaderConfig;
}
