// Shared logic for all monitoring display components
const SharedDisplayMixin = {
    props: ['agentId'],
    data() {
        return {
            liveData: null,
            pollingInterval: null,
            error: null,
        };
    },
    beforeUnmount() {
        if (this.pollingInterval) {
            clearInterval(this.pollingInterval);
        }
    },
    methods: {
        async fetchData(endpoint, dataKey) {
            if (!this.agentId) return;
            
            try {
                const token = localStorage.getItem('token');
                const response = await fetch(endpoint, {
                    headers: {
                        'Authorization': `Bearer ${token}`,
                        'Accept': 'application/json'
                    }
                });

                if (!response.ok) {
                    if (response.status === 401) {
                        throw new Error("Session expired. Please login again.");
                    }
                    const errorText = await response.text();
                    throw new Error(`HTTP error! status: ${response.status}`);
                }

                const data = await response.json();
                // Handle both wrapped CommandResponse and direct arrays
                this.liveData = dataKey ? data[dataKey] : data;
                this.error = null;
            } catch (e) {
                console.error(`Failed to fetch data for ${endpoint}:`, e);
                this.error = e.message;
            }
        }
    }
};

window.SharedDisplayMixin = SharedDisplayMixin;