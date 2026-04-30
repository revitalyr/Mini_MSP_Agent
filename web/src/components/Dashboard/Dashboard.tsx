import React, { useEffect, useState } from 'react';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';
import { useAgentStore } from '../../store/agentStore';
import { wsService } from '../../services/websocket';
import { Agent, Metrics } from '../../types';
import clsx from 'clsx';
import { format } from 'date-fns';

const Dashboard: React.FC = () => {
  const { agents, selectedAgent, metrics, setSelectedAgent } = useAgentStore();
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Subscribe to metrics for selected agent
    if (selectedAgent) {
      wsService.subscribeMetrics(selectedAgent);
    }

    return () => {
      wsService.subscribe('metrics', () => {}); // Clean up
    };
  }, [selectedAgent]);

  useEffect(() => {
    // Get agents list
    wsService.listAgents();
    setLoading(false);
  }, []);

  const selectedAgentData = agents.find(a => a.id === selectedAgent);

  const formatMetrics = (data: Metrics[]) => {
    return data.map(m => ({
      time: format(new Date(m.timestamp * 1000), 'HH:mm:ss'),
      cpu: m.cpu,
      ram: m.ram,
      disk: m.disk,
    }));
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <h1 className="text-2xl font-bold text-gray-900">Dashboard</h1>
        <div className="flex items-center space-x-4">
          <select
            value={selectedAgent || ''}
            onChange={(e) => setSelectedAgent(e.target.value || null)}
            className="border border-gray-300 rounded-lg px-4 py-2 focus:ring-2 focus:ring-blue-500"
          >
            <option value="">Select Agent</option>
            {agents.map((agent) => (
              <option key={agent.id} value={agent.id}>
                {agent.hostname} ({agent.id.slice(0, 8)})
              </option>
            ))}
          </select>
        </div>
      </div>

      {selectedAgentData ? (
        <>
          {/* Agent Info Cards */}
          <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
            <InfoCard
              title="CPU Usage"
              value={`${metrics[metrics.length - 1]?.cpu.toFixed(1) || 0}%`}
              icon="CPU"
              color="blue"
            />
            <InfoCard
              title="RAM Usage"
              value={`${metrics[metrics.length - 1]?.ram.toFixed(1) || 0}%`}
              icon="RAM"
              color="green"
            />
            <InfoCard
              title="Disk Usage"
              value={`${metrics[metrics.length - 1]?.disk.toFixed(1) || 0}%`}
              icon="DISK"
              color="yellow"
            />
            <InfoCard
              title="Uptime"
              value={formatUptime(metrics[metrics.length - 1]?.uptime || 0)}
              icon="UPTIME"
              color="purple"
            />
          </div>

          {/* Metrics Chart */}
          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-lg font-semibold mb-4">Real-time Metrics</h2>
            <ResponsiveContainer width="100%" height={300}>
              <LineChart data={formatMetrics(metrics)}>
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis dataKey="time" />
                <YAxis domain={[0, 100]} />
                <Tooltip />
                <Legend />
                <Line type="monotone" dataKey="cpu" stroke="#3b82f6" name="CPU %" />
                <Line type="monotone" dataKey="ram" stroke="#10b981" name="RAM %" />
                <Line type="monotone" dataKey="disk" stroke="#f59e0b" name="Disk %" />
              </LineChart>
            </ResponsiveContainer>
          </div>

          {/* Agent Details */}
          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-lg font-semibold mb-4">Agent Details</h2>
            <div className="grid grid-cols-2 gap-4">
              <DetailItem label="Agent ID" value={selectedAgentData.id} />
              <DetailItem label="Hostname" value={selectedAgentData.hostname} />
              <DetailItem label="Status" value={selectedAgentData.status} />
              <DetailItem label="Version" value={selectedAgentData.version} />
              <DetailItem label="Last Seen" value={format(new Date(selectedAgentData.last_seen), 'PPpp')} />
            </div>
          </div>
        </>
      ) : (
        <div className="bg-white rounded-lg shadow p-12 text-center">
          <div className="text-6xl mb-4">Computer</div>
          <h2 className="text-xl font-semibold text-gray-700 mb-2">No Agent Selected</h2>
          <p className="text-gray-500">Select an agent from the dropdown to view metrics</p>
        </div>
      )}
    </div>
  );
};

const InfoCard: React.FC<{ title: string; value: string; icon: string; color: string }> = ({
  title,
  value,
  icon,
  color,
}) => {
  const colorClasses = {
    blue: 'bg-blue-50 border-blue-200',
    green: 'bg-green-50 border-green-200',
    yellow: 'bg-yellow-50 border-yellow-200',
    purple: 'bg-purple-50 border-purple-200',
  };

  return (
    <div className={clsx('border rounded-lg p-4', colorClasses[color as keyof typeof colorClasses])}>
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm text-gray-600">{title}</p>
          <p className="text-2xl font-bold mt-1">{value}</p>
        </div>
        <div className="text-3xl">{icon}</div>
      </div>
    </div>
  );
};

const DetailItem: React.FC<{ label: string; value: string }> = ({ label, value }) => (
  <div className="flex justify-between py-2 border-b">
    <span className="text-gray-600">{label}</span>
    <span className="font-medium">{value}</span>
  </div>
);

const formatUptime = (seconds: number): string => {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  
  if (days > 0) return `${days}d ${hours}h ${mins}m`;
  if (hours > 0) return `${hours}h ${mins}m`;
  return `${mins}m`;
};

export default Dashboard;
