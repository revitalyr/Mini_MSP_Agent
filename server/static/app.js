// Mini MSP Agent Dashboard JavaScript
class AgentDashboard {
    constructor() {
        this.agents = new Map();
        this.ws = null;
        this.reconnectInterval = null;
        this.isConnecting = false;
        
        this.init();
    }

    init() {
        this.setupEventListeners();
        this.connectWebSocket();
        this.loadAgents();
        feather.replace();
        
        // Update last update time every minute
        setInterval(() => this.updateLastUpdateTime(), 60000);
    }

    setupEventListeners() {
        // Refresh button
        document.getElementById('refreshBtn').addEventListener('click', () => {
            this.loadAgents();
        });

        // Modal close
        document.getElementById('modalClose').addEventListener('click', () => {
            this.closeModal();
        });

        // Close modal on outside click
        document.getElementById('agentModal').addEventListener('click', (e) => {
            if (e.target.id === 'agentModal') {
                this.closeModal();
            }
        });
    }

    async connectWebSocket() {
        if (this.isConnecting) return;
        this.isConnecting = true;

        try {
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const wsUrl = `${protocol}//${window.location.host}/ws`;
            
            this.ws = new WebSocket(wsUrl);
            
            this.ws.onopen = () => {
                console.log('WebSocket connected');
                this.updateConnectionStatus(true);
                this.isConnecting = false;
                
                if (this.reconnectInterval) {
                    clearInterval(this.reconnectInterval);
                    this.reconnectInterval = null;
                }
            };

            this.ws.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    this.handleWebSocketMessage(data);
                } catch (error) {
                    console.error('Error parsing WebSocket message:', error);
                }
            };

            this.ws.onclose = () => {
                console.log('WebSocket disconnected');
                this.updateConnectionStatus(false);
                this.isConnecting = false;
                
                // Attempt to reconnect after 5 seconds
                if (!this.reconnectInterval) {
                    this.reconnectInterval = setInterval(() => {
                        if (!this.isConnecting) {
                            this.connectWebSocket();
                        }
                    }, 5000);
                }
            };

