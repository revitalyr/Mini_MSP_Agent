const VideoStreamDisplay = {
    mixins: [SharedDisplayMixin],
    data() {
        return {
            frameUrl: null
        };
    },
    async mounted() {
        this.pollingInterval = setInterval(this.fetchFrame, 200); // 5 FPS
    },
    methods: {
        async fetchFrame() {
            if (!this.agentId) return;
            try {
                const token = localStorage.getItem('token');
                const response = await fetch(`/agents/${this.agentId}/data/video_frame`, {
                    headers: { 'Authorization': `Bearer ${token}` }
                });
                
                if (response.ok) {
                    const blob = await response.blob();
                    if (this.frameUrl) {
                        URL.revokeObjectURL(this.frameUrl); // Освобождаем память предыдущего кадра
                    }
                    this.frameUrl = URL.createObjectURL(blob);
                }
            } catch (e) {
                console.error("Frame fetch failed", e);
            }
        }
    },
    template: `
        <div class="data-display-section card video-monitor">
            <h4>📹 Live Video Stream</h4>
            <div class="video-container" style="background: #000; min-height: 240px; display: flex; justify-content: center;">
                <img v-if="frameUrl" :src="frameUrl" style="max-width: 100%; height: auto;" />
                <div v-else class="placeholder" style="color: #666; align-self: center;">
                    No signal...
                </div>
            </div>
            <div v-if="liveData" class="video-stats">
                <small>Res: {{liveData.width}}x{{liveData.height}} | Format: {{liveData.format}}</small>
            </div>
            <div v-if="error" class="error-text">{{ error }}</div>
        </div>
    `
};
window.VideoStreamDisplay = VideoStreamDisplay;