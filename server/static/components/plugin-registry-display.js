const PluginRegistryDisplay = {
    mixins: [SharedDisplayMixin],
    async mounted() {
        await this.fetchRegistry();
        this.pollingInterval = setInterval(this.fetchRegistry, 10000);
    },
    methods: {
        async fetchRegistry() {
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
window.PluginRegistryDisplay = PluginRegistryDisplay;