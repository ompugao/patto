import { useState, useEffect, useCallback, useRef } from 'react'
import VirtualRenderer, { AstNode } from './components/VirtualRenderer'
import { FileText, Folder, Signal, SignalZero } from 'lucide-react'

function App() {
  const [ast, setAst] = useState<AstNode | null>(null)
  const [files, setFiles] = useState<string[]>([])
  const [selectedFile, setSelectedFile] = useState<string | null>(null)
  const [isConnected, setIsConnected] = useState(false)
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
            setFiles(data.files || []);
          } else if (msg.type === 'FileChanged') {
            console.log('[patto] FileChanged ast:', JSON.stringify(data.ast).substring(0, 200));
            setAst(data.ast ?? null);
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
    const targetFile = files.find(f => f.replace(/\.pn$/, '') === link || f === link || f === `${link}.pn`);
    if (targetFile) {
      handleSelectFile(targetFile);
    }
  }, [files, handleSelectFile]);

  return (
    <div className="flex h-screen w-screen bg-white overflow-hidden text-slate-800">
      {/* Sidebar */}
      <div className="w-64 min-w-[16rem] border-r border-slate-200 bg-slate-50 flex flex-col">
        <div className="px-4 py-3 border-b border-slate-200 flex justify-between items-center">
          <h2 className="font-semibold flex items-center gap-2 text-sm">
            <Folder size={16} className="text-slate-500" />
            Workspace
          </h2>
          <div title={isConnected ? 'Connected' : 'Connecting...'}>
            {isConnected
              ? <Signal size={14} className="text-green-500" />
              : <SignalZero size={14} className="text-slate-400 animate-pulse" />}
          </div>
        </div>
        <div className="flex-1 overflow-y-auto p-1 text-sm">
          {files.length === 0 ? (
            <div className="p-4 text-slate-400 italic text-center text-xs">
              {isConnected ? 'No files found' : 'Connecting...'}
            </div>
          ) : (
            files.map(file => (
              <div
                key={file}
                onClick={() => handleSelectFile(file)}
                className={`flex items-center gap-2 px-3 py-1.5 cursor-pointer rounded-md transition-colors ${selectedFile === file
                  ? 'bg-blue-100 text-blue-700 font-medium'
                  : 'hover:bg-slate-200 text-slate-600'
                  }`}
              >
                <FileText size={14} className={selectedFile === file ? 'text-blue-500' : 'text-slate-400'} />
                <span className="truncate" title={file}>{file.split('/').pop()}</span>
              </div>
            ))
          )}
        </div>
      </div>

      {/* Main Content Area */}
      <div className="flex-1 overflow-hidden h-full">
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
