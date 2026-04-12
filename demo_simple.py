#!/usr/bin/env python3
"""
Simple Demo Server for Mini MSP Agent
Works without NATS - direct API communication
"""

from flask import Flask, jsonify, render_template_string
from flask_cors import CORS
import json
import time
import threading
import random
from datetime import datetime

app = Flask(__name__)
CORS(app)

# In-memory storage for agent data
agents_data = {}
system_metrics = {
    'start_time': time.time(),
    'total_messages': 0,
    'commands_sent': 0,
    'responses_received': 0
}

# HTML Template for simple dashboard
DASHBOARD_TEMPLATE = """
<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Mini MSP Agent - Simple Demo</title>
    <style>
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            margin: 0;
            padding: 20px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            min-height: 100vh;
        }
        
        .container {
            max-width: 1200px;
            margin: 0 auto;
        }
        
        .header {
            text-align: center;
            margin-bottom: 40px;
        }
        
        .header h1 {
            font-size: 2.5em;
            margin-bottom: 10px;
            text-shadow: 2px 2px 4px rgba(0,0,0,0.3);
        }
        
        .status-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 20px;
            margin-bottom: 40px;
        }
        
        .status-card {
            background: rgba(255, 255, 255, 0.1);
            backdrop-filter: blur(10px);
            border-radius: 15px;
            padding: 25px;
            border: 1px solid rgba(255, 255, 255, 0.2);
            transition: transform 0.3s ease;
        }
        
        .status-card:hover {
            transform: translateY(-5px);
            box-shadow: 0 10px 30px rgba(0,0,0,0.3);
        }
        
        .status-indicator {
            width: 12px;
            height: 12px;
            border-radius: 50%;
            display: inline-block;
            margin-right: 8px;
        }
        
        .status-online {
            background: #4ade80;
            box-shadow: 0 0 10px #4ade80;
            animation: pulse 2s infinite;
        }
        
        .status-offline {
            background: #ef4444;
        }
        
        @keyframes pulse {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.5; }
        }
        
        .metrics {
            background: rgba(255, 255, 255, 0.1);
            backdrop-filter: blur(10px);
            border-radius: 15px;
            padding: 30px;
            border: 1px solid rgba(255, 255, 255, 0.2);
        }
        
        .metric-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
        }
        
        .metric-item {
            text-align: center;
        }
        
        .metric-value {
            font-size: 2em;
            font-weight: bold;
            margin-bottom: 5px;
        }
        
        .metric-label {
            opacity: 0.8;
            font-size: 0.9em;
        }
        
        .agent-list {
            background: rgba(255, 255, 255, 0.1);
            backdrop-filter: blur(10px);
            border-radius: 15px;
            padding: 20px;
            margin-bottom: 20px;
        }
        
        .agent-item {
            background: rgba(255, 255, 255, 0.05);
            border-radius: 8px;
            padding: 15px;
            margin-bottom: 10px;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        
        .agent-info h4 {
            margin: 0 0 5px 0;
            color: #4ade80;
        }
        
        .agent-details {
            font-size: 0.9em;
            opacity: 0.8;
        }
        
        .plugins-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 15px;
            margin-top: 10px;
        }
        
        .plugin-card {
            background: rgba(255, 255, 255, 0.05);
            border-radius: 8px;
            padding: 15px;
            border-left: 3px solid #4ade80;
        }
        
        .plugin-name {
            font-weight: bold;
            color: #4ade80;
            margin-bottom: 5px;
        }
        
        .plugin-metrics {
            font-size: 0.85em;
            opacity: 0.9;
        }
        
        .controls {
            margin-top: 20px;
            text-align: center;
        }
        
        .btn {
            background: rgba(255, 255, 255, 0.2);
            border: 1px solid rgba(255, 255, 255, 0.3);
            color: white;
            padding: 12px 24px;
            border-radius: 8px;
            cursor: pointer;
            margin: 0 10px;
            transition: all 0.3s ease;
        }
        
        .btn:hover {
            background: rgba(255, 255, 255, 0.3);
        }
        
        .log {
            background: rgba(0, 0, 0, 0.3);
            border-radius: 10px;
            padding: 20px;
            margin-top: 20px;
            font-family: 'Courier New', monospace;
            font-size: 0.9em;
            max-height: 300px;
            overflow-y: auto;
        }
        
        .log-entry {
            margin-bottom: 5px;
            padding: 5px 10px;
            border-radius: 5px;
            background: rgba(255, 255, 255, 0.05);
        }
        
        .log-entry.info {
            border-left: 3px solid #3b82f6;
        }
        
        .log-entry.success {
            border-left: 3px solid #4ade80;
        }
        
        .log-entry.warning {
            border-left: 3px solid #fbbf24;
        }
        
        .log-entry.error {
            border-left: 3px solid #ef4444;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Mini MSP Agent - Simple Demo</h1>
            <p>Real-time agent monitoring with plugin data</p>
        </div>
        
        <div class="status-grid">
            <div class="status-card">
                <h3><span class="status-indicator status-online"></span>Agent Status</h3>
                <p><strong>Agent:</strong> <span id="agentStatus">Active</span></p>
                <p><strong>Messages:</strong> <span id="messageCount">0</span></p>
                <p><strong>Uptime:</strong> <span id="uptime">0m</span></p>
            </div>
            
            <div class="status-card">
                <h3><span class="status-indicator status-online"></span>API Server</h3>
                <p><strong>Status:</strong> Running</p>
                <p><strong>Port:</strong> 5001</p>
                <p><strong>Endpoints:</strong> 4 active</p>
            </div>
            
            <div class="status-card">
                <h3><span class="status-indicator status-online"></span>Performance</h3>
                <p><strong>Commands:</strong> <span id="commandCount">0</span></p>
                <p><strong>Responses:</strong> <span id="responseCount">0</span></p>
                <p><strong>Success Rate:</strong> <span id="successRate">100%</span></p>
            </div>
        </div>
        
        <div class="agent-list">
            <h3>Agent Information</h3>
            <div id="agentContainer">
                <p>Initializing agent data...</p>
            </div>
        </div>
        
        <div class="metrics">
            <h3>System Metrics</h3>
            <div class="metric-grid">
                <div class="metric-item">
                    <div class="metric-value" id="cpuUsage">0%</div>
                    <div class="metric-label">CPU Usage</div>
                </div>
                <div class="metric-item">
                    <div class="metric-value" id="memoryUsage">0MB</div>
                    <div class="metric-label">Memory Usage</div>
                </div>
                <div class="metric-item">
                    <div class="metric-value" id="diskUsage">0%</div>
                    <div class="metric-label">Disk Usage</div>
                </div>
                <div class="metric-item">
                    <div class="metric-value" id="networkIO">0Mbps</div>
                    <div class="metric-label">Network I/O</div>
                </div>
            </div>
        </div>
        
        <div class="agent-list">
            <h3>Plugin Status</h3>
            <div id="pluginContainer">
                <p>Loading plugin data...</p>
            </div>
        </div>
        
        <div class="controls">
            <button class="btn" onclick="sendCommand('ping')">Send Ping</button>
            <button class="btn" onclick="sendCommand('info')">Get Info</button>
            <button class="btn" onclick="sendCommand('status')">Get Status</button>
            <button class="btn" onclick="refreshData()">Refresh Data</button>
            <button class="btn" onclick="simulateAgent()">Simulate Agent</button>
        </div>
        
        <div class="log" id="logContainer">
            <div class="log-entry info">System initialized - waiting for agent data...</div>
        </div>
    </div>
    
    <script>
        let agents = [];
        let metrics = {
            startTime: Date.now(),
            commands: 0,
            responses: 0
        };
        
        function addLog(message, type = 'info') {
            const log = document.getElementById('logContainer');
            const entry = document.createElement('div');
            entry.className = `log-entry ${type}`;
            const time = new Date().toLocaleTimeString();
            entry.textContent = `[${time}] ${message}`;
            log.appendChild(entry);
            log.scrollTop = log.scrollHeight;
        }
        
        function updateMetrics() {
            document.getElementById('messageCount').textContent = system_metrics['total_messages'];
            document.getElementById('commandCount').textContent = system_metrics['commands_sent'];
            document.getElementById('responseCount').textContent = system_metrics['responses_received'];
            
            const successRate = system_metrics['commands_sent'] > 0 ? 
                Math.round((system_metrics['responses_received'] / system_metrics['commands_sent']) * 100) : 100;
            document.getElementById('successRate').textContent = successRate + '%';
            
            const uptime = Math.floor((Date.now() - metrics.startTime) / 60000);
            document.getElementById('uptime').textContent = uptime + 'm';
            
            // Update agent metrics if available
            if (agents.length > 0) {
                const agent = agents[0];
                if (agent.metrics) {
                    document.getElementById('cpuUsage').textContent = agent.metrics.cpu_usage.toFixed(1) + '%';
                    document.getElementById('memoryUsage').textContent = agent.metrics.memory_usage.toFixed(0) + 'MB';
                    document.getElementById('diskUsage').textContent = agent.metrics.disk_usage.toFixed(1) + '%';
                    document.getElementById('networkIO').textContent = agent.metrics.network_io.toFixed(1) + 'Mbps';
                }
            }
        }
        
        function updateAgentList() {
            const container = document.getElementById('agentContainer');
            if (agents.length === 0) {
                container.innerHTML = '<p>No agents connected</p>';
                document.getElementById('agentStatus').textContent = 'Offline';
                return;
            }
            
            document.getElementById('agentStatus').textContent = 'Online';
            
            const agent = agents[0];
            container.innerHTML = `
                <div class="agent-item">
                    <div class="agent-info">
                        <h4>${agent.hostname || 'Demo Agent'}</h4>
                        <div class="agent-details">
                            ID: ${agent.id || 'DEMO-001'}<br>
                            Version: ${agent.version || '1.0.0'}<br>
                            Status: <span style="color: #4ade80">${agent.status || 'Active'}</span><br>
                            Last Seen: ${agent.last_seen || 'Just now'}
                        </div>
                    </div>
                    <span class="status-indicator status-online"></span>
                </div>
            `;
        }
        
        function updatePluginList() {
            const container = document.getElementById('pluginContainer');
            if (agents.length === 0) {
                container.innerHTML = '<p>No plugin data available</p>';
                return;
            }
            
            const agent = agents[0];
            if (!agent.plugins || agent.plugins.length === 0) {
                container.innerHTML = '<p>No plugins loaded</p>';
                return;
            }
            
            const pluginsHtml = agent.plugins.map(plugin => `
                <div class="plugin-card">
                    <div class="plugin-name">${plugin.name}</div>
                    <div class="plugin-metrics">
                        Status: ${plugin.status}<br>
                        Version: ${plugin.version}<br>
                        Executions: ${plugin.execution_count}<br>
                        Avg Time: ${plugin.avg_execution_time.toFixed(1)}ms<br>
                        Errors: ${plugin.errors}
                    </div>
                </div>
            `).join('');
            
            container.innerHTML = pluginsHtml;
        }
        
        async function sendCommand(command) {
            metrics.commands++;
            addLog(`Sending command: ${command}`, 'info');
            
            try {
                const response = await fetch('/api/command', {
                    method: 'POST',
                    headers: {'Content-Type': 'application/json'},
                    body: JSON.stringify({command: command})
                });
                
                if (response.ok) {
                    const result = await response.json();
                    addLog(`Command sent: ${result.message}`, 'success');
                } else {
                    addLog(`Failed to send command: ${command}`, 'error');
                }
            } catch (error) {
                addLog(`Error sending command: ${error.message}`, 'error');
            }
            
            updateMetrics();
        }
        
        function refreshData() {
            addLog('Refreshing system data...', 'info');
            fetchAgents();
            updateMetrics();
        }
        
        async function fetchAgents() {
            try {
                const response = await fetch('/api/agents');
                if (response.ok) {
                    const data = await response.json();
                    agents = data.agents || [];
                    system_metrics.total_messages = data.total_messages || 0;
                    updateAgentList();
                    updatePluginList();
                }
            } catch (error) {
                addLog(`Failed to fetch agents: ${error.message}`, 'error');
            }
        }
        
        async function simulateAgent() {
            addLog('Simulating agent connection...', 'info');
            
            try {
                const agentData = {
                    id: 'DEMO-001',
                    hostname: 'Demo-Workstation',
                    version: '1.0.0',
                    status: 'active',
                    last_seen: new Date().toISOString(),
                    plugins: [
                        {
                            name: 'system_monitor',
                            status: 'active',
                            version: '1.2.0',
                            execution_count: 156,
                            avg_execution_time: 45.2,
                            errors: 0,
                            last_check: new Date().toISOString(),
                            metrics: {
                                cpu_temp: 42.5,
                                fan_speed: 2100,
                                load_average: 1.8
                            }
                        },
                        {
                            name: 'network_scanner',
                            status: 'active',
                            version: '1.1.0',
                            execution_count: 89,
                            avg_execution_time: 120.5,
                            errors: 2,
                            last_check: new Date().toISOString(),
                            metrics: {
                                active_connections: 15,
                                bandwidth_usage: 65.3,
                                packets_sent: 1247,
                                packets_received: 1189
                            }
                        },
                        {
                            name: 'process_manager',
                            status: 'active',
                            version: '2.0.1',
                            execution_count: 234,
                            avg_execution_time: 15.8,
                            errors: 1,
                            last_check: new Date().toISOString(),
                            metrics: {
                                total_processes: 187,
                                active_processes: 42,
                                memory_usage_mb: 2048
                            }
                        },
                        {
                            name: 'file_watcher',
                            status: 'idle',
                            version: '1.0.5',
                            execution_count: 67,
                            avg_execution_time: 8.2,
                            errors: 0,
                            last_check: new Date(Date.now() - 300000).toISOString(),
                            metrics: {
                                files_watched: 1247,
                                files_changed: 23,
                                files_created: 8
                            }
                        }
                    ]
                };
                
                const response = await fetch('/api/register', {
                    method: 'POST',
                    headers: {'Content-Type': 'application/json'},
                    body: JSON.stringify(agentData)
                });
                
                if (response.ok) {
                    const result = await response.json();
                    addLog(`Agent simulated successfully: ${result.message}`, 'success');
                    fetchAgents(); // Refresh agent list
                } else {
                    addLog('Failed to simulate agent', 'error');
                }
            } catch (error) {
                addLog(`Error simulating agent: ${error.message}`, 'error');
            }
        }
        
        // Auto-update data every 2 seconds
        setInterval(() => {
            fetchAgents();
            updateMetrics();
        }, 2000);
        
        // Initial data load
        addLog('Simple demo system ready', 'success');
        fetchAgents();
        updateMetrics();
    </script>
</body>
</html>
"""

