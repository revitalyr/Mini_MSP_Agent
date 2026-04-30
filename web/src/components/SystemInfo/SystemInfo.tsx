import React, { useEffect, useState } from 'react';
import { useAgentStore } from '../../store/agentStore';
import { wsService } from '../../services/websocket';

interface SystemInfo {
  hostname: string;
  platform: string;
  cpu_cores: number;
  total_memory: number;
  available_memory: number;
  uptime: number;
  version: string;
}

const SystemInfoComponent: React.FC = () => {
  const { selectedAgent } = useAgentStore();
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (selectedAgent) {
      loadSystemInfo();
    }
  }, [selectedAgent]);

  const loadSystemInfo = async () => {
    if (!selectedAgent) return;
    
    setLoading(true);
    try {
      wsService.executeCommand(selectedAgent, 'get_system_info', {});
      
      const unsubscribe = wsService.subscribe('command_response', (data: any) => {
        if (data.system_info) {
          setSystemInfo(data.system_info);
        }
        unsubscribe();
      });
    } finally {
      setLoading(false);
    }
  };

  const formatMemory = (bytes: number): string => {
    return (bytes / 1024 / 1024 / 1024).toFixed(2) + ' GB';
  };

  const formatUptime = (seconds: number): string => {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${days}d ${hours}h ${minutes}m`;
  };

  if (!selectedAgent) {
    return (
      <div className="p-6 text-center text-gray-500">
        Select an agent to view system information
      </div>
    );
  }

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">System Information</h1>
        <button
          onClick={loadSystemInfo}
          disabled={loading}
          className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
        >
          {loading ? '...' : 'Refresh'}
        </button>
      </div>

      {loading ? (
        <div className="flex justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
        </div>
      ) : systemInfo ? (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-lg font-semibold mb-4">Hardware Information</h2>
            <div className="space-y-3">
              <div className="flex justify-between">
                <span className="text-gray-600">CPU Cores:</span>
                <span className="font-medium">{systemInfo.cpu_cores}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">Total Memory:</span>
                <span className="font-medium">{formatMemory(systemInfo.total_memory)}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">Available Memory:</span>
                <span className="font-medium">{formatMemory(systemInfo.available_memory)}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">Memory Usage:</span>
                <span className="font-medium">
                  {((systemInfo.total_memory - systemInfo.available_memory) / systemInfo.total_memory * 100).toFixed(1)}%
                </span>
              </div>
            </div>
          </div>

          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-lg font-semibold mb-4">System Information</h2>
            <div className="space-y-3">
              <div className="flex justify-between">
                <span className="text-gray-600">Hostname:</span>
                <span className="font-medium">{systemInfo.hostname}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">Platform:</span>
                <span className="font-medium">{systemInfo.platform}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">Agent Version:</span>
                <span className="font-medium">{systemInfo.version}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">Uptime:</span>
                <span className="font-medium">{formatUptime(systemInfo.uptime)}</span>
              </div>
            </div>
          </div>
        </div>
      ) : (
        <div className="bg-white rounded-lg shadow p-12 text-center">
          <div className="text-6xl mb-4">System</div>
          <h2 className="text-xl font-semibold text-gray-700 mb-2">No System Information</h2>
          <p className="text-gray-500">Unable to retrieve system information from the agent</p>
        </div>
      )}
    </div>
  );
};

export default SystemInfoComponent;
