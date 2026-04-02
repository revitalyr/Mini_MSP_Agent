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
        async fetchData(endpoint, dataKey, isRetry = false) {
            if (!this.agentId) return;
            
            try {
                const token = localStorage.getItem('token');
                let response = await fetch(endpoint, {
                    headers: {
                        'Authorization': `Bearer ${token}`,
                        'Accept': 'application/json'
                    }
                });

                if (!response.ok) {
                    if (response.status === 401 && !isRetry) {
                        const refreshed = await this.refreshAccessToken();
                        if (refreshed) {
                            return this.fetchData(endpoint, dataKey, true);
                        }
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
        },

        async refreshAccessToken() {
            const refreshToken = localStorage.getItem('refreshToken');
            if (!refreshToken) return false;

            try {
                const response = await fetch('/refresh', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ refresh_token: refreshToken })
                });

                if (response.ok) {
                    const data = await response.json();
                    localStorage.setItem('token', data.token);
                    console.log("Access token refreshed successfully");
                    return true;
                }
            } catch (e) {
                console.error("Token refresh failed", e);
            }

            localStorage.removeItem('token');
            localStorage.removeItem('refreshToken');
            return false;
        }
    }
};

window.SharedDisplayMixin = SharedDisplayMixin;