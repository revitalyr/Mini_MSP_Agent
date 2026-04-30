export interface Agent {
  id: string;
  hostname: string;
  status: 'online' | 'offline' | 'connecting';
  last_seen: string;
  version: string;
}

export interface Metrics {
  agent_id: string;
  timestamp: number;
  cpu: number;
  ram: number;
  disk: number;
  hostname: string;
  uptime: number;
}

export interface Plugin {
  name: string;
  version: string;
  platform: string;
  status: 'loaded' | 'unloaded' | 'error' | 'loading';
  description?: string;
  last_loaded?: string;
}

export interface CommandResponse {
  command_id?: string;
  type: string;
  status: 'ok' | 'error';
  data: any;
  timestamp: number;
}

export interface FileEntry {
  name: string;
  path: string;
  size: number;
  type: 'file' | 'directory';
  modified: string;
  permissions: string;
}

export interface Process {
  pid: number;
  name: string;
  cpu_usage: number;
  memory_usage: number;
  start_time: number;
}

export interface TerminalMessage {
  type: 'input' | 'output' | 'error' | 'system';
  content: string;
  timestamp: number;
}

// WebSocket messages
export interface WSMessage {
  type: string;
  [key: string]: any;
}

export interface WSCommandMessage extends WSMessage {
  type: 'execute_command';
  agent_id: string;
  command: string;
  params?: any;
}

export interface WSMetricsMessage extends WSMessage {
  type: 'subscribe_metrics';
  agent_id: string;
}