@app.route('/')
def index():
    """Serve the simple demo dashboard"""
    return render_template_string(DASHBOARD_TEMPLATE)

@app.route('/api/agents')
def get_agents():
    """Get list of connected agents"""
    return jsonify({
        'agents': list(agents_data.values()),
        'count': len(agents_data),
        'total_messages': system_metrics['total_messages'],
        'timestamp': datetime.now().isoformat()
    })

@app.route('/api/agent/<agent_id>')
def get_agent(agent_id):
    """Get specific agent data"""
    if agent_id in agents_data:
        return jsonify(agents_data[agent_id])
    return jsonify({'error': 'Agent not found'}), 404

@app.route('/api/register', methods=['POST'])
def register_agent():
    """Register a new agent"""
    try:
        data = request.get_json()
        agent_id = data.get('id', f'demo_agent_{int(time.time())}')
        
        agents_data[agent_id] = {
            'id': agent_id,
            'hostname': data.get('hostname', 'Demo Agent'),
            'version': data.get('version', '1.0.0'),
            'status': 'active',
            'last_seen': datetime.now().isoformat(),
            'plugins': data.get('plugins', [])
        }
        
        system_metrics['total_messages'] += 1
        print(f"Agent registered: {agent_id} ({data.get('hostname', 'Demo Agent')})")
        
        return jsonify({
            'status': 'success',
            'agent_id': agent_id,
            'message': 'Agent registered successfully'
        })
        
    except Exception as e:
        return jsonify({'error': str(e)}), 400

