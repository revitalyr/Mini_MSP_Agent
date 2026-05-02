# Mini MSP Agent - Web Interface

## Overview

A modern web interface for managing and monitoring Mini MSP Agents with real-time capabilities, plugin management, and system monitoring.

## Architecture

```
web-interface/              # React + TypeScript Frontend
  src/
    components/             # UI Components
      Dashboard/            # Real-time metrics dashboard
      Terminal/             # Command execution interface
      Plugins/              # Plugin management
      FileExplorer/         # File system browser
      SystemInfo/           # System information display
    services/               # API and WebSocket services
    store/                  # State management (Zustand)
    types/                  # TypeScript definitions

backend/                    # Rust + Axum Backend
  src/
    main.rs                 # Application entry point
    websocket.rs            # WebSocket handling
    nats_client.rs          # NATS messaging client
    api.rs                  # REST API endpoints
```

## Features

### Real-time Dashboard
- Live CPU, RAM, and Disk usage monitoring
- Interactive charts with historical data
- Multi-agent support with agent switching
- System information display

### Terminal Interface
- Command execution with real-time output
- Command history and quick commands
- Syntax highlighting and auto-completion
- Multi-agent terminal sessions

### Plugin Management
- Load/unload/reload plugins dynamically
- Plugin status monitoring
- Version and compatibility information
- Hot-reload support

### File Explorer
- Browse remote file systems
- File upload/download capabilities
- Directory navigation
- File permissions and metadata

### System Information
- Hardware specifications
- Operating system details
- Network configuration
- Performance metrics

## Technology Stack

### Frontend
- **React 18** with TypeScript
- **Vite** for fast development and building
- **Tailwind CSS** for styling
- **Zustand** for state management
- **Recharts** for data visualization
- **Monaco Editor** for terminal interface

### Backend
- **Rust** with Axum web framework
- **WebSocket** for real-time communication
- **NATS** for message brokering
- **Serde** for JSON serialization
- **Tokio** for async runtime

### Infrastructure
- **Docker** for containerization
- **Docker Compose** for orchestration
- **Nginx** for serving static files
- **NATS JetStream** for persistence

## Quick Start

### Prerequisites
- Docker and Docker Compose
- Node.js 18+ (for development)
- Rust 1.75+ (for development)

### Using Docker Compose

1. Clone the repository:
```bash
git clone <repository-url>
cd mini-msp-agent
```

2. Start all services:
```bash
docker-compose up -d
```

3. Access the web interface:
- **Web Interface**: http://localhost:80
- **API**: http://localhost:3000
- **NATS Monitoring**: http://localhost:8222

### Development Setup

1. Install frontend dependencies:
```bash
cd web-interface
npm install
```

2. Start frontend development server:
```bash
npm run dev
```

3. Start backend:
```bash
cd backend
cargo run
```

4. Start NATS:
```bash
docker run -d --name nats -p 4222:4222 -p 8222:8222 nats:2.10-alpine --jetstream
```

## Configuration

### Environment Variables

#### Backend
- `RUST_LOG`: Logging level (debug, info, warn, error)
- `NATS_URL`: NATS server URL (default: nats://localhost:4222)

#### Frontend
- `VITE_API_URL`: Backend API URL (default: http://localhost:3000)
- `VITE_WS_URL`: WebSocket URL (default: ws://localhost:3000/ws)

### NATS Configuration

The system uses NATS for message brokering with the following subjects:

- `commands.{agent_id}`: Send commands to agents
- `events.{agent_id}`: Receive events from agents
- `heartbeat.>`: Agent heartbeat messages

## API Documentation

### WebSocket Messages

#### Subscribe to Metrics
```json
{
  "type": "subscribe_metrics",
  "agent_id": "agent-001"
}
```

#### Execute Command
```json
{
  "type": "execute_command",
  "agent_id": "agent-001",
  "command": "exec",
  "params": {
    "cmd": "ls -la"
  }
}
```

#### List Agents
```json
{
  "type": "list_agents"
}
```

### REST Endpoints

#### GET /api/agents
List all connected agents

#### GET /api/agents/{agent_id}/metrics
Get current metrics for a specific agent

#### GET /api/agents/{agent_id}/plugins
List plugins for a specific agent

#### POST /api/agents/{agent_id}/command
Execute a command on an agent

#### GET /api/agents/{agent_id}/files
List files in a directory

## Plugin Development

### Creating a Plugin

1. Create a new plugin in the `plugins/src/` directory
2. Implement the required C ABI interface
3. Add the plugin to the appropriate CMakeLists.txt
4. Build and test with the web interface

### Plugin Interface
```cpp
extern "C" {
    typedef struct {
        const char* name;
        const char* version;
        const char* description;
    } PluginInfo;
    
    typedef struct {
        PluginInfo* (*get_plugin_info)();
        bool (*init)();
        void (*cleanup)();
        void* (*get_system_metrics)();
        void* (*handle_command)(const char* command);
        void* (*get_file_info)(const char* path);
    } PluginInterface;
    
    __declspec(dllexport) void* get_plugin_interface();
}
```

## Deployment

### Production Deployment

1. Build the application:
```bash
./scripts/build-web.sh
```

2. Deploy with Docker Compose:
```bash
docker-compose -f docker-compose.prod.yml up -d
```

### Scaling

- **Web Frontend**: Scale horizontally behind a load balancer
- **Web Backend**: Scale horizontally with shared NATS
- **NATS**: Use clustering for high availability
- **Agents**: Deploy across multiple hosts

## Monitoring

### Health Checks
- Frontend: `/health` endpoint
- Backend: `/health` endpoint
- NATS: HTTP monitoring on port 8222

### Metrics
- Application metrics via structured logging
- System metrics via agent plugins
- Network metrics via NATS monitoring

## Troubleshooting

### Common Issues

1. **WebSocket Connection Failed**
   - Check if backend is running on port 3000
   - Verify NATS connection
   - Check browser console for errors

2. **Agent Not Connected**
   - Verify agent configuration
   - Check NATS connection
   - Review agent logs

3. **Plugin Loading Failed**
   - Check plugin compilation
   - Verify plugin interface implementation
   - Review agent logs for specific errors

### Logs

View logs for each service:
```bash
docker-compose logs -f web-backend
docker-compose logs -f web-frontend
docker-compose logs -f agent
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.
