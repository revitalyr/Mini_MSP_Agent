import React, { useEffect, useState } from 'react';
import { useAgentStore } from '../../store/agentStore';
import { wsService } from '../../services/websocket';
import { Plugin } from '../../types';
import clsx from 'clsx';

const PluginManager: React.FC = () => {
  const { selectedAgent, plugins, setPlugins } = useAgentStore();
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (selectedAgent) {
      loadPlugins();
    }
  }, [selectedAgent]);

  const loadPlugins = async () => {
    if (!selectedAgent) return;
    
    setLoading(true);
    try {
      // Request plugins list via WebSocket
      wsService.executeCommand(selectedAgent, 'get_plugin_registry', {});
      
      // Subscribe to response
      const unsubscribe = wsService.subscribe('command_response', (data: any) => {
        if (data.plugins) {
          setPlugins(data.plugins);
        }
        unsubscribe();
      });
    } finally {
      setLoading(false);
    }
  };

  const reloadPlugin = async (pluginName: string) => {
    if (!selectedAgent) return;
    
    wsService.executeCommand(selectedAgent, 'reload_plugin', { name: pluginName });
  };

  const unloadPlugin = async (pluginName: string) => {
    if (!selectedAgent) return;
    
    wsService.executeCommand(selectedAgent, 'unload_plugin', { name: pluginName });
  };

  const getStatusColor = (status: Plugin['status']) => {
    switch (status) {
      case 'loaded':
      case 'active':
        return 'bg-green-100 text-green-800';
      case 'error':
        return 'bg-red-100 text-red-800';
      case 'loading':
      case 'unloading':
        return 'bg-yellow-100 text-yellow-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  if (!selectedAgent) {
    return (
      <div className="p-6 text-center text-gray-500">
        Select an agent to view plugins
      </div>
    );
  }

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">Plugin Manager</h1>
        <button
          onClick={loadPlugins}
          disabled={loading}
          className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
        >
          {loading ? '...' : 'Refresh'}
        </button>
      </div>

      <div className="grid gap-4">
        {plugins.map((plugin) => (
          <div
            key={plugin.name}
            className="bg-white rounded-lg shadow p-4 border border-gray-200"
          >
            <div className="flex justify-between items-start">
              <div className="flex-1">
                <div className="flex items-center space-x-3">
                  <h3 className="text-lg font-semibold">{plugin.name}</h3>
                  <span className={clsx('px-2 py-1 rounded text-xs', getStatusColor(plugin.status))}>
                    {plugin.status}
                  </span>
                </div>
                <p className="text-sm text-gray-600 mt-1">
                  Version: {plugin.version} Platform: {plugin.platform}
                </p>
                {plugin.description && (
                  <p className="text-sm text-gray-500 mt-2">{plugin.description}</p>
                )}
              </div>
              
              <div className="flex space-x-2">
                {plugin.status === 'loaded' && (
                  <>
                    <button
                      onClick={() => reloadPlugin(plugin.name)}
                      className="px-3 py-1 text-sm bg-yellow-100 text-yellow-800 rounded hover:bg-yellow-200"
                    >
                      Reload
                    </button>
                    <button
                      onClick={() => unloadPlugin(plugin.name)}
                      className="px-3 py-1 text-sm bg-red-100 text-red-800 rounded hover:bg-red-200"
                    >
                      Unload
                    </button>
                  </>
                )}
                {plugin.status === 'unloaded' && (
                  <button
                    onClick={() => reloadPlugin(plugin.name)}
                    className="px-3 py-1 text-sm bg-green-100 text-green-800 rounded hover:bg-green-200"
                  >
                    Load
                  </button>
                )}
              </div>
            </div>
          </div>
        ))}

        {plugins.length === 0 && !loading && (
          <div className="text-center py-12 text-gray-500">
            <div className="text-4xl mb-4">Plugin</div>
            <p>No plugins loaded</p>
          </div>
        )}
      </div>
    </div>
  );
};

export default PluginManager;
