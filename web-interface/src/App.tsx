import React, { useEffect } from 'react';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { wsService } from './services/websocket';
import Dashboard from './components/Dashboard/Dashboard';
import Terminal from './components/Terminal/Terminal';
import PluginManager from './components/Plugins/PluginManager';
import FileExplorer from './components/FileExplorer/FileExplorer';
import SystemInfo from './components/SystemInfo/SystemInfo';
import './App.css';

const queryClient = new QueryClient();

function App() {
  useEffect(() => {
    // Connect to WebSocket on load
    wsService.connect('ws://localhost:3000/ws');

    // Handle connections
    wsService.subscribe('connected', () => {
      console.log('Connected to server');
    });

    wsService.subscribe('disconnected', () => {
      console.log('Disconnected from server');
    });

    return () => {
      wsService.disconnect();
    };
  }, []);

  return (
    <QueryClientProvider client={queryClient}>
      <Router>
        <div className="app">
          <Sidebar />
          <main className="main-content">
            <Routes>
              <Route path="/" element={<Navigate to="/dashboard" replace />} />
              <Route path="/dashboard" element={<Dashboard />} />
              <Route path="/terminal" element={<Terminal />} />
              <Route path="/plugins" element={<PluginManager />} />
              <Route path="/files" element={<FileExplorer />} />
              <Route path="/system" element={<SystemInfo />} />
            </Routes>
          </main>
        </div>
      </Router>
    </QueryClientProvider>
  );
}

const Sidebar: React.FC = () => {
  const routes = [
    { path: '/dashboard', label: 'Dashboard', icon: 'Dashboard' },
    { path: '/terminal', label: 'Terminal', icon: 'Terminal' },
    { path: '/plugins', label: 'Plugins', icon: 'Plugins' },
    { path: '/files', label: 'File Explorer', icon: 'Files' },
    { path: '/system', label: 'System Info', icon: 'System' },
  ];

  return (
    <aside className="sidebar">
      <div className="sidebar-header">
        <h1>MSP Agent</h1>
      </div>
      <nav className="sidebar-nav">
        {routes.map((route) => (
          <a
            key={route.path}
            href={route.path}
            className="sidebar-link"
          >
            <span className="icon">{route.icon}</span>
            <span className="label">{route.label}</span>
          </a>
        ))}
      </nav>
      <div className="sidebar-footer">
        <p className="text-xs text-gray-500">v0.1.0</p>
      </div>
    </aside>
  );
};

export default App;
