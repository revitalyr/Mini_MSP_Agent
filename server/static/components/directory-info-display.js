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
            if (!this.pluginConfig?.targetPath) {
                this.liveData = null;
                return;
            }

            const params = new URLSearchParams({
                path: this.pluginConfig.targetPath,
                include_subdirs: this.pluginConfig.includeSubdirs || false,
                show_hidden: this.pluginConfig.showHidden || false,
                max_depth: this.pluginConfig.maxDepth || 1,
            }).toString();

            await this.fetchData(
                `/agents/${this.agentId}/data/directory_info?${params}`, 
                'DirectoryInfo'
            );
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