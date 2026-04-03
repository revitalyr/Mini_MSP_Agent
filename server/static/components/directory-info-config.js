// Directory Info Configuration Component
const DirectoryInfoConfig = {
    props: ['plugin', 'agentId'],
    emits: ['update'],
    data() {
        return {
            config: { ...this.plugin }
        };
    },
    async mounted() {
        await this.loadPlatformDefaults();
    },
    methods: {
        async loadPlatformDefaults() {
            if (typeof window !== 'undefined' && window.platformManager) {
                // Wait for platform configuration to be fully loaded
                await window.platformManager.ready();
                const platformConfig = window.platformManager.getPluginConfig('directory_info');
                if (!this.config.targetPath) {
                    this.config.targetPath = platformConfig.targetPath || '';
                }
            }
        },

        async browseDirectory() {
            const res = await fetch('/api/browse/directory', { method: 'POST' });
            const data = await res.json();
            if (data.path) {
                this.config.targetPath = data.path;
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
            <h4>📁 Directory Info Settings</h4>
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
                <label>Include Subdirectories:</label>
                <input type="checkbox" v-model="config.includeSubdirs" @change="emitUpdate">
                <small class="hint">{{ getHint('includeSubdirs') }}</small>
            </div>
            <div class="config-item">
                <label>Show Hidden Files:</label>
                <input type="checkbox" v-model="config.showHidden" @change="emitUpdate">
                <small class="hint">{{ getHint('showHidden') }}</small>
            </div>
            <div class="config-item">
                <label>Max Depth:</label>
                <input type="number" v-model="config.maxDepth" min="1" max="10" @input="emitUpdate">
                <small class="hint">{{ getHint('maxDepth') }}</small>
            </div>

            <!-- Live Data Monitor -->
            <hr>
            <div class="data-display-section">
                <h4>📁 Directory Information</h4>
                <p>Directory monitoring active for: <strong>{{ config.targetPath || 'Not configured' }}</strong></p>
                <div v-if="config.targetPath">
                    <p><small>Files and statistics will appear here when connected to agent</small></p>
                </div>
            </div>
        </div>
    `
};

// Export for use in other modules
if (typeof window !== 'undefined') {
    window.DirectoryInfoConfig = DirectoryInfoConfig;
} else if (typeof module !== 'undefined' && module.exports) {
    module.exports = DirectoryInfoConfig;
}
