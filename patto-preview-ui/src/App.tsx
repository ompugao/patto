import { useState, useEffect, useCallback, useRef, useMemo } from 'react'
import VirtualRenderer, { AstNode } from './components/VirtualRenderer'
import PrintRenderer from './components/PrintRenderer'
import { FileText, Folder, Search, PanelLeftClose, PanelLeftOpen, Pin, PinOff } from 'lucide-react'
import type { FileEntry, ServerMessage } from './protocol'
import { noteFromLocation, useNoteHistory } from './hooks/useNoteHistory'
import { usePreviewSocket } from './hooks/usePreviewSocket'

const byNewest = (entries: FileEntry[]) => [...entries].sort((a, b) => b.modified - a.modified)

function App() {
  const [ast, setAst] = useState<AstNode | null>(null)
  const [files, setFiles] = useState<FileEntry[]>([])
  const [pinnedFiles, setPinnedFiles] = useState<string[]>([])
  const [searchQuery, setSearchQuery] = useState('')
  // Seeded from `?note=` so a deep link shows "Loading..." rather than the empty state
  const [selectedFile, setSelectedFile] = useState<string | null>(() => noteFromLocation())
  const [sidebarOpen, setSidebarOpen] = useState(true)
  const [highlightedIndex, setHighlightedIndex] = useState<number>(-1)
  const [hoveredFile, setHoveredFile] = useState<string | null>(null)

  // Socket callbacks can run before React has re-rendered, so they read the
  // selection from a ref that moves in the same tick as the state does.
  const selectedFileRef = useRef<string | null>(selectedFile)
  const setSelected = useCallback((path: string | null) => {
    selectedFileRef.current = path
    setSelectedFile(path)
  }, [])

  // Show a note without touching history — for Back/Forward, which has already
  // moved it, and for the empty state when `note` is null.
  const showNote = useCallback((note: string | null) => {
    setSelected(note)
    setAst(null) // Clear while loading
  }, [setSelected])

  const { scrollRef, pushNote, replaceNote, clearNote } = useNoteHistory({
    content: ast,
    onNavigate: showNote,
  })

  const handleMessage = useCallback((msg: ServerMessage, isOwnReply: boolean) => {
    switch (msg.type) {
      case 'FileList': {
        const paths = msg.data.files ?? []
        const metadata = msg.data.metadata ?? {}
        setFiles(byNewest(paths.map(path => ({ path, modified: metadata[path]?.modified ?? 0 }))))

        // A `?note=` naming a file that is not in the workspace would otherwise
        // sit on "Loading..." forever, since the server answers it with nothing.
        const current = selectedFileRef.current
        if (current && !paths.includes(current)) {
          console.warn('[patto] note from URL not found in workspace:', current)
          showNote(null)
          clearNote()
        }
        break
      }

      case 'FileChanged': {
        const { path, metadata, ast: changed } = msg.data
        if (isOwnReply) {
          // Show it only if it is still what we are on: pressing Back before it
          // arrived means we have moved on since asking.
          if (path === selectedFileRef.current) setAst(changed ?? null)
        } else {
          // A broadcast — some file changed on disk, including Neovim buffers
          // arriving over the LSP bridge — and it takes over the view. Keep the
          // URL honest about that, but replace rather than push: a `git pull`
          // touching many files would otherwise shred the back stack.
          if (path !== selectedFileRef.current) {
            setSelected(path)
            replaceNote(path)
          }
          setAst(changed ?? null)
        }
        if (metadata) {
          setFiles(prev => byNewest(
            prev.map(f => (f.path === path ? { ...f, modified: metadata.modified } : f))))
        }
        break
      }

      case 'FileAdded':
        if (msg.data.path && msg.data.metadata) {
          setFiles(prev => byNewest([...prev, { path: msg.data.path, modified: msg.data.metadata.modified }]))
        }
        break

      case 'FileRemoved':
        if (msg.data.path) setFiles(prev => prev.filter(f => f.path !== msg.data.path))
        break

      case 'PinnedFiles':
        setPinnedFiles(msg.data.pinned ?? [])
        break
    }
  }, [clearNote, replaceNote, setSelected, showNote])

  const { isConnected, selectNote, setPinned } = usePreviewSocket({ onMessage: handleMessage })

  // Ask the server for whatever we are meant to be showing. Running again on
  // reconnect is what brings the view back when the server restarts, and it also
  // covers the `?note=` we were opened with, which is selected before we connect.
  useEffect(() => {
    if (isConnected && selectedFile) selectNote(selectedFile)
  }, [isConnected, selectedFile, selectNote])

  // User-intent navigation: a sidebar click, fuzzy-find Enter, or a wiki link.
  const handleSelectFile = useCallback((path: string) => {
    // Re-selecting the open note only makes sure the URL agrees; re-showing it
    // would blank the view with nothing on the way to replace it.
    if (path === selectedFileRef.current) {
      replaceNote(path)
      return
    }
    pushNote(path)
    showNote(path)
  }, [pushNote, replaceNote, showNote])

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
    setPinned(path, !pinnedFiles.includes(path));
  }, [pinnedFiles, setPinned]);

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
