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
            if (typeof window !== 'undefined' && window.platformManager) {
                const platformConfig = window.platformManager.getPluginConfig('directory_info');
                if (!this.config.targetPath) {
                    this.config.targetPath = platformConfig.targetPath || '';
                }
            }
        },

        browseDirectory() {
            console.log('🔍 Browse button clicked - START DEBUG');
            console.log('window:', typeof window);
            console.log('window.platformManager:', typeof window !== 'undefined' ? window.platformManager : 'undefined');
            
            if (typeof window !== 'undefined' && window.platformManager) {
                console.log('✅ window.platformManager available');
                console.log('Getting default paths for directory_info...');
                const defaultPaths = window.platformManager.getDefaultPaths('directory_info');
                console.log('Default paths received:', defaultPaths);
                console.log('Is array?', Array.isArray(defaultPaths));
                console.log('Length:', defaultPaths ? defaultPaths.length : 'undefined');
                
                if (Array.isArray(defaultPaths) && defaultPaths.length > 0) {
                    // Выберем следующий путь из списка для демонстрации
                    const currentIndex = defaultPaths.indexOf(this.config.targetPath);
                    const nextIndex = (currentIndex + 1) % defaultPaths.length;
                    const selectedPath = defaultPaths[nextIndex];
                    console.log('Current path:', this.config.targetPath);
                    console.log('Selected path:', selectedPath);
                    
                    this.config.targetPath = selectedPath;
                    this.emitUpdate();
                    
                    // Show user feedback (more elegant than alert)
                    console.log(`✅ Directory selected: ${selectedPath}`);
                    
                    // Visual feedback - briefly highlight the input
                    const input = event.target?.previousElementSibling;
                    if (input) {
                        input.style.backgroundColor = '#e8f5e8';
                        setTimeout(() => {
                            input.style.backgroundColor = '';
                        }, 500);
                    }
                } else {
                    console.error('❌ No default paths available');
                    alert('No default paths available for this plugin');
                }
            } else {
                console.error('❌ Window or platformManager not available');
                alert('Platform manager not available');
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
        </div>
    `
};

// Export for use in other modules
if (typeof window !== 'undefined') {
    window.DirectoryInfoConfig = DirectoryInfoConfig;
} else if (typeof module !== 'undefined' && module.exports) {
    module.exports = DirectoryInfoConfig;
}
