import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { Agent, Metrics, Plugin, TerminalMessage } from '../types';

interface AgentState {
  agents: Agent[];
  selectedAgent: string | null;
  metrics: Metrics[];
  plugins: Plugin[];
  terminalMessages: TerminalMessage[];
  
  // Actions
  setAgents: (agents: Agent[]) => void;
  setSelectedAgent: (agentId: string | null) => void;
  addMetric: (metric: Metrics) => void;
  setPlugins: (plugins: Plugin[]) => void;
  addTerminalMessage: (message: TerminalMessage) => void;
  clearTerminal: () => void;
  clearMetrics: () => void;
}

export const useAgentStore = create<AgentState>()(
  persist(
    (set) => ({
      agents: [],
      selectedAgent: null,
      metrics: [],
      plugins: [],
      terminalMessages: [],

      setAgents: (agents) => set({ agents }),
      
      setSelectedAgent: (agentId) => set({ selectedAgent: agentId }),
      
      addMetric: (metric) => set((state) => ({
        metrics: [...state.metrics.slice(-99), metric], // Store last 100 points
      })),
      
      setPlugins: (plugins) => set({ plugins }),
      
      addTerminalMessage: (message) => set((state) => ({
        terminalMessages: [...state.terminalMessages.slice(-999), message],
      })),
      
      clearTerminal: () => set({ terminalMessages: [] }),
      clearMetrics: () => set({ metrics: [] }),
    }),
    {
      name: 'agent-storage',
      partialize: (state) => ({ selectedAgent: state.selectedAgent }),
    }
  )
);
