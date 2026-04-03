// Demo Agent for Testing
const demoAgent = {
    id: 'demo-agent-001',
    hostname: 'Demo Agent',
    status: 'online',
    last_seen: Date.now(),
    cpu: 15,
    ram: 45,
    disk: 60,
    uptime: '2h 34m',
    version: '1.0.0'
};

// Mock agents endpoint for testing
if (typeof window !== 'undefined') {
    // Override fetch for /agents endpoint
    const originalFetch = window.fetch;
    window.fetch = function(url, options) {
        if (url === '/agents') {
            return Promise.resolve({
                ok: true,
                json: () => Promise.resolve({
                    agents: [demoAgent]
                })
            });
        }
        return originalFetch.apply(this, arguments);
    };
}
