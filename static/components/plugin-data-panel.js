// Universal Plugin Data Panel Component
// Shows dropdown of available objects from plugin and displays data
const PluginDataPanel = {
    props: ['pluginName', 'agentId'],
    emits: ['data-received', 'error'],
    data() {
        return {
            availableObjects: [],
            selectedObject: null,
            objectData: null,
            loading: false,
            error: null,
            objectType: 'unknown'
        };
    },
    async mounted() {
        await this.loadAvailableObjects();
    },
    watch: {
        agentId(newVal, oldVal) {
            if (newVal && newVal !== oldVal) {
                // Reset state when agent changes
                this.selectedObject = null;
                this.objectData = null;
                this.availableObjects = [];
                this.loadAvailableObjects();
            }
        }
    },
    methods: {
        // Load list of available objects from plugin
        async loadAvailableObjects() {
            if (!this.agentId || this.agentId === 'null') {
                console.warn(`Cannot load objects: no agent selected for plugin ${this.pluginName}`);
                this.availableObjects = [];
                return;
            }
            this.loading = true;
            try {
                const response = await fetch(`/api/agents/${this.agentId}/plugin/${this.pluginName}/objects`);
                if (response.ok) {
                    const data = await response.json();
                    this.availableObjects = data.objects || [];
                    this.objectType = data.object_type || 'item';
                } else {
                    // Fallback: request from plugin via WebSocket command
                    await this.requestObjectsViaWebSocket();
                }
            } catch (e) {
                console.warn(`Failed to load objects for ${this.pluginName}:`, e);
                this.availableObjects = [];
            } finally {
                this.loading = false;
            }
        },

        // Request objects list via WebSocket (fallback)
        async requestObjectsViaWebSocket() {
            if (!window.webSocketClient) return;
            
            const request = {
                type: 'command',
                command: 'get_available_objects',
                params: { plugin: this.pluginName },
                agent_id: this.agentId
            };
            
            try {
                const response = await window.webSocketClient.sendAndWait(request, 5000);
                if (response.success && response.data) {
                    this.availableObjects = response.data.objects || [];
                    this.objectType = response.data.object_type || 'item';
                }
            } catch (e) {
                console.warn('WebSocket object request failed:', e);
            }
        },

        // Load data for selected object
        async loadObjectData() {
            if (!this.selectedObject) return;
            
            this.loading = true;
            this.error = null;
            
            try {
                const response = await fetch(
                    `/api/agents/${this.agentId}/plugin/${this.pluginName}/object/${encodeURIComponent(this.selectedObject)}`
                );
                
                if (response.ok) {
                    this.objectData = await response.json();
                    this.$emit('data-received', {
                        plugin: this.pluginName,
                        object: this.selectedObject,
                        data: this.objectData
                    });
                } else {
                    // Fallback: request via WebSocket
                    await this.requestDataViaWebSocket();
                }
            } catch (e) {
                this.error = `Failed to load data: ${e.message}`;
                this.$emit('error', { plugin: this.pluginName, error: e.message });
            } finally {
                this.loading = false;
            }
        },

        // Request object data via WebSocket (fallback)
        async requestDataViaWebSocket() {
            if (!window.webSocketClient) {
                this.error = 'No WebSocket connection';
                return;
            }
            
            const request = {
                type: 'command',
                command: 'get_object_data',
                params: { 
                    plugin: this.pluginName,
                    object_id: this.selectedObject 
                },
                agent_id: this.agentId
            };
            
            try {
                const response = await window.webSocketClient.sendAndWait(request, 10000);
                if (response.success) {
                    this.objectData = response.data;
                    this.$emit('data-received', {
                        plugin: this.pluginName,
                        object: this.selectedObject,
                        data: this.objectData
                    });
                } else {
                    this.error = response.error || 'Plugin returned error';
                }
            } catch (e) {
                this.error = `WebSocket request failed: ${e.message}`;
            }
        },

        // Get display name for object type
        getObjectTypeLabel() {
            const labels = {
                'directory': '📁 Directories',
                'file': '📄 Files',
                'process': '⚙️ Processes',
                'event': '📊 Events',
                'watcher': '👁️ Watchers',
                'sensor': '📡 Sensors',
                'item': '📋 Items'
            };
            return labels[this.objectType] || '📋 Objects';
        },

        // Format object name for display
        formatObjectName(obj) {
            if (typeof obj === 'string') return obj;
            if (obj.name) return obj.name;
            if (obj.id) return obj.id;
            if (obj.path) return obj.path;
            return JSON.stringify(obj);
        },

        // Get object value for option
        getObjectValue(obj) {
            if (typeof obj === 'string') return obj;
            return obj.id || obj.path || obj.name || JSON.stringify(obj);
        }
    },
    template: `
        <div class="plugin-data-panel">
            <!-- Object Selector -->
            <div class="object-selector">
                <label class="selector-label">{{ getObjectTypeLabel() }}:</label>
                <select v-model="selectedObject" @change="loadObjectData" :disabled="loading || availableObjects.length === 0">
                    <option value="" disabled selected>
                        {{ availableObjects.length === 0 ? 'No objects available' : 'Select ' + objectType + '...' }}
                    </option>
                    <option v-for="obj in availableObjects" :key="getObjectValue(obj)" :value="getObjectValue(obj)">
                        {{ formatObjectName(obj) }}
                    </option>
                </select>
                <button class="btn btn-small btn-primary" @click="loadAvailableObjects" :disabled="loading" title="Refresh list">
                    🔄
                </button>
            </div>

            <!-- Loading indicator -->
            <div v-if="loading" class="loading-indicator">
                <span class="spinner"></span> Loading...
            </div>

            <!-- Error display -->
            <div v-if="error" class="error-message">
                <strong>⚠️ Error:</strong> {{ error }}
            </div>

            <!-- Data Display -->
            <div v-if="objectData && !loading" class="data-display">
                <h4>{{ formatObjectName(selectedObject) }}</h4>
                
                <!-- Raw JSON view -->
                <div class="json-data">
                    <pre>{{ JSON.stringify(objectData, null, 2) }}</pre>
                </div>

                <!-- Structured view if data has known format -->
                <div v-if="objectData.files" class="structured-view">
                    <h5>📁 Files ({{ objectData.files.length }})</h5>
                    <ul class="file-list">
                        <li v-for="file in objectData.files" :key="file.path || file.name">
                            <span class="file-name">{{ file.name || file.path }}</span>
                            <span class="file-size" v-if="file.size">({{ file.size }} bytes)</span>
                        </li>
                    </ul>
                </div>

                <div v-if="objectData.processes" class="structured-view">
                    <h5>⚙️ Processes ({{ objectData.processes.length }})</h5>
                    <ul class="process-list">
                        <li v-for="proc in objectData.processes" :key="proc.pid">
                            <span class="process-pid">[{{ proc.pid }}]</span>
                            <span class="process-name">{{ proc.name }}</span>
                            <span class="process-cpu" v-if="proc.cpu !== undefined">CPU: {{ proc.cpu }}%</span>
                        </li>
                    </ul>
                </div>

                <div v-if="objectData.events" class="structured-view">
                    <h5>📊 Events ({{ objectData.events.length }})</h5>
                    <ul class="event-list">
                        <li v-for="evt in objectData.events" :key="evt.id || evt.timestamp">
                            <span class="event-time">{{ evt.timestamp || evt.time }}</span>
                            <span class="event-type">{{ evt.type || evt.event }}</span>
                        </li>
                    </ul>
                </div>
            </div>

            <!-- No data message -->
            <div v-if="!objectData && !loading && !error && selectedObject" class="no-data">
                No data received for selected object
            </div>
        </div>
    `
};

// Export for use in other modules
if (typeof window !== 'undefined') {
    window.PluginDataPanel = PluginDataPanel;
} else if (typeof module !== 'undefined' && module.exports) {
    module.exports = PluginDataPanel;
}
