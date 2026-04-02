const EventDataDisplay = {
    props: ['agentId', 'pluginConfig'],
    data() {
        return { liveData: null, pollingInterval: null, error: null };
    },
    async mounted() {
        await this.fetchData();
        this.pollingInterval = setInterval(this.fetchData, 5000);
    },
    beforeUnmount() { clearInterval(this.pollingInterval); },
    methods: {
        async fetchData() {
            if (!this.agentId || !this.pluginConfig?.monitorPath) return;
            try {
                const token = localStorage.getItem('token');
                const response = await fetch(`/agents/${this.agentId}/data/event_data?path=${encodeURIComponent(this.pluginConfig.monitorPath)}`, {
                    headers: { 'Authorization': `Bearer ${token}` }
                });
                const data = await response.json();
                this.liveData = data.EventData;
            } catch (e) { this.error = e.message; }
        }
    },
    template: `
        <div class="data-display-section">
            <h4>📊 Live Event Stream: {{ pluginConfig.monitorPath }}</h4>
            <div v-if="liveData">
                <p>Events Processed: {{ liveData.events_count }}</p>
                <p>Buffer Usage: {{ liveData.buffer_usage }}%</p>
                <p>Last Event Type: <strong>{{ liveData.last_event }}</strong></p>
            </div>
            <div v-else-if="error" class="error-message">{{ error }}</div>
        </div>
    `
};
window.EventDataDisplay = EventDataDisplay;