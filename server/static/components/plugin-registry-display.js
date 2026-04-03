const PluginRegistryDisplay = {
    mixins: [SharedDisplayMixin],
    async mounted() {
        await this.fetchRegistry();
        this.pollingInterval = setInterval(this.fetchRegistry, 10000);
    },
    methods: {
        async fetchRegistry() {
            // For demo, show mock data
            if (this.agentId === 'demo-agent-001') {
                this.liveData = [
                    {name: 'directory_info', version: '1.0.0', status: 'Active'},
                    {name: 'event_data', version: '1.0.0', status: 'Active'},
                    {name: 'file_reader', version: '1.0.0', status: 'Inactive'}
                ];
                return;
            }
            
            await this.fetchData(`/agents/${this.agentId}/data/plugin_registry`, 'plugins');
        }
    },
    template: `
        <div class="data-display-section">
            <h4>🔌 Loaded Plugins Registry</h4>
            <table class="plugin-table" v-if="liveData && liveData.length">
                <thead>
                    <tr><th>Name</th><th>Version</th><th>Status</th></tr>
                </thead>
                <tbody>
                    <tr v-for="p in liveData" :key="p.name">
                        <td>{{ p.name }}</td>
                        <td>{{ p.version }}</td>
                        <td><span :class="'status-' + p.status.toLowerCase()">{{ p.status }}</span></td>
                    </tr>
                </tbody>
            </table>
            <p v-else-if="!error">Querying plugin registry...</p>
            <div v-else class="error-message">{{ error }}</div>
        </div>
    `
};

// Export for use in other modules
if (typeof window !== 'undefined') {
    window.PluginRegistryDisplay = PluginRegistryDisplay;
} else if (typeof module !== 'undefined' && module.exports) {
    module.exports = PluginRegistryDisplay;
}