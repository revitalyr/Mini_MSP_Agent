class AgentDashboard {
    constructor() {
        this.agents = new Map();
        this.selectedAgent = null;
        this.commandResponses = [];
        this.init();
    }

    async init() {
        try {
            await this.connectWebSocket();
            this.loadAgents();
            this.setupEventListeners();
            this.updateAgentSelector();
            feather.replace();
        } catch (error) {
            console.error('Failed to initialize dashboard:', error);
            this.showError('Failed to initialize dashboard');
        }
    }

    async connectWebSocket() {
        try {
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const wsUrl = `${protocol}//${window.location.host}/ws`;
            
            this.ws = new WebSocket(wsUrl);
            
            this.ws.onopen = () => {
                console.log('WebSocket connected');
                this.updateConnectionStatus(true);
            };
            
            this.ws.onmessage = (event) => {
                this.handleWebSocketMessage(event);
            };
            
            this.ws.onclose = () => {
                console.log('WebSocket disconnected');
                this.updateConnectionStatus(false);
                // Attempt to reconnect after 5 seconds
                setTimeout(() => this.connectWebSocket(), 5000);
            };
            
            this.ws.onerror = (error) => {
                console.error('WebSocket error:', error);
                this.updateConnectionStatus(false);
            };
        } catch (error) {
            console.error('Failed to connect WebSocket:', error);
            this.updateConnectionStatus(false);
        }
    }

    handleWebSocketMessage(event) {
        try {
            const data = JSON.parse(event.data);
            
            switch (data.type) {
                case 'heartbeat':
                    this.updateAgentHeartbeat(data.agent_id, data);
                    break;
                case 'command_response':
                    this.handleCommandResponse(data);
                    break;
                case 'system_info':
                    this.handleSystemInfoResponse(data);
                    break;
                case 'processes':
                    this.handleProcessesResponse(data);
                    break;
                default:
                    console.log('Unknown message type:', data.type);
            }
        } catch (error) {
            console.error('Failed to parse WebSocket message:', error);
        }
    }

    handleCommandResponse(response) {
        const responseItem = {
            id: Date.now(),
            agentId: response.agent_id,
            command: response.command,
            status: response.status,
            data: response.data,
            timestamp: new Date().toLocaleString()
        };
        
        this.commandResponses.unshift(responseItem);
        this.updateCommandResponses();
        this.showSuccess(`Command executed on ${response.agent_id}`);
    }

    handleSystemInfoResponse(data) {
        this.showSuccess(`System info received from ${data.agent_id}`);
        // Could update modal with system info
    }

    handleProcessesResponse(data) {
        this.showSuccess(`Process list received from ${data.agent_id}`);
        // Could update modal with process list
    }

    async loadAgents() {
        try {
            const response = await fetch('/agents');
            const data = await response.json();
            
            data.agents.forEach(agent => {
                this.agents.set(agent.id, {
                    ...agent,
                    last_seen: Date.now()
                });
            });
            
            this.updateAgentsDisplay();
            this.updateStats();
        } catch (error) {
            console.error('Failed to load agents:', error);
            this.showError('Failed to load agents');
        }
    }

    updateConnectionStatus(connected) {
        const statusDot = document.getElementById('connectionStatus');
        const statusText = document.getElementById('connectionText');
        
        if (connected) {
            statusDot.classList.add('connected');
            statusText.textContent = 'Connected';
        } else {
            statusDot.classList.remove('connected');
            statusText.textContent = 'Disconnected';
        }
    }

    updateAgentSelector() {
        const selector = document.getElementById('agentSelect');
        selector.innerHTML = '<option value="">Choose an agent...</option>';
        
        this.agents.forEach((agent, id) => {
            const option = document.createElement('option');
            option.value = id;
            option.textContent = `${agent.hostname} (${id})`;
            selector.appendChild(option);
        });
        
        selector.addEventListener('change', (e) => {
            this.selectedAgent = e.target.value;
        });
    }

    updateAgentsDisplay() {
        const grid = document.getElementById('agentsGrid');
        grid.innerHTML = '';
        
        this.agents.forEach((agent, id) => {
            const card = this.createAgentCard(id, agent);
            grid.appendChild(card);
        });
        
        feather.replace();
    }

    createAgentCard(id, agent) {
        const card = document.createElement('div');
        card.className = 'agent-card';
        card.onclick = () => this.showAgentDetails(id);
        
        const isOnline = this.isAgentActive(agent);
        
        card.innerHTML = `
            <div class="agent-header">
                <div class="agent-name">${agent.hostname}</div>
                <div class="agent-status ${isOnline ? 'online' : 'offline'}">
                    ${isOnline ? 'Online' : 'Offline'}
                </div>
            </div>
            <div class="agent-metrics">
                <div class="metric">
                    <div class="metric-value">${agent.uptime ? this.formatUptime(agent.uptime) : 'N/A'}</div>
                    <div class="metric-label">Uptime</div>
                </div>
                <div class="metric">
                    <div class="metric-value">${agent.cpu ? agent.cpu.toFixed(1) + '%' : 'N/A'}</div>
                    <div class="metric-label">CPU</div>
                </div>
                <div class="metric">
                    <div class="metric-value">${agent.ram ? agent.ram.toFixed(1) + '%' : 'N/A'}</div>
                    <div class="metric-label">RAM</div>
                </div>
            </div>
        `;
        
        return card;
    }

    showAgentDetails(agentId) {
        const agent = this.agents.get(agentId);
        if (!agent) return;
        
        this.selectedAgent = agentId;
        
        const modal = document.getElementById('agentModal');
        const modalTitle = document.getElementById('modalTitle');
        const modalBody = document.getElementById('modalBody');
        
        modalTitle.textContent = `Agent Details: ${agent.hostname}`;
        
        const isOnline = this.isAgentActive(agent);
        
        modalBody.innerHTML = `
            <div style="display: grid; gap: 1rem;">
                <div><strong>Agent ID:</strong> ${agentId}</div>
                <div><strong>Hostname:</strong> ${agent.hostname}</div>
                <div><strong>Status:</strong> <span class="agent-status ${isOnline ? 'online' : 'offline'}">${isOnline ? 'Online' : 'Offline'}</span></div>
                <div><strong>Uptime:</strong> ${agent.uptime ? this.formatUptime(agent.uptime) : 'N/A'}</div>
                <div><strong>CPU Usage:</strong> ${agent.cpu ? agent.cpu.toFixed(1) + '%' : 'N/A'}</div>
                <div><strong>RAM Usage:</strong> ${agent.ram ? agent.ram.toFixed(1) + '%' : 'N/A'}</div>
                <div><strong>Disk Usage:</strong> ${agent.disk ? agent.disk.toFixed(1) + '%' : 'N/A'}</div>
                <div><strong>Last Seen:</strong> ${new Date(agent.last_seen).toLocaleString()}</div>
            </div>
        `;
        
        modal.classList.add('active');
        feather.replace();
    }

    closeModal() {
        const modal = document.getElementById('agentModal');
        modal.classList.remove('active');
    }

    showCustomCommandDialog() {
        if (!this.selectedAgent) {
            this.showError('Please select an agent first');
            return;
        }
        
        const modal = document.getElementById('customCommandModal');
        modal.classList.add('active');
    }

    closeCustomCommandDialog() {
        const modal = document.getElementById('customCommandModal');
        modal.classList.remove('active');
    }

    async executeCustomCommand() {
        const commandInput = document.getElementById('customCommand');
        const command = commandInput.value.trim();
        
        if (!command) {
            this.showError('Please enter a command');
            return;
        }
        
        if (!this.selectedAgent) {
            this.showError('Please select an agent first');
            return;
        }
        
        await this.sendCommandToAgent('Exec', { cmd: command });
        this.closeCustomCommandDialog();
        commandInput.value = '';
    }

    async sendCommand(commandType) {
        if (!this.selectedAgent) {
            this.showError('Please select an agent first');
            return;
        }
        
        let commandData;
        
        switch (commandType) {
            case 'GetSystemInfo':
                commandData = { type: 'GetSystemInfo' };
                break;
            case 'GetProcesses':
                commandData = { type: 'GetProcesses' };
                break;
            default:
                commandData = { type: 'Exec', data: { cmd: commandType } };
        }
        
        await this.sendCommandToAgent(commandData.type, commandData.data || {});
    }

    async sendCommandToAgent(type, data = {}) {
        if (!this.selectedAgent) {
            this.showError('Please select an agent first');
            return;
        }
        
        try {
            const commandPayload = { type };
            if (Object.keys(data).length > 0) {
                commandPayload.data = data;
            }
            
            const response = await fetch(`/agents/${this.selectedAgent}/command`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(commandPayload)
            });
            
            if (response.ok) {
                const result = await response.json();
                console.log('Command sent:', result);
                this.showSuccess(`Command sent to ${this.selectedAgent}`);
            } else {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }
        } catch (error) {
            console.error('Failed to send command:', error);
            this.showError(`Failed to send command: ${error.message}`);
        }
    }

    updateCommandResponses() {
        const container = document.getElementById('responsesContainer');
        
        if (this.commandResponses.length === 0) {
            container.innerHTML = '<p class="no-responses">No commands executed yet</p>';
            return;
        }
        
        container.innerHTML = this.commandResponses.map(response => `
            <div class="response-item">
                <div class="response-header">
                    <div class="response-command">${this.formatCommandDisplay(response.command)}</div>
                    <div class="response-timestamp">${response.timestamp}</div>
                </div>
                <div class="response-status ${response.status}">
                    ${response.status}
                </div>
                ${response.data ? `<div class="response-content">${this.formatResponseData(response.data)}</div>` : ''}
            </div>
        `).join('');
    }

    formatCommandDisplay(command) {
        if (typeof command === 'string') return command;
        if (command.type) {
            switch (command.type) {
                case 'GetSystemInfo': return 'Get System Info';
                case 'GetProcesses': return 'Get Processes';
                case 'Exec': return `Exec: ${command.data?.cmd || 'Unknown'}`;
                default: return command.type;
            }
        }
        return JSON.stringify(command);
    }

    formatResponseData(data) {
        if (typeof data === 'string') return data;
        if (data.output) return data.output;
        if (data.processes) return `${data.processes.length} processes`;
        if (data.error) return `Error: ${data.error}`;
        return JSON.stringify(data, null, 2);
    }

    updateAgentHeartbeat(agentId, data) {
        if (this.agents.has(agentId)) {
            const agent = this.agents.get(agentId);
            agent.last_seen = Date.now();
            if (data.uptime) agent.uptime = data.uptime;
            if (data.hostname) agent.hostname = data.hostname;
            if (data.cpu) agent.cpu = data.cpu;
            if (data.ram) agent.ram = data.ram;
            if (data.disk) agent.disk = data.disk;
            
            this.updateAgentsDisplay();
            this.updateStats();
        }
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
        const threshold = 60000; // 1 minute
        return (now - lastSeen) < threshold;
    }

    formatUptime(seconds) {
        if (!seconds) return 'N/A';
        
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

    setupEventListeners() {
        // Modal close events
        document.addEventListener('click', (e) => {
            if (e.target.classList.contains('modal')) {
                e.target.classList.remove('active');
            }
        });
        
        // Keyboard shortcuts
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') {
                this.closeModal();
                this.closeCustomCommandDialog();
            }
        });
    }

    showSuccess(message) {
        this.showNotification(message, 'success');
    }

    showError(message) {
        this.showNotification(message, 'error');
    }

    showNotification(message, type) {
        // Create notification element
        const notification = document.createElement('div');
        notification.style.cssText = `
            position: fixed;
            top: 20px;
            right: 20px;
            padding: 1rem 1.5rem;
            background: ${type === 'success' ? '#10b981' : '#ef4444'};
            color: white;
            border-radius: 8px;
            box-shadow: 0 4px 15px rgba(0, 0, 0, 0.2);
            z-index: 10000;
            font-weight: 600;
            transform: translateX(100%);
            transition: transform 0.3s ease;
        `;
        notification.textContent = message;
        
        document.body.appendChild(notification);
        
        // Animate in
        setTimeout(() => {
            notification.style.transform = 'translateX(0)';
        }, 100);
        
        // Remove after 3 seconds
        setTimeout(() => {
            notification.style.transform = 'translateX(100%)';
            setTimeout(() => {
                if (notification.parentNode) {
                    notification.parentNode.removeChild(notification);
                }
            }, 300);
        }, 3000);
    }
}

// Initialize dashboard when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    window.dashboard = new AgentDashboard();
});
