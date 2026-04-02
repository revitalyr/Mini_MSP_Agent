const PluginRegistryDisplay = {
    props: ['agentId'],
    data() {
        return { plugins: [], pollingInterval: null };
    },
    async mounted() {
        await this.fetchData();
        this.pollingInterval = setInterval(this.fetchData, 10000);
    },
    beforeUnmount() { clearInterval(this.pollingInterval); },
    methods: {
        async fetchData() {
            try {
                const token = localStorage.getItem('token');
                const response = await fetch(`/agents/${this.agentId}/data/plugin_registry`, {
                    headers: { 'Authorization': `Bearer ${token}` }
                });
                const data = await response.json();
                this.plugins = data.plugins || [];
            } catch (e) { console.error(e); }
        }
    },
    template: `
        <div class="data-display-section">
            <h4>🔌 Loaded Plugins Registry</h4>
            <table class="plugin-table" v-if="plugins.length">
                <thead>
                    <tr><th>Name</th><th>Version</th><th>Status</th></tr>
                </thead>
                <tbody>
                    <tr v-for="p in plugins" :key="p.name">
                        <td>{{ p.name }}</td>
                        <td>{{ p.version }}</td>
                        <td><span :class="'status-' + p.status.toLowerCase()">{{ p.status }}</span></td>
                    </tr>
                </tbody>
            </table>
            <p v-else>Querying plugin registry...</p>
        </div>
    `
};
window.PluginRegistryDisplay = PluginRegistryDisplay;