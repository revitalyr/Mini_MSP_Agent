const EventDataDisplay = {
    mixins: [SharedDisplayMixin],
    props: ['pluginConfig'],
    async mounted() {
        await this.fetchEventData();
        this.pollingInterval = setInterval(this.fetchEventData, 5000);
    },
    methods: {
        async fetchEventData() {
            if (!this.pluginConfig?.monitorPath) return;
            
            // For demo, show mock data
            if (this.agentId === 'demo-agent-001') {
                this.liveData = {
                    events_count: 42,
                    buffer_usage: 15,
                    last_event: 'File Created'
                };
                return;
            }
            
            await this.fetchData(
                `/agents/${this.agentId}/data/event_data?path=${encodeURIComponent(this.pluginConfig.monitorPath)}`,
                'EventData'
            );
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