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
    mounted() {
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
        
        browseDirectory() {
            if (typeof window !== 'undefined' && window.platformManager) {
                const defaultPaths = window.platformManager.getDefaultPaths('event_data');
                const selectedPath = defaultPaths[0] || defaultPaths[1];
                this.config.monitorPath = selectedPath;
                this.emitUpdate();
                
                console.log(`Directory selected: ${selectedPath}`);
            }
        },
        
        emitUpdate() {
            this.$emit('update', this.config);
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
                <small class="hint">{{ window.platformManager.getHint('monitorPath') }}</small>
            </div>
            <div class="config-item">
                <label>Buffer Size:</label>
                <input type="number" v-model="config.bufferSize" min="100" max="10000" @input="emitUpdate">
                <small class="hint">{{ window.platformManager.getHint('bufferSize') }}</small>
            </div>
        </div>
    `
};
