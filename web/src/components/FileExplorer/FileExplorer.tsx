import React, { useEffect, useState } from 'react';
import { useAgentStore } from '../../store/agentStore';
import { wsService } from '../../services/websocket';
import { FileEntry } from '../../types';

const FileExplorer: React.FC = () => {
  const { selectedAgent } = useAgentStore();
  const [currentPath, setCurrentPath] = useState('/');
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (selectedAgent) {
      loadFiles(currentPath);
    }
  }, [selectedAgent, currentPath]);

  const loadFiles = async (path: string) => {
    if (!selectedAgent) return;
    
    setLoading(true);
    try {
      wsService.executeCommand(selectedAgent, 'get_directory_info', { path });
      
      const unsubscribe = wsService.subscribe('command_response', (data: any) => {
        if (data.files) {
          setFiles(data.files);
        }
        unsubscribe();
      });
    } finally {
      setLoading(false);
    }
  };

  const handleFileClick = (file: FileEntry) => {
    if (file.type === 'directory') {
      setCurrentPath(file.path);
    }
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  };

  if (!selectedAgent) {
    return (
      <div className="p-6 text-center text-gray-500">
        Select an agent to explore files
      </div>
    );
  }

  return (
    <div className="p-6">
      <div className="mb-6">
        <h1 className="text-2xl font-bold mb-4">File Explorer</h1>
        <div className="flex items-center space-x-2">
          <button
            onClick={() => setCurrentPath('/')}
            className="px-3 py-1 bg-gray-200 hover:bg-gray-300 rounded"
          >
            Root
          </button>
          {currentPath.split('/').filter(Boolean).map((part, index) => {
            const path = '/' + currentPath.split('/').filter(Boolean).slice(0, index + 1).join('/');
            return (
              <React.Fragment key={path}>
                <span>/</span>
                <button
                  onClick={() => setCurrentPath(path)}
                  className="px-3 py-1 bg-gray-200 hover:bg-gray-300 rounded"
                >
                  {part}
                </button>
              </React.Fragment>
            );
          })}
        </div>
      </div>

      {loading ? (
        <div className="flex justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
        </div>
      ) : (
        <div className="bg-white rounded-lg shadow overflow-hidden">
          <table className="min-w-full">
            <thead className="bg-gray-50">
              <tr>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Name
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Size
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Modified
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Permissions
                </th>
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-gray-200">
              {files.map((file) => (
                <tr
                  key={file.path}
                  className="hover:bg-gray-50 cursor-pointer"
                  onClick={() => handleFileClick(file)}
                >
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="flex items-center">
                      <span className="mr-2">
                        {file.type === 'directory' ? 'Folder' : 'File'}
                      </span>
                      <span className="text-sm font-medium text-gray-900">
                        {file.name}
                      </span>
                    </div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                    {file.type === 'directory' ? '-' : formatFileSize(file.size)}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                    {new Date(file.modified).toLocaleString()}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                    {file.permissions}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};

export default FileExplorer;
