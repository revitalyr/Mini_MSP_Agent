// Directory Info Configuration Component
const DirectoryInfoConfig = {
    props: ['plugin'],
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
            if (window.platformManager) {
                const platformConfig = window.platformManager.getPluginConfig('directory_info');
                if (!this.config.targetPath) {
                    this.config.targetPath = platformConfig.targetPath || '';
                }
            }
        },
        
        browseDirectory() {
            if (window.platformManager) {
                const defaultPaths = window.platformManager.getDefaultPaths('directory_info');
                const selectedPath = defaultPaths[0] || defaultPaths[1];
                this.config.targetPath = selectedPath;
                this.emitUpdate();
            }
        },
        
        emitUpdate() {
            this.$emit('update', this.config);
        },
        
        getHint(key) {
            return window.platformManager ? window.platformManager.getHint(key) : '';
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
        </div>
    `
};
