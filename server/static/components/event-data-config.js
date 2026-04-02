// Event Data Configuration Component
const EventDataConfig = {
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
                const platformConfig = window.platformManager.getPluginConfig('event_data');
                if (!this.config.monitorPath) {
                    this.config.monitorPath = platformConfig.monitorPath || '';
                }
                if (!this.config.bufferSize) {
                    this.config.bufferSize = platformConfig.bufferSize || 1000;
                }
            }
        },
        
        loadSystemEvents() {
            if (typeof window !== 'undefined' && window.platformManager) {
                this.systemEvents = window.platformManager.getSystemEvents();
                if (!this.config.events) {
                    this.config.events = {};
                    // Initialize events from platform configuration
                    Object.values(this.systemEvents).flat().forEach(eventGroup => {
                        if (Array.isArray(eventGroup)) {
                            eventGroup.forEach(event => {
                                this.config.events[event.key] = event.enabled;
                            });
                        }
                    });
                }
            }
        },
        
        browseDirectory(event) {
            if (typeof window !== 'undefined' && window.platformManager) {
                const defaultPaths = window.platformManager.getDefaultPaths('event_data');
                if (Array.isArray(defaultPaths) && defaultPaths.length > 0) {
                    const currentIndex = defaultPaths.indexOf(this.config.monitorPath);
                    const nextIndex = (currentIndex + 1) % defaultPaths.length;
                    const selectedPath = defaultPaths[nextIndex];
                    
                    this.config.monitorPath = selectedPath;
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
            <h4>📋 Event Data Settings</h4>
            <div class="config-item">
                <label>Events to Monitor:</label>
                <div class="event-groups">
                    <div v-if="systemEvents.fileEvents" class="event-group">
                        <h5>📄 File Events</h5>
                        <div class="checkbox-group">
                            <label v-for="event in systemEvents.fileEvents" :key="event.key">
                                <input type="checkbox" v-model="config.events[event.key]" @change="emitUpdate">
                                {{ event.label }}
                            </label>
                        </div>
                    </div>
                    <div v-if="systemEvents.directoryEvents" class="event-group">
                        <h5>📁 Directory Events</h5>
                        <div class="checkbox-group">
                            <label v-for="event in systemEvents.directoryEvents" :key="event.key">
                                <input type="checkbox" v-model="config.events[event.key]" @change="emitUpdate">
                                {{ event.label }}
                            </label>
                        </div>
                    </div>
                    <div v-if="systemEvents.systemEvents" class="event-group">
                        <h5>⚙️ System Events</h5>
                        <div class="checkbox-group">
                            <label v-for="event in systemEvents.systemEvents" :key="event.key">
                                <input type="checkbox" v-model="config.events[event.key]" @change="emitUpdate">
                                {{ event.label }}
                            </label>
                        </div>
                    </div>
                </div>
            </div>
            <div class="config-item">
                <label>Monitor Path:</label>
                <div class="path-input-group">
                    <input type="text" v-model="config.monitorPath" 
                           placeholder="Click Browse to select directory"
                           @input="emitUpdate">
                    <button class="btn btn-small btn-secondary" @click="browseDirectory">Browse</button>
                </div>
                <small class="hint">{{ getHint('monitorPath') }}</small>
            </div>
            <div class="config-item">
                <label>Buffer Size:</label>
                <input type="number" v-model="config.bufferSize" min="100" max="10000" @input="emitUpdate">
                <small class="hint">{{ getHint('bufferSize') }}</small>
            </div>
        </div>
    `
};

// Export for use in other modules
if (typeof window !== 'undefined') {
    window.EventDataConfig = EventDataConfig;
} else if (typeof module !== 'undefined' && module.exports) {
    module.exports = EventDataConfig;
}
