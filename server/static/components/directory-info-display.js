// Directory Info Display Component
const DirectoryInfoDisplay = {
    mixins: [SharedDisplayMixin],
    props: ['pluginConfig'], 
    async mounted() {
        await this.fetchDirectoryInfo();
        this.pollingInterval = setInterval(this.fetchDirectoryInfo, 5000);
    },
    methods: {
        async fetchDirectoryInfo() {
            if (!this.pluginConfig?.targetPath) return;
            
            // For demo, show mock data
            if (this.agentId === 'demo-agent-001') {
                this.liveData = {
                    files: [
                        {name: 'demo.txt', size: 1024},
                        {name: 'test.doc', size: 2048}
                    ],
                    totalSize: 3072,
                    path: this.pluginConfig.targetPath
                };
                return;
            }
            
            const params = new URLSearchParams({
                path: this.pluginConfig.targetPath,
                recursive: this.pluginConfig.recursive ? 'true' : 'false',
                includeHidden: this.pluginConfig.includeHidden ? 'true' : 'false'
            }).toString();

            await this.fetchData(
                `/agents/${this.agentId}/data/directory_info?${params}`, 
                'DirectoryInfo'
            );
        }
    },
    template: `
        <div class="data-display-section">
            <h4>📁 Directory Information</h4>
            <div v-if="liveData && liveData.files" class="directory-stats">
                <div class="stat-row">
                    <span class="stat-label">Total Files:</span>
                    <span class="stat-value">{{ liveData.files.length || 0 }}</span>
                </div>
                <div class="stat-row">
                    <span class="stat-label">Total Size:</span>
                    <span class="stat-value">{{ formatFileSize(liveData.totalSize || 0) }}</span>
                </div>
                <div class="stat-row">
                    <span class="stat-label">Path:</span>
                    <span class="stat-value">{{ pluginConfig?.targetPath || 'N/A' }}</span>
                </div>
            </div>
            <p v-else-if="!error">Querying directory information...</p>
            <div v-else class="error-message">{{ error }}</div>
        </div>
    `
};

// Export for use in other modules
if (typeof window !== 'undefined') {
    window.DirectoryInfoDisplay = DirectoryInfoDisplay;
} else if (typeof module !== 'undefined' && module.exports) {
    module.exports = DirectoryInfoDisplay;
}