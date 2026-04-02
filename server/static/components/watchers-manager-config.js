// Watchers Manager Configuration Component
const WatchersManagerConfig = {
    props: ['plugin'],
    emits: ['update'],
    data() {
        return {
            config: { ...this.plugin },
            systemEvents: {}
        };
    },
    async mounted() {
        if (typeof window !== 'undefined' && window.platformManager) {
            await window.platformManager.ready();
        }
        this.loadPlatformDefaults();
        this.loadSystemEvents();
    },
    methods: {
        loadPlatformDefaults() {
            if (typeof window !== 'undefined' && window.platformManager) {
                const platformConfig = window.platformManager.getPluginConfig('watchers_manager');
                if (!this.config.watchPaths || this.config.watchPaths.length === 0) {
                    this.config.watchPaths = platformConfig.watchPaths || [''];
                }
                if (!this.config.recursive) {
                    this.config.recursive = platformConfig.recursive !== undefined ? platformConfig.recursive : true;
                }
            }
        },
        
        loadSystemEvents() {
            if (typeof window !== 'undefined' && window.platformManager) {
                this.systemEvents = window.platformManager.getSystemEvents();
                if (!this.config.watchEvents) {
                    this.config.watchEvents = {};
                    // Initialize watcher events from platform configuration
                    const fileEvents = this.systemEvents.fileEvents || [];
                    fileEvents.forEach(event => {
                        this.config.watchEvents[event.key.replace('file', '').toLowerCase()] = event.enabled;
                    });
                }
            }
        },
        
        addWatchPath() {
            this.config.watchPaths.push('');
            this.emitUpdate();
        },
        
        removeWatchPath(index) {
            this.config.watchPaths.splice(index, 1);
            this.emitUpdate();
        },
        
        browseDirectoryForPath(index, event) {
            if (typeof window !== 'undefined' && window.platformManager) {
                const defaultPaths = window.platformManager.getDefaultPaths('watchers_manager');
                if (Array.isArray(defaultPaths) && defaultPaths.length > 0) {
                    const currentPath = this.config.watchPaths[index];
                    const currentIndex = defaultPaths.indexOf(currentPath);
                    const nextIndex = (currentIndex + 1) % defaultPaths.length;
                    const selectedPath = defaultPaths[nextIndex];
                    
                    this.config.watchPaths[index] = selectedPath;
                    this.emitUpdate();
                    
                    // Visual feedback
                    const input = event?.currentTarget?.previousElementSibling;
                    if (input) {
                        input.style.backgroundColor = '#e8f5e8';
                        setTimeout(() => { input.style.backgroundColor = ''; }, 500);
                    }
                }
            }
        },

        getHint(key) {
            return (typeof window !== 'undefined' && window.platformManager) 
                ? window.platformManager.getHint(key) 
                : '';
        },
        
        emitUpdate() {
            this.$emit('update', this.config);
        }
    },
    template: `
        <div class="config-section">
            <h4>👁️ Watchers Manager Settings</h4>
            <div class="config-item">
                <label>Watch Directories:</label>
                <div class="path-list">
                    <div v-for="(path, index) in config.watchPaths" :key="index" class="path-item">
                        <input type="text" v-model="config.watchPaths[index]" 
                               placeholder="Click Browse to select directory"
                               @input="emitUpdate">
                        <button class="btn btn-small btn-secondary" @click="browseDirectoryForPath(index, $event)">Browse</button>
                        <button class="btn btn-small btn-danger" @click="removeWatchPath(index)">×</button>
                    </div>
                    <button class="btn btn-small btn-secondary" @click="addWatchPath">+ Add Path</button>
                </div>
                <small class="hint">Directories to monitor for changes</small>
            </div>
            <div class="config-item">
                <label>Watch Events:</label>
                <div class="checkbox-group">
                    <label v-for="(enabled, key) in config.watchEvents" :key="key">
                        <input type="checkbox" v-model="config.watchEvents[key]" @change="emitUpdate">
                        {{ key.charAt(0).toUpperCase() + key.slice(1) }}
                    </label>
                </div>
            </div>
            <div class="config-item">
                <label>Recursive:</label>
                <input type="checkbox" v-model="config.recursive" @change="emitUpdate">
                <small class="hint">Monitor subdirectories recursively</small>
            </div>
        </div>
    `
};

// Export for use in other modules
if (typeof window !== 'undefined') {
    window.WatchersManagerConfig = WatchersManagerConfig;
} else if (typeof module !== 'undefined' && module.exports) {
    module.exports = WatchersManagerConfig;
}
