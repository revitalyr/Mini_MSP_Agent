// Directory Info Display Component
const DirectoryInfoDisplay = {
    props: ['agentId', 'pluginConfig'], // agentId to know which agent to query, pluginConfig for current settings
    data() {
        return {
            liveData: null,
            pollingInterval: null,
            error: null,
        };
    },
    async mounted() {
        // Initial fetch
        await this.fetchDirectoryInfo();
        // Start polling
        this.pollingInterval = setInterval(this.fetchDirectoryInfo, 5000); // Poll every 5 seconds
    },
    beforeUnmount() {
        clearInterval(this.pollingInterval);
    },
    methods: {
        async fetchDirectoryInfo() {
            if (!this.agentId || !this.pluginConfig || !this.pluginConfig.targetPath) {
                this.error = "Agent ID or target path not available.";
                this.liveData = null;
                return;
            }
            try {
                const params = new URLSearchParams({
                    path: this.pluginConfig.targetPath,
                    include_subdirs: this.pluginConfig.includeSubdirs || false,
                    show_hidden: this.pluginConfig.showHidden || false,
                    max_depth: this.pluginConfig.maxDepth || 1,
                }).toString();

                const token = localStorage.getItem('token');
                const response = await fetch(`/agents/${this.agentId}/data/directory_info?${params}`, {
                    headers: {
                        'Authorization': `Bearer ${token}`,
                        'Accept': 'application/json'
                    }
                });

                if (!response.ok) {
                    if (response.status === 401) {
                        this.error = "Session expired. Please login again.";
                        return;
                    }
                    const errorText = await response.text();
                    throw new Error(`HTTP error! status: ${response.status} - ${errorText}`);
                }
                const data = await response.json();
                if (data.DirectoryInfo) { // Assuming CommandResponse is wrapped
                    this.liveData = data.DirectoryInfo;
                    this.error = null;
                } else {
                    this.error = "Unexpected response format or no DirectoryInfo data.";
                    this.liveData = null;
                }
            } catch (e) {
                console.error("Failed to fetch directory info:", e);
                this.error = e.message;
                this.liveData = null;
            }
        },
    },
    template: `
        <div class="data-display-section">
            <h4>📁 Live Directory Info for: {{ pluginConfig.targetPath }}</h4>
            <div v-if="liveData">
                <p>Total Files: {{ liveData.total_files }}</p>
                <p>Total Directories: {{ liveData.total_directories }}</p>
                <p>Total Size: {{ (liveData.total_size_bytes / (1024 * 1024)).toFixed(2) }} MB</p>
                <p>Hidden Files: {{ liveData.hidden_files }}</p>
                <p>Hidden Dirs: {{ liveData.hidden_directories }}</p>
                <p>Scan Progress: {{ liveData.scan_progress }}%</p>
                <p>Last Scan: {{ new Date(liveData.scan_timestamp * 1000).toLocaleString() }}</p>
            </div>
            <div v-else-if="error" class="error-message">Error: {{ error }}</div>
            <div v-else>Loading directory information...</div>
        </div>
    `
};

// Export for use in other modules
if (typeof window !== 'undefined') {
    window.DirectoryInfoDisplay = DirectoryInfoDisplay;
} else if (typeof module !== 'undefined' && module.exports) {
    module.exports = DirectoryInfoDisplay;
}