            this.ws.onerror = (error) => {
                console.error('WebSocket error:', error);
                this.updateConnectionStatus(false);
                this.isConnecting = false;
            };

        } catch (error) {
            console.error('Error connecting WebSocket:', error);
            this.updateConnectionStatus(false);
            this.isConnecting = false;
        }
    }

    handleWebSocketMessage(data) {
        console.log('WebSocket message:', data);
        
        switch (data.type) {
            case 'heartbeat':
                this.updateAgentHeartbeat(data.agent_id, data.data);
                break;
            case 'register':
                this.addAgent(data.data);
                break;
            case 'unregister':
                this.removeAgent(data.agent_id);
                break;
            default:
                console.log('Unknown message type:', data.type);
        }
    }

    updateConnectionStatus(connected) {
        const statusDot = document.getElementById('connectionStatus');
        const statusText = document.getElementById('statusText');
        
        if (connected) {
            statusDot.className = 'status-dot connected';
            statusText.textContent = 'Connected';
        } else {
            statusDot.className = 'status-dot disconnected';
            statusText.textContent = 'Disconnected';
        }
    }

    async loadAgents() {
        try {
            const response = await fetch('/agents');
            const data = await response.json();
            
            this.agents.clear();
            data.agents.forEach(agent => {
                this.agents.set(agent.id, {
                    ...agent,
                    last_seen: Date.now() - (agent.last_heartbeat * 1000)
                });
            });
            
            this.updateAgentsDisplay();
            this.updateStats();
            
        } catch (error) {
            console.error('Error loading agents:', error);
            this.showError('Failed to load agents');
        }
    }

    updateAgentsDisplay() {
        const agentsGrid = document.getElementById('agentsGrid');
        const emptyState = document.getElementById('emptyState');
        
        if (this.agents.size === 0) {
            agentsGrid.style.display = 'none';
            emptyState.style.display = 'block';
        } else {
            agentsGrid.style.display = 'grid';
            emptyState.style.display = 'none';
            
            agentsGrid.innerHTML = '';
            this.agents.forEach((agent, id) => {
                const card = this.createAgentCard(id, agent);
                agentsGrid.appendChild(card);
            });
        }
        
        feather.replace();
    }

    createAgentCard(id, agent) {
        const card = document.createElement('div');
        card.className = 'agent-card';
        card.onclick = () => this.showAgentDetails(id, agent);
        
        const isActive = this.isAgentActive(agent);
        const uptime = this.formatUptime(agent.uptime);
        
        card.innerHTML = `
            <div class="agent-header">
                <div class="agent-id">${id}</div>
                <div class="agent-status">
                    <span class="status-indicator-small ${isActive ? 'active' : 'inactive'}"></span>
                    <span>${isActive ? 'Active' : 'Inactive'}</span>
                </div>
            </div>
            <div class="agent-info">
                <div class="info-row">
                    <span class="info-label">Hostname:</span>
                    <span class="info-value">${agent.hostname || 'Unknown'}</span>
                </div>
                <div class="info-row">
                    <span class="info-label">Uptime:</span>
                    <span class="info-value">${uptime}</span>
                </div>
                <div class="info-row">
                    <span class="info-label">Last seen:</span>
                    <span class="info-value">${this.formatLastSeen(agent.last_seen)}</span>
                </div>
            </div>
        `;
        
        return card;
    }

    showAgentDetails(id, agent) {
        const modal = document.getElementById('agentModal');
        const modalTitle = document.getElementById('modalTitle');
        const modalBody = document.getElementById('modalBody');
        
        modalTitle.textContent = `Agent: ${id}`;
        
        const isActive = this.isAgentActive(agent);
        const uptime = this.formatUptime(agent.uptime);
        
        modalBody.innerHTML = `
            <div class="agent-details">
                <div class="detail-section">
                    <h4>Basic Information</h4>
                    <div class="detail-grid">
                        <div class="detail-item">
                            <span class="detail-label">Agent ID:</span>
                            <span class="detail-value">${id}</span>
                        </div>
                        <div class="detail-item">
                            <span class="detail-label">Hostname:</span>
                            <span class="detail-value">${agent.hostname || 'Unknown'}</span>
                        </div>
                        <div class="detail-item">
                            <span class="detail-label">Status:</span>
                            <span class="detail-value">
                                <span class="status-indicator-small ${isActive ? 'active' : 'inactive'}"></span>
                                ${isActive ? 'Active' : 'Inactive'}
                            </span>
                        </div>
                        <div class="detail-item">
                            <span class="detail-label">Uptime:</span>
                            <span class="detail-value">${uptime}</span>
                        </div>
                    </div>
                </div>
                
                <div class="detail-section">
                    <h4>Connection Information</h4>
                    <div class="detail-grid">
                        <div class="detail-item">
                            <span class="detail-label">Last Heartbeat:</span>
                            <span class="detail-value">${this.formatLastSeen(agent.last_seen)}</span>
                        </div>
                        <div class="detail-item">
                            <span class="detail-label">Connection Time:</span>
                            <span class="detail-value">${new Date(agent.last_seen).toLocaleString()}</span>
                        </div>
                    </div>
                </div>
                
                <div class="detail-actions">
                    <button class="btn btn-secondary" onclick="dashboard.sendCommand('${id}', 'status')">
                        <i data-feather="terminal"></i>
                        Send Status Command
                    </button>
                    <button class="btn btn-secondary" onclick="dashboard.sendCommand('${id}', 'restart')">
                        <i data-feather="refresh-cw"></i>
                        Restart Agent
                    </button>
                </div>
            </div>
        `;
        
        modal.classList.add('active');
        feather.replace();
    }

    closeModal() {
        const modal = document.getElementById('agentModal');
        modal.classList.remove('active');
    }

    async sendCommand(agentId, command) {
        try {
            let commandData;
            
            switch (command) {
                case 'status':
                    commandData = {
                        type: "GetSystemInfo"
                    };
                    break;
                case 'restart':
                    commandData = {
                        type: "Exec",
                        data: {
                            cmd: "echo 'Restart command received' && sleep 1"
                        }
                    };
                    break;
                default:
                    commandData = {
                        type: "Exec",
                        data: {
                            cmd: command
                        }
                    };
            }
            
            const response = await fetch(`/agents/${agentId}/command`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(commandData)
            });
            
            if (response.ok) {
                this.showSuccess(`Command sent to agent ${agentId}`);
            } else {
                const errorText = await response.text();
                throw new Error(`Failed to send command: ${errorText}`);
            }
        } catch (error) {
            console.error('Error sending command:', error);
            this.showError(`Failed to send command: ${error.message}`);
        }
    }

    updateAgentHeartbeat(agentId, data) {
        if (this.agents.has(agentId)) {
            const agent = this.agents.get(agentId);
            agent.last_seen = Date.now();
            if (data.uptime) agent.uptime = data.uptime;
            if (data.hostname) agent.hostname = data.hostname;
            
            this.agents.set(agentId, agent);
            this.updateAgentsDisplay();
            this.updateStats();
        }
    }

    addAgent(agentData) {
        this.agents.set(agentData.id, {
            ...agentData,
            last_seen: Date.now()
        });
        this.updateAgentsDisplay();
        this.updateStats();
    }

    removeAgent(agentId) {
        this.agents.delete(agentId);
        this.updateAgentsDisplay();
        this.updateStats();
    }

    updateStats() {
        const totalAgents = this.agents.size;
        const activeAgents = Array.from(this.agents.values()).filter(agent => 
            this.isAgentActive(agent)
        ).length;
        
        document.getElementById('totalAgents').textContent = totalAgents;
        document.getElementById('activeAgents').textContent = activeAgents;
        document.getElementById('systemStatus').textContent = 
            activeAgents > 0 ? 'Healthy' : 'Warning';
        
        this.updateLastUpdateTime();
    }

    updateLastUpdateTime() {
        const now = new Date();
        const timeString = now.toLocaleTimeString();
        document.getElementById('lastUpdate').textContent = timeString;
    }

    isAgentActive(agent) {
        const now = Date.now();
        const lastSeen = agent.last_seen || 0;
        return (now - lastSeen) < 120000; // 2 minutes
    }

    formatUptime(seconds) {
        if (!seconds) return 'Unknown';
        
        const days = Math.floor(seconds / 86400);
        const hours = Math.floor((seconds % 86400) / 3600);
        const minutes = Math.floor((seconds % 3600) / 60);
        
        if (days > 0) {
            return `${days}d ${hours}h ${minutes}m`;
        } else if (hours > 0) {
            return `${hours}h ${minutes}m`;
        } else {
            return `${minutes}m`;
        }
    }

    formatLastSeen(timestamp) {
        if (!timestamp) return 'Never';
        
        const now = Date.now();
        const diff = now - timestamp;
        
        if (diff < 60000) {
            return 'Just now';
        } else if (diff < 3600000) {
            const minutes = Math.floor(diff / 60000);
            return `${minutes} min ago`;
        } else if (diff < 86400000) {
            const hours = Math.floor(diff / 3600000);
            return `${hours} hours ago`;
        } else {
            return new Date(timestamp).toLocaleDateString();
        }
    }

    showError(message) {
        // Simple error notification - could be enhanced with a toast library
        console.error(message);
        alert(`Error: ${message}`);
    }

    showSuccess(message) {
        // Simple success notification - could be enhanced with a toast library
        console.log(message);
        alert(`Success: ${message}`);
    }
}

// Initialize dashboard when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    window.dashboard = new AgentDashboard();
});

// Add some CSS for the modal details
const additionalStyles = `
.detail-section {
    margin-bottom: 25px;
}

.detail-section h4 {
    font-size: 1.1rem;
    font-weight: 600;
    color: #1f2937;
    margin-bottom: 15px;
    padding-bottom: 8px;
    border-bottom: 1px solid #e5e7eb;
}

.detail-grid {
    display: grid;
    gap: 12px;
}

.detail-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 0;
}

.detail-label {
    font-weight: 500;
    color: #6b7280;
}

.detail-value {
    color: #1f2937;
    font-weight: 500;
}

.detail-actions {
    display: flex;
    gap: 12px;
    margin-top: 20px;
    padding-top: 20px;
    border-top: 1px solid #e5e7eb;
}

.status-indicator-small.inactive {
    background: #ef4444;
}
`;

// Inject additional styles
const styleSheet = document.createElement('style');
styleSheet.textContent = additionalStyles;
document.head.appendChild(styleSheet);
