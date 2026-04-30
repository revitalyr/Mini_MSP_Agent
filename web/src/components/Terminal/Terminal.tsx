import React, { useState, useEffect, useRef } from 'react';
import { useAgentStore } from '../../store/agentStore';
import { wsService } from '../../services/websocket';
import { TerminalMessage } from '../../types';
import clsx from 'clsx';

const Terminal: React.FC = () => {
  const { selectedAgent, terminalMessages, addTerminalMessage, clearTerminal } = useAgentStore();
  const [command, setCommand] = useState('');
  const [isExecuting, setIsExecuting] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Subscribe to command responses
    const unsubscribe = wsService.subscribe('command_response', (data: any) => {
      addTerminalMessage({
        type: 'output',
        content: JSON.stringify(data, null, 2),
        timestamp: Date.now(),
      });
      setIsExecuting(false);
    });

    return () => unsubscribe();
  }, [addTerminalMessage]);

  useEffect(() => {
    // Auto-scroll to latest message
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [terminalMessages]);

  const executeCommand = async () => {
    if (!selectedAgent || !command.trim() || isExecuting) return;

    setIsExecuting(true);
    
    // Add command to history
    addTerminalMessage({
      type: 'input',
      content: command,
      timestamp: Date.now(),
    });

    try {
      wsService.executeCommand(selectedAgent, 'exec', { cmd: command });
    } catch (error) {
      addTerminalMessage({
        type: 'error',
        content: `Error: ${error}`,
        timestamp: Date.now(),
      });
      setIsExecuting(false);
    }

    setCommand('');
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      executeCommand();
    }
  };

  const quickCommands = [
    'ps aux | head -20',
    'df -h',
    'free -m',
    'uptime',
    'top -bn1 | head -20',
    'cat /etc/os-release',
  ];

  return (
    <div className="h-full flex flex-col bg-gray-900 text-green-400 font-mono">
      {/* Header */}
      <div className="flex justify-between items-center p-4 border-b border-gray-700">
        <div className="flex items-center space-x-2">
          <span className="text-xl">Terminal</span>
          {selectedAgent && (
            <span className="text-sm text-gray-400">
              {selectedAgent.slice(0, 8)}
            </span>
          )}
        </div>
        <button
          onClick={clearTerminal}
          className="px-3 py-1 text-sm bg-gray-700 hover:bg-gray-600 rounded"
        >
          Clear
        </button>
      </div>

      {/* Quick Commands */}
      <div className="p-4 border-b border-gray-700">
        <div className="flex flex-wrap gap-2">
          {quickCommands.map((cmd) => (
            <button
              key={cmd}
              onClick={() => setCommand(cmd)}
              className="px-3 py-1 text-xs bg-gray-800 hover:bg-gray-700 rounded border border-gray-600"
            >
              {cmd.split(' ')[0]}
            </button>
          ))}
        </div>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4 space-y-2">
        {terminalMessages.map((msg, index) => (
          <div
            key={index}
            className={clsx('p-2 rounded', {
              'bg-gray-800': msg.type === 'input',
              'bg-green-900/30': msg.type === 'output',
              'bg-red-900/30': msg.type === 'error',
              'bg-blue-900/30': msg.type === 'system',
            })}
          >
            <div className="text-xs text-gray-500 mb-1">
              {new Date(msg.timestamp).toLocaleTimeString()}
            </div>
            <pre className="whitespace-pre-wrap text-sm">
              {msg.type === 'input' && <span className="text-yellow-400">$ </span>}
              {msg.content}
            </pre>
          </div>
        ))}
        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div className="p-4 border-t border-gray-700">
        <div className="flex items-center space-x-2">
          <span className="text-yellow-400">$</span>
          <textarea
            value={command}
            onChange={(e) => setCommand(e.target.value)}
            onKeyDown={handleKeyDown}
            className="flex-1 bg-gray-800 text-green-400 font-mono border border-gray-600 rounded px-3 py-2 focus:outline-none focus:border-green-400 resize-none"
            rows={2}
            placeholder="Enter command..."
          />
          <button
            onClick={executeCommand}
            disabled={!selectedAgent || !command.trim() || isExecuting}
            className={clsx(
              'px-4 py-2 rounded font-semibold',
              selectedAgent && command.trim() && !isExecuting
                ? 'bg-green-600 hover:bg-green-700'
                : 'bg-gray-600 cursor-not-allowed'
            )}
          >
            {isExecuting ? '...' : 'Run'}
          </button>
        </div>
      </div>
    </div>
  );
};

export default Terminal;
