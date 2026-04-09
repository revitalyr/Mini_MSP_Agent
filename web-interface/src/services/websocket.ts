import { WSMessage, Metrics, CommandResponse } from '../types';

class WebSocketService {
  private ws: WebSocket | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;
  private messageHandlers: Map<string, Set<(data: any) => void>> = new Map();

  connect(url: string) {
    this.ws = new WebSocket(url);

    this.ws.onopen = () => {
      console.log('WebSocket connected');
      this.reconnectAttempts = 0;
      this.emit('connected', {});
    };

    this.ws.onmessage = (event) => {
      try {
        const data: WSMessage = JSON.parse(event.data);
        this.emit(data.type, data);
      } catch (error) {
        console.error('Failed to parse WebSocket message:', error);
      }
    };

    this.ws.onclose = () => {
      console.log('WebSocket disconnected');
      this.emit('disconnected', {});
      this.attemptReconnect(url);
    };

    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
      this.emit('error', { error });
    };
  }

  private attemptReconnect(url: string) {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error('Max reconnect attempts reached');
      return;
    }

    this.reconnectAttempts++;
    const delay = this.reconnectDelay * this.reconnectAttempts;
    
    console.log(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);
    
    setTimeout(() => {
      this.connect(url);
    }, delay);
  }

  send(message: WSMessage) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    } else {
      console.error('WebSocket is not connected');
    }
  }

  subscribe(type: string, handler: (data: any) => void) {
    if (!this.messageHandlers.has(type)) {
      this.messageHandlers.set(type, new Set());
    }
    this.messageHandlers.get(type)!.add(handler);

    // Return unsubscribe function
    return () => {
      this.messageHandlers.get(type)?.delete(handler);
    };
  }

  private emit(type: string, data: any) {
    this.messageHandlers.get(type)?.forEach((handler) => handler(data));
  }

  // Convenience methods
  subscribeMetrics(agentId: string) {
    this.send({
      type: 'subscribe_metrics',
      agent_id: agentId,
    });
  }

  executeCommand(agentId: string, command: string, params?: any) {
    this.send({
      type: 'execute_command',
      agent_id: agentId,
      command,
      params,
    });
  }

  listAgents() {
    this.send({ type: 'list_agents' });
  }

  disconnect() {
    this.ws?.close();
    this.ws = null;
  }
}

export const wsService = new WebSocketService();
