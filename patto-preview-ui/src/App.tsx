import { useState, useEffect, useCallback, useRef, useMemo } from 'react'
import VirtualRenderer, { AstNode } from './components/VirtualRenderer'
import PrintRenderer from './components/PrintRenderer'
import { FileText, Folder, Search, PanelLeftClose, PanelLeftOpen, Pin, PinOff } from 'lucide-react'
import {
  noteFromLocation,
  noteUrl,
  currentHistoryState,
  nextEntryKey,
  saveScrollForEntry,
  scrollForEntry,
  type HistoryState,
} from './noteUrl'

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
  const [pinnedFiles, setPinnedFiles] = useState<string[]>([])
  const [searchQuery, setSearchQuery] = useState('')
  // Seeded from `?note=` so a deep link shows "Loading..." rather than the empty state
  const [selectedFile, setSelectedFile] = useState<string | null>(() => noteFromLocation())
  const [isConnected, setIsConnected] = useState(false)
  const [sidebarOpen, setSidebarOpen] = useState(true)
  const [highlightedIndex, setHighlightedIndex] = useState<number>(-1)
  const [hoveredFile, setHoveredFile] = useState<string | null>(null)
  // Use a ref for WS so handleSelectFile always has the live socket, no stale closure
  const wsRef = useRef<WebSocket | null>(null)
  // The socket handlers are built once, so they need a ref to see the live selection
  const selectedFileRef = useRef<string | null>(selectedFile)
  // Scroll container of the renderer, so history entries can carry a scroll offset
  const scrollRef = useRef<HTMLDivElement | null>(null)
  const pendingScrollRef = useRef<number | null>(null)
  const initializedRef = useRef(false)
  // Which history entry we are on, so its scroll offset can be found again
  const currentKeyRef = useRef<string | null>(null)
  // Paths we asked for and have not seen answered. `FileChanged` is used both as
  // the reply to SelectFile and as the broadcast for any file changing on disk,
  // and only this queue tells the two apart.
  const pendingRequestsRef = useRef<string[]>([])

  const setSelected = useCallback((path: string | null) => {
    selectedFileRef.current = path
    setSelectedFile(path)
  }, [])

  // Every entry we create carries a key. `push` starts a new entry (user-intent
  // navigation); otherwise the current one is corrected in place.
  const writeHistory = useCallback((note: string | null, url?: string, push = false) => {
    const key = push ? nextEntryKey() : currentHistoryState()?.key ?? nextEntryKey()
    const state: HistoryState = { note, key }
    if (push) window.history.pushState(state, '', url)
    else window.history.replaceState(state, '', url)
    currentKeyRef.current = key
  }, [])

  const sendSelectFile = useCallback((path: string) => {
    const socket = wsRef.current;
    if (socket && socket.readyState === WebSocket.OPEN) {
      // Backend WsClientMessage: #[serde(tag = "type", content = "data")]
      socket.send(JSON.stringify({ type: 'SelectFile', data: { path } }));
      pendingRequestsRef.current.push(path);
      console.log('[patto] SelectFile sent:', path);
    } else {
      console.warn('[patto] WebSocket not open, state:', socket?.readyState);
    }
  }, [])

  // Show a note without touching history — for popstate, reconnect and initial load
  const applyNote = useCallback((path: string) => {
    setSelected(path);
    setAst(null); // Clear while loading
    sendSelectFile(path);
  }, [setSelected, sendSelectFile])

  useEffect(() => {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    // Use the port we were served from, so `--port` works; in `vite dev` the
    // dev-server proxy (see vite.config.ts) forwards /ws to the backend.
    const wsUrl = `${protocol}//${window.location.host}/ws`;

    const connect = () => {
      const socket = new WebSocket(wsUrl);
      wsRef.current = socket;

      socket.onopen = () => {
        if (wsRef.current !== socket) return;
        console.log('[patto] WebSocket connected to', wsUrl);
        setIsConnected(true);

        // Restore what we should be showing: on first connect that comes from
        // `?note=` (which `lua/patto_preview.lua` sets when Neovim opens the
        // preview); on a reconnect it is whatever we were already showing.
        const path = selectedFileRef.current ?? noteFromLocation();
        if (path) {
          if (initializedRef.current) {
            // Reconnect: keep the stale render up until the fresh AST lands
            sendSelectFile(path);
          } else {
            writeHistory(path, noteUrl(path));
            pendingScrollRef.current = scrollForEntry(currentKeyRef.current);
            applyNote(path);
          }
        }
        initializedRef.current = true;
      };

      socket.onmessage = (event) => {
        if (wsRef.current !== socket) return;
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

            // A `?note=` pointing at a file that does not exist would otherwise
            // sit on "Loading..." forever, since the server answers with nothing.
            const current = selectedFileRef.current;
            if (current && !filePaths.includes(current)) {
              console.warn('[patto] note from URL not found in workspace:', current);
              setSelected(null);
              setAst(null);
              writeHistory(null, window.location.pathname);
            }

          } else if (msg.type === 'FileChanged') {
            console.log('[patto] FileChanged ast:', JSON.stringify(data.ast).substring(0, 200));
            // Is this the answer to a SelectFile we sent? Splicing off everything
            // up to the match also drops requests that were never answered (an
            // unreadable file), so the queue cannot drift out of step.
            const queue = pendingRequestsRef.current;
            const queued = data.path ? queue.indexOf(data.path) : -1;
            if (queued !== -1) queue.splice(0, queued + 1);

            if (queued !== -1) {
              // Our own reply. Render it only if it is still what we are showing:
              // pressing Back before it arrived means we have moved on since.
              if (data.path === selectedFileRef.current) setAst(data.ast ?? null);
            } else {
              // A broadcast: some file changed on disk, including Neovim buffers
              // arriving over the LSP bridge, and it takes over the view. Keep the
              // URL honest about that, but only replace — pushing here would let a
              // `git pull` touching many files shred the back stack.
              if (data.path && data.path !== selectedFileRef.current) {
                setSelected(data.path);
                writeHistory(data.path, noteUrl(data.path));
              }
              setAst(data.ast ?? null);
            }
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
          } else if (msg.type === 'PinnedFiles') {
            setPinnedFiles(data.pinned || []);
          }
        } catch (e) {
          console.error('[patto] Failed to parse websocket message', e, event.data);
        }
      };

      socket.onclose = () => {
        // A superseded socket (StrictMode's double-mount, or a reconnect that
        // already landed) must not clear the live one or start a second loop.
        if (wsRef.current !== socket) return;
        console.log('[patto] WebSocket closed, reconnecting in 2s...');
        setIsConnected(false);
        pendingRequestsRef.current = [];
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
    // These callbacks are all stable, so the socket is still built exactly once
  }, [applyNote, sendSelectFile, setSelected, writeHistory]);

  // Seed a key on the entry we landed on, so scrolling has somewhere to record to
  useEffect(() => {
    if (!currentHistoryState()) writeHistory(noteFromLocation());
    else currentKeyRef.current = currentHistoryState()!.key;
  }, [writeHistory]);

  // User-intent navigation: sidebar click, fuzzy-find Enter, or a wiki link.
  // Re-selecting the open note refreshes it without stacking a duplicate entry.
  const handleSelectFile = useCallback((path: string) => {
    writeHistory(path, noteUrl(path), path !== selectedFileRef.current);
    applyNote(path);
  }, [applyNote, writeHistory]);

  // Back/Forward, including mouse side-buttons and Alt+Left/Right.
  // Never pushes — StrictMode double-invokes effects in dev, and this must not
  // fabricate entries.
  useEffect(() => {
    const onPopState = (e: PopStateEvent) => {
      const state = e.state as HistoryState | null;
      currentKeyRef.current = state?.key ?? null;
      const path = state?.note ?? noteFromLocation();
      if (path) {
        pendingScrollRef.current = scrollForEntry(currentKeyRef.current);
        applyNote(path);
      } else {
        pendingScrollRef.current = null;
        setSelected(null);
        setAst(null);
      }
    };
    window.addEventListener('popstate', onPopState);
    return () => window.removeEventListener('popstate', onPopState);
  }, [applyNote, setSelected]);

  // Record the offset continuously against the current entry, so it is already
  // saved however the user leaves — a click, Back, Forward, or a reload. Saving
  // only on the way out would miss Back, which fires after the entry has changed.
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    let raf = 0;
    const onScroll = () => {
      if (raf) return;
      raf = requestAnimationFrame(() => {
        raf = 0;
        // Ignore the frames while a restore is still settling, or we would
        // record the offset we are on our way *from*.
        if (pendingScrollRef.current != null) return;
        if (currentKeyRef.current) saveScrollForEntry(currentKeyRef.current, el.scrollTop);
      });
    };
    el.addEventListener('scroll', onScroll, { passive: true });
    return () => {
      el.removeEventListener('scroll', onScroll);
      if (raf) cancelAnimationFrame(raf);
    };
  }, [ast]);

  // Restore the scroll offset a Back/Forward asked for. The virtualizer measures
  // rows lazily, so the container keeps growing for a few frames after the AST
  // lands and an early write gets clamped — retry until it sticks, then give up.
  useEffect(() => {
    const target = pendingScrollRef.current;
    if (!ast || target == null) return;
    if (target === 0) {
      pendingScrollRef.current = null;
      return;
    }
    let frames = 0;
    let raf = 0;
    const step = () => {
      const el = scrollRef.current;
      if (el) {
        el.scrollTop = target;
        if (Math.abs(el.scrollTop - target) < 1) {
          pendingScrollRef.current = null;
          return;
        }
      }
      if (++frames < 20) {
        raf = requestAnimationFrame(step);
      } else {
        pendingScrollRef.current = null;
      }
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  }, [ast]);

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

  // When no filter is active, show pinned files at the top
  const displayFiles = useMemo(() => {
    if (searchQuery.trim()) return filteredFiles;
    const pinnedSet = new Set(pinnedFiles);
    const pinned = filteredFiles.filter(f => pinnedSet.has(f.path));
    const rest = filteredFiles.filter(f => !pinnedSet.has(f.path));
    return [...pinned, ...rest];
  }, [filteredFiles, pinnedFiles, searchQuery]);

  const handleTogglePin = useCallback((e: React.MouseEvent, path: string) => {
    e.stopPropagation();
    const socket = wsRef.current;
    if (!socket || socket.readyState !== WebSocket.OPEN) return;
    const isPinned = pinnedFiles.includes(path);
    socket.send(JSON.stringify({ type: isPinned ? 'UnpinFile' : 'PinFile', data: { path } }));
  }, [pinnedFiles]);

  // Reset highlight when query changes
  const handleSearchChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setSearchQuery(e.target.value);
    setHighlightedIndex(-1);
  }, []);

  const handleSearchKeyDown = useCallback((e: React.KeyboardEvent<HTMLInputElement>) => {
    const len = displayFiles.length;
    if (len === 0) return;
    if (e.key === 'Tab' || e.key === 'ArrowDown') {
      e.preventDefault();
      setHighlightedIndex(i => (i + 1) % len);
    } else if ((e.key === 'Tab' && e.shiftKey) || e.key === 'ArrowUp') {
      e.preventDefault();
      setHighlightedIndex(i => (i - 1 + len) % len);
    } else if (e.key === 'Enter') {
      const idx = highlightedIndex >= 0 ? highlightedIndex : 0;
      handleSelectFile(displayFiles[idx].path);
    }
  }, [displayFiles, highlightedIndex, handleSelectFile]);

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
          {displayFiles.length === 0 ? (
            <div className="p-4 text-slate-400 italic text-center text-xs">
              {isConnected ? 'No files found' : 'Connecting...'}
            </div>
          ) : (
            displayFiles.map((file, idx) => {
              const isHighlighted = idx === highlightedIndex;
              const isSelected = selectedFile === file.path;
              const isPinned = pinnedFiles.includes(file.path);
              const isHovered = hoveredFile === file.path;
              return (
                <div
                  key={file.path}
                  ref={el => { if (isHighlighted && el) el.scrollIntoView({ block: 'nearest' }); }}
                  onClick={() => handleSelectFile(file.path)}
                  onMouseEnter={() => setHoveredFile(file.path)}
                  onMouseLeave={() => setHoveredFile(null)}
                  className={`flex items-center gap-2 px-3 py-1.5 cursor-pointer rounded-md transition-colors ${isSelected
                      ? 'bg-blue-100 text-blue-700 font-medium'
                      : isHighlighted
                        ? 'bg-slate-200 text-slate-800'
                        : 'hover:bg-slate-200 text-slate-600'
                    }`}
                >
                  <FileText size={14} className={isSelected ? 'text-blue-500' : 'text-slate-400 min-w-4 max-w-4'} />
                  <span className="truncate flex-1" title={file.path}>{file.path.split('/').pop()}</span>
                  {(isHovered || isPinned) && (
                    <button
                      onClick={e => handleTogglePin(e, file.path)}
                      title={isPinned ? 'Unpin' : 'Pin to top'}
                      className={`shrink-0 transition-colors ${isPinned ? 'text-blue-500 hover:text-slate-400' : 'text-slate-300 hover:text-slate-500'}`}
                    >
                      {isPinned ? <Pin size={12} /> : <PinOff size={12} />}
                    </button>
                  )}
                </div>
              );
            })
          )}
        </div>
      </div>

      {/* Main Content Area */}
      <div className="flex-1 overflow-hidden h-full relative">
        {/* Toolbar — always visible */}
        {!sidebarOpen && (
          <div className="no-print absolute top-2 left-2 z-10">
            <button
              onClick={() => setSidebarOpen(true)}
              title="Open sidebar"
              className="p-1.5 rounded-md bg-white border border-slate-200 text-slate-400 hover:text-slate-600 hover:bg-slate-50 shadow-sm"
            >
              <PanelLeftOpen size={16} />
            </button>
          </div>
        )}
        {!ast ? (
          <div className="flex items-center justify-center h-full flex-col text-slate-400 gap-3">
            <FileText size={48} className="opacity-30" />
            <p className="text-sm">{selectedFile ? 'Loading...' : (isConnected ? 'Select a file to preview' : 'Connecting to backend...')}</p>
          </div>
        ) : (
          <>
            <div className="screen-only h-full">
              <VirtualRenderer ast={ast} onWikiLinkClick={handleWikiLinkClick} scrollElementRef={scrollRef} />
            </div>
            <PrintRenderer ast={ast} onWikiLinkClick={handleWikiLinkClick} />
          </>
        )}
      </div>
    </div>
  )
}

export default App
