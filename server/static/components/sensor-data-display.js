const SensorDataDisplay = {
    mixins: [SharedDisplayMixin],
    async mounted() {
        await this.fetchSensorData();
        this.pollingInterval = setInterval(this.fetchSensorData, 2000); // Опрос каждые 2 секунды
    },
    methods: {
        async fetchSensorData() {
            await this.fetchData(`/agents/${this.agentId}/data/sensors`, 'SensorData');
        }
    },
    template: `
        <div class="data-display-section card">
            <h4>🌡️ Мониторинг датчиков</h4>
            <div v-if="liveData">
                <div class="metric">
                    <span class="label">Температура:</span>
                    <span class="value">{{ liveData.temperature }}°C</span>
                </div>
                <div class="metric">
                    <span class="label">Влажность:</span>
                    <span class="value">{{ liveData.humidity }}%</span>
                </div>
                <small>Последнее обновление: {{ new Date(liveData.timestamp).toLocaleTimeString() }}</small>
            </div>
            <div v-else-if="error" class="error-text">{{ error }}</div>
            <div v-else>Ожидание данных...</div>
        </div>
    `
};
window.SensorDataDisplay = SensorDataDisplay;