import { useState, useEffect, useCallback, useRef, useMemo } from 'react'
import VirtualRenderer, { AstNode } from './components/VirtualRenderer'
import { FileText, Folder, Search, PanelLeftClose, PanelLeftOpen } from 'lucide-react'

interface FileMetadata {
  modified: number;
  created: number;
  linkCount: number;
}

interface FileEntry {
  path: string;
  modified: number;
}

function App() {
  const [ast, setAst] = useState<AstNode | null>(null)
  const [files, setFiles] = useState<FileEntry[]>([])
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedFile, setSelectedFile] = useState<string | null>(null)
  const [isConnected, setIsConnected] = useState(false)
  const [sidebarOpen, setSidebarOpen] = useState(true)
  const [highlightedIndex, setHighlightedIndex] = useState<number>(-1)
  // Use a ref for WS so handleSelectFile always has the live socket, no stale closure
  const wsRef = useRef<WebSocket | null>(null)

  useEffect(() => {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.hostname}:3000/ws`;

    const connect = () => {
      const socket = new WebSocket(wsUrl);
      wsRef.current = socket;

      socket.onopen = () => {
        console.log('[patto] WebSocket connected to', wsUrl);
        setIsConnected(true);
      };

      socket.onmessage = (event) => {
        try {
          // Backend uses #[serde(tag = "type", content = "data")]
          // so messages arrive as { type: "...", data: { ... } }
          const msg = JSON.parse(event.data);
          const data = msg.data ?? {};

          console.log('[patto] msg:', msg.type);

          if (msg.type === 'FileList') {
            const filePaths: string[] = data.files || [];
            const metadataMap: Record<string, FileMetadata> = data.metadata || {};

            const fileEntries: FileEntry[] = filePaths.map(path => {
              const meta = metadataMap[path];
              return {
                path,
                modified: meta ? meta.modified : 0
              };
            });

            // Sort by newest modified first
            fileEntries.sort((a, b) => b.modified - a.modified);
            setFiles(fileEntries);

          } else if (msg.type === 'FileChanged') {
            console.log('[patto] FileChanged ast:', JSON.stringify(data.ast).substring(0, 200));
            setAst(data.ast ?? null);
            if (data.path && data.metadata) {
              setFiles(prev => {
                const updated = prev.map(f =>
                  f.path === data.path ? { ...f, modified: data.metadata.modified } : f
                );
                return [...updated].sort((a, b) => b.modified - a.modified);
              });
            }
          } else if (msg.type === 'FileAdded') {
            if (data.path && data.metadata) {
              setFiles(prev =>
                [...prev, { path: data.path, modified: data.metadata.modified }]
                  .sort((a, b) => b.modified - a.modified)
              );
            }
          } else if (msg.type === 'FileRemoved') {
            if (data.path) {
              setFiles(prev => prev.filter(f => f.path !== data.path));
            }
          }
        } catch (e) {
          console.error('[patto] Failed to parse websocket message', e, event.data);
        }
      };

      socket.onclose = () => {
        console.log('[patto] WebSocket closed, reconnecting in 2s...');
        setIsConnected(false);
        wsRef.current = null;
        setTimeout(connect, 2000);
      };

      socket.onerror = (err) => {
        console.error('[patto] WebSocket error:', err);
      };
    };

    connect();

    return () => {
      wsRef.current?.close();
    };
  }, []);

  const handleSelectFile = useCallback((path: string) => {
    setSelectedFile(path);
    setAst(null); // Clear while loading
    const socket = wsRef.current;
    if (socket && socket.readyState === WebSocket.OPEN) {
      // Backend WsClientMessage: #[serde(tag = "type", content = "data")]
      socket.send(JSON.stringify({ type: 'SelectFile', data: { path } }));
      console.log('[patto] SelectFile sent:', path);
    } else {
      console.warn('[patto] WebSocket not open, state:', socket?.readyState);
    }
  }, []);

  const handleWikiLinkClick = useCallback((link: string, _anchor?: string) => {
    const targetFile = files.find(f => f.path.replace(/\.pn$/, '') === link || f.path === link || f.path === `${link}.pn`);
    if (targetFile) {
      handleSelectFile(targetFile.path);
    }
  }, [files, handleSelectFile]);

  // Fuzzy filter files
  const filteredFiles = useMemo(() => {
    if (!searchQuery.trim()) return files;
    const lowerQuery = searchQuery.toLowerCase();
    return files.filter(f => f.path.toLowerCase().includes(lowerQuery));
  }, [files, searchQuery]);

  // Reset highlight when query changes
  const handleSearchChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setSearchQuery(e.target.value);
    setHighlightedIndex(-1);
  }, []);

  const handleSearchKeyDown = useCallback((e: React.KeyboardEvent<HTMLInputElement>) => {
    const len = filteredFiles.length;
    if (len === 0) return;
    if (e.key === 'Tab' || e.key === 'ArrowDown') {
      e.preventDefault();
      setHighlightedIndex(i => (i + 1) % len);
    } else if ((e.key === 'Tab' && e.shiftKey) || e.key === 'ArrowUp') {
      e.preventDefault();
      setHighlightedIndex(i => (i - 1 + len) % len);
    } else if (e.key === 'Enter') {
      const idx = highlightedIndex >= 0 ? highlightedIndex : 0;
      handleSelectFile(filteredFiles[idx].path);
    }
  }, [filteredFiles, highlightedIndex, handleSelectFile]);

  return (
    <div className="flex h-screen w-screen bg-white overflow-hidden text-slate-800">
      {/* Sidebar */}
      <div
        className="border-r border-slate-200 bg-slate-50 flex flex-col overflow-hidden transition-all duration-200"
        style={{ width: sidebarOpen ? '17rem' : '0', minWidth: sidebarOpen ? '17rem' : '0' }}
      >
        <div className="px-4 py-3 border-b border-slate-200 flex justify-between items-center min-w-[17rem]">
          <h2 className="font-semibold flex items-center gap-2 text-sm">
            <Folder size={16} className="text-slate-500" />
            Workspace
          </h2>
          <div className="flex items-center gap-2">
            <div
              title={isConnected ? 'Connected' : 'Reconnecting...'}
              className={`w-2 h-2 rounded-full ${isConnected ? 'bg-green-400' : 'bg-amber-400 animate-pulse'}`}
            />
            <button onClick={() => setSidebarOpen(false)} title="Close sidebar" className="text-slate-400 hover:text-slate-600">
              <PanelLeftClose size={16} />
            </button>
          </div>
        </div>

        {/* Search Bar */}
        <div className="p-2 border-b border-slate-200 min-w-[17rem]">
          <div className="relative">
            <Search size={14} className="absolute left-2.5 top-2.5 text-slate-400" />
            <input
              type="text"
              placeholder="Fuzzy find files..."
              value={searchQuery}
              onChange={handleSearchChange}
              onKeyDown={handleSearchKeyDown}
              className="w-full pl-8 pr-3 py-1.5 text-sm bg-white border border-slate-200 rounded-md focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
            />
          </div>
        </div>

        <div className="flex-1 overflow-y-auto p-1 text-sm min-w-[17rem]">
          {filteredFiles.length === 0 ? (
            <div className="p-4 text-slate-400 italic text-center text-xs">
              {isConnected ? 'No files found' : 'Connecting...'}
            </div>
          ) : (
            filteredFiles.map((file, idx) => {
              const isHighlighted = idx === highlightedIndex;
              const isSelected = selectedFile === file.path;
              return (
                <div
                  key={file.path}
                  ref={el => { if (isHighlighted && el) el.scrollIntoView({ block: 'nearest' }); }}
                  onClick={() => handleSelectFile(file.path)}
                  className={`flex items-center gap-2 px-3 py-1.5 cursor-pointer rounded-md transition-colors ${isSelected
                      ? 'bg-blue-100 text-blue-700 font-medium'
                      : isHighlighted
                        ? 'bg-slate-200 text-slate-800'
                        : 'hover:bg-slate-200 text-slate-600'
                    }`}
                >
                  <FileText size={14} className={isSelected ? 'text-blue-500' : 'text-slate-400 min-w-4 max-w-4'} />
                  <span className="truncate" title={file.path}>{file.path.split('/').pop()}</span>
                </div>
              );
            })
          )}
        </div>
      </div>

      {/* Main Content Area */}
      <div className="flex-1 overflow-hidden h-full relative">
        {/* Toggle button — always visible */}
        {!sidebarOpen && (
          <button
            onClick={() => setSidebarOpen(true)}
            title="Open sidebar"
            className="absolute top-2 left-2 z-10 p-1.5 rounded-md bg-white border border-slate-200 text-slate-400 hover:text-slate-600 hover:bg-slate-50 shadow-sm"
          >
            <PanelLeftOpen size={16} />
          </button>
        )}
        {!ast ? (
          <div className="flex items-center justify-center h-full flex-col text-slate-400 gap-3">
            <FileText size={48} className="opacity-30" />
            <p className="text-sm">{selectedFile ? 'Loading...' : (isConnected ? 'Select a file to preview' : 'Connecting to backend...')}</p>
          </div>
        ) : (
          <VirtualRenderer ast={ast} onWikiLinkClick={handleWikiLinkClick} />
        )}
      </div>
    </div>
  )
}

export default App