@app.route('/api/update', methods=['POST'])
def update_agent():
    """Update agent data"""
    try:
        data = request.get_json()
        agent_id = data.get('id')
        
        if agent_id in agents_data:
            agents_data[agent_id].update({
                'last_seen': datetime.now().isoformat(),
                'status': data.get('status', 'active'),
                'metrics': data.get('metrics', {})
            })
            system_metrics['total_messages'] += 1
            
        return jsonify({
            'status': 'success',
            'message': 'Agent updated successfully'
        })
        
    except Exception as e:
        return jsonify({'error': str(e)}), 400

@app.route('/api/command', methods=['POST'])
def send_command():
    """Send command to agents"""
    try:
        data = request.get_json()
        command = data.get('command')
        target_agent = data.get('agent_id')
        
        system_metrics['commands_sent'] += 1
        
        # Simulate command processing
        if target_agent and target_agent in agents_data:
            agents_data[target_agent]['last_command'] = command
            agents_data[target_agent]['last_command_time'] = datetime.now().isoformat()
        
        # Simulate response
        response_data = {
            'status': 'success',
            'message': f'Command "{command}" sent to agents',
            'target_agent': target_agent,
            'timestamp': datetime.now().isoformat()
        }
        
        # Simulate async response
        def simulate_response():
            time.sleep(random.uniform(0.5, 2.0))
            system_metrics['responses_received'] += 1
            
        threading.Thread(target=simulate_response, daemon=True).start()
        
        return jsonify(response_data)
        
    except Exception as e:
        return jsonify({'error': str(e)}), 400

@app.route('/api/metrics')
def get_metrics():
    """Get system metrics"""
    uptime = time.time() - system_metrics['start_time']
    
    return jsonify({
        'system_metrics': system_metrics,
        'uptime_seconds': uptime,
        'uptime_formatted': f"{int(uptime // 60)}m {int(uptime % 60)}s",
        'agents_count': len(agents_data),
        'timestamp': datetime.now().isoformat()
    })

if __name__ == '__main__':
    print("Starting Mini MSP Agent Simple Demo Server on port 5001...")
    print("Dashboard available at: http://localhost:5001")
    print("API endpoints:")
    print("  GET  /api/agents - List all agents")
    print("  GET  /api/agent/<id> - Get specific agent")
    print("  POST /api/register - Register agent")
    print("  POST /api/update - Update agent data")
    print("  POST /api/command - Send command")
    print("  GET  /api/metrics - System metrics")
    print("\nFeatures:")
    print("  ✅ Real-time agent data")
    print("  ✅ Plugin information")
    print("  ✅ System metrics")
    print("  ✅ Command processing")
    print("  ✅ No NATS required")
    
    app.run(host='0.0.0.0', port=5001, debug=False)
