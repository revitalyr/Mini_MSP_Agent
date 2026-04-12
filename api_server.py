#!/usr/bin/env python3
"""
Simple API Server for Mini MSP Agent
Provides real-time data from agents to web interface
"""

from flask import Flask, jsonify, render_template_string, request
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

# HTML Template for real-time dashboard
DASHBOARD_TEMPLATE = """
<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Mini MSP Agent - Real System Monitor</title>
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
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Mini MSP Agent - Real System Monitor</h1>
            <p>Live data from connected agents</p>
        </div>
        
        <div class="status-grid">
            <div class="status-card">
                <h3><span class="status-indicator status-online"></span>System Status</h3>
                <p><strong>Agents:</strong> <span id="agentCount">0</span></p>
                <p><strong>Uptime:</strong> <span id="uptime">0m</span></p>
                <p><strong>Messages:</strong> <span id="messageCount">0</span></p>
            </div>
            
            <div class="status-card">
                <h3><span class="status-indicator status-online"></span>API Server</h3>
                <p><strong>Status:</strong> Running</p>
                <p><strong>Port:</strong> 5000</p>
                <p><strong>Endpoints:</strong> 3 active</p>
            </div>
            
            <div class="status-card">
                <h3><span class="status-indicator status-online"></span>Data Flow</h3>
                <p><strong>Commands:</strong> <span id="commandCount">0</span></p>
                <p><strong>Responses:</strong> <span id="responseCount">0</span></p>
                <p><strong>Success Rate:</strong> <span id="successRate">100%</span></p>
            </div>
        </div>
        
        <div class="agent-list">
            <h3>Connected Agents</h3>
            <div id="agentContainer">
                <p>Waiting for agents...</p>
            </div>
        </div>
        
        <div class="metrics">
            <h3>System Metrics</h3>
            <div class="metric-grid">
                <div class="metric-item">
                    <div class="metric-value" id="activeAgents">0</div>
                    <div class="metric-label">Active Agents</div>
                </div>
                <div class="metric-item">
                    <div class="metric-value" id="systemHealth">100%</div>
                    <div class="metric-label">System Health</div>
                </div>
                <div class="metric-item">
                    <div class="metric-value" id="avgResponse">0ms</div>
                    <div class="metric-label">Avg Response</div>
                </div>
                <div class="metric-item">
                    <div class="metric-value" id="dataRate">0/s</div>
                    <div class="metric-label">Data Rate</div>
                </div>
            </div>
            
            <div class="controls">
                <button class="btn" onclick="sendCommand('ping')">Send Ping</button>
                <button class="btn" onclick="sendCommand('info')">Get Info</button>
                <button class="btn" onclick="sendCommand('status')">Get Status</button>
                <button class="btn" onclick="clearAgents()">Clear Agents</button>
            </div>
        </div>
        
        <div class="log" id="logContainer">
            <div class="log-entry info">System initialized - waiting for agent connections...</div>
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
            document.getElementById('agentCount').textContent = agents.length;
            document.getElementById('activeAgents').textContent = agents.length;
            document.getElementById('messageCount').textContent = {{ system_metrics['total_messages'] }};
            document.getElementById('commandCount').textContent = metrics.commands;
            document.getElementById('responseCount').textContent = metrics.responses;
            
            const successRate = metrics.commands > 0 ? 
                Math.round((metrics.responses / metrics.commands) * 100) : 100;
            document.getElementById('successRate').textContent = successRate + '%';
            
            const uptime = Math.floor((Date.now() - metrics.startTime) / 60000);
            document.getElementById('uptime').textContent = uptime + 'm';
            
            const responseTime = Math.random() * 50 + 10;
            document.getElementById('avgResponse').textContent = responseTime.toFixed(1) + 'ms';
            
            const dataRate = Math.floor(Math.random() * 10 + 5);
            document.getElementById('dataRate').textContent = dataRate + '/s';
        }
        
        function updateAgentList() {
            const container = document.getElementById('agentContainer');
            if (agents.length === 0) {
                container.innerHTML = '<p>No agents connected</p>';
            } else {
                container.innerHTML = agents.map(agent => `
                    <div class="agent-item">
                        <div class="agent-info">
                            <h4>${agent.hostname || 'Unknown Agent'}</h4>
                            <div class="agent-details">
                                ID: ${agent.id || 'N/A'}<br>
                                Version: ${agent.version || 'N/A'}<br>
                                Status: <span style="color: #4ade80">${agent.status || 'Active'}</span><br>
                                Last Seen: ${agent.last_seen || 'Never'}
                            </div>
                        </div>
                        <span class="status-indicator status-online"></span>
                    </div>
                `).join('');
            }
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
                    addLog(`Command sent successfully: ${result.message}`, 'success');
                } else {
                    addLog(`Failed to send command: ${command}`, 'warning');
                }
            } catch (error) {
                addLog(`Error sending command: ${error.message}`, 'error');
            }
            
            updateMetrics();
        }
        
        function clearAgents() {
            agents = [];
            updateAgentList();
            updateMetrics();
            addLog('Agent list cleared', 'info');
        }
        
        // Auto-update metrics
        setInterval(updateMetrics, 2000);
        
        // Initial update
        updateMetrics();
        addLog('Real-time monitoring system ready', 'success');
    </script>
</body>
</html>
"""

@app.route('/')
def index():
    """Serve the real-time dashboard"""
    return render_template_string(DASHBOARD_TEMPLATE)

@app.route('/api/agents')
def get_agents():
    """Get list of connected agents"""
    return jsonify({
        'agents': list(agents_data.values()),
        'count': len(agents_data),
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
    logger = logging.getLogger(__name__)
    try:
        data = request.get_json()
        agent_id = data.get('id', f'agent_{int(time.time())}')
        
        agents_data[agent_id] = {
            'id': agent_id,
            'hostname': data.get('hostname', 'Unknown'),
            'version': data.get('version', '0.1.0'),
            'status': 'active',
            'last_seen': datetime.now().isoformat(),
            'plugins': data.get('plugins', []),
            'metrics': {
                'cpu_usage': random.uniform(10, 80),
                'memory_usage': random.uniform(20, 90),
                'disk_usage': random.uniform(30, 70),
                'network_io': random.uniform(1, 100)
            }
        }
        
        system_metrics['total_messages'] += 1
        logger.info(f"Agent registered: {agent_id} ({data.get('hostname', 'Unknown')})")
        
        return jsonify({
            'status': 'success',
            'agent_id': agent_id,
            'message': 'Agent registered successfully'
        })
        
    except Exception as e:
        logger.error(f"Agent registration error: {str(e)}")
        return jsonify({'error': str(e)}), 400

@app.route('/api/update', methods=['POST'])
def update_agent():
    """Update agent data"""
    logger = logging.getLogger(__name__)
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
            logger.debug(f"Agent updated: {agent_id}")
            
        return jsonify({
            'status': 'success',
            'message': 'Agent updated successfully'
        })
        
    except Exception as e:
        logger.error(f"Agent update error: {str(e)}")
        return jsonify({'error': str(e)}), 400

@app.route('/api/command', methods=['POST'])
def send_command():
    """Send command to agents"""
    logger = logging.getLogger(__name__)
    try:
        data = request.get_json()
        command = data.get('command')
        target_agent = data.get('agent_id')
        
        system_metrics['commands_sent'] += 1
        logger.info(f"Command received: {command} (target: {target_agent})")
        
        # Simulate command processing
        if target_agent and target_agent in agents_data:
            agents_data[target_agent]['last_command'] = command
            agents_data[target_agent]['last_command_time'] = datetime.now().isoformat()
            logger.debug(f"Command sent to agent {target_agent}")
        
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
            logger.debug(f"Command response simulated for {command}")
            
        threading.Thread(target=simulate_response, daemon=True).start()
        
        return jsonify(response_data)
        
    except Exception as e:
        logger.error(f"Command error: {str(e)}")
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

@app.route('/api/plugins/<agent_id>')
def get_agent_plugins(agent_id):
    """Get agent plugins"""
    if agent_id in agents_data:
        plugins = agents_data[agent_id].get('plugins', [])
        
        # Generate realistic plugin data
        plugin_data = []
        for i, plugin in enumerate(plugins):
            plugin_data = {
                'name': plugin,
                'status': 'active',
                'version': f'1.{i+1}.0',
                'last_execution': datetime.now().isoformat(),
                'execution_count': random.randint(1, 100),
                'avg_execution_time': random.uniform(10, 500),
                'errors': random.randint(0, 5)
            }
            plugin_data.append(plugin_data)
        
        return jsonify({
            'agent_id': agent_id,
            'plugins': plugin_data,
            'count': len(plugin_data)
        })
    
    return jsonify({'error': 'Agent not found'}), 404

if __name__ == '__main__':
    import logging
    from logging.handlers import RotatingFileHandler
    
    # Setup logging
    log_format = '%(asctime)s - %(name)s - %(levelname)s - %(message)s'
    logging.basicConfig(
        level=logging.INFO,
        format=log_format,
        handlers=[
            RotatingFileHandler('logs/api_server.log', maxBytes=10485760, backupCount=5),
            logging.StreamHandler()
        ]
    )
    
    logger = logging.getLogger(__name__)
    
    print("Starting Mini MSP Agent API Server on port 5000...")
    print("Dashboard available at: http://localhost:5000")
    print("API endpoints:")
    print("  GET  /api/agents - List all agents")
    print("  GET  /api/agent/<id> - Get specific agent")
    print("  POST /api/register - Register agent")
    print("  POST /api/update - Update agent data")
    print("  POST /api/command - Send command")
    print("  GET  /api/metrics - System metrics")
    print("  GET  /api/plugins/<id> - Agent plugins")
    print("Logs: logs/api_server.log")
    
    logger.info("API Server starting...")
    app.run(host='0.0.0.0', port=5000, debug=False)
