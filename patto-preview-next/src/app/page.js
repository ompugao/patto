'use client';

import { useEffect } from 'react';
import { usePattoStore, getConnectionIndicator, ConnectionState } from '../lib/store';
import Sidebar from '../components/Sidebar';
import Preview from '../components/Preview';

export default function PattoApp() {
  // Select state and actions from Zustand store
  const files = usePattoStore(s => s.files);
  const fileMetadata = usePattoStore(s => s.fileMetadata);
  const previewHtml = usePattoStore(s => s.previewHtml);
  const backLinks = usePattoStore(s => s.backLinks);
  const twoHopLinks = usePattoStore(s => s.twoHopLinks);
  const currentNote = usePattoStore(s => s.currentNote);
  const anchor = usePattoStore(s => s.anchor);
  const sortBy = usePattoStore(s => s.sortBy);
  const sidebarCollapsed = usePattoStore(s => s.sidebarCollapsed);
  const connectionState = usePattoStore(s => s.connectionState);

  // Actions
  const initialize = usePattoStore(s => s.initialize);
  const connect = usePattoStore(s => s.connect);
  const disconnect = usePattoStore(s => s.disconnect);
  const selectFile = usePattoStore(s => s.selectFile);
  const setSortBy = usePattoStore(s => s.setSortBy);
  const toggleSidebar = usePattoStore(s => s.toggleSidebar);

  // Initialize store and WebSocket on mount
  useEffect(() => {
    const cleanup = initialize();
    connect();
    return () => {
      cleanup?.();
      disconnect();
    };
  }, [initialize, connect, disconnect]);

  const connectionIndicator = getConnectionIndicator(connectionState);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh' }}>
      {/* Header */}
      <div style={{
        backgroundColor: '#333',
        color: 'white',
        padding: '10px 15px',
        display: 'flex',
        justifyContent: 'start',
        alignItems: 'center'
      }}>
        <div style={connectionIndicator} title={`Connection: ${connectionState}`} />
        <span style={{ fontSize: 'larger', height: '1.2em' }}>{currentNote || ''}</span>
      </div>

      {/* Main content */}
      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        <Sidebar
          files={files}
          fileMetadata={fileMetadata}
          currentFile={currentNote}
          onSelectFile={selectFile}
          sortBy={sortBy}
          onSortChange={setSortBy}
          collapsed={sidebarCollapsed}
          onToggle={toggleSidebar}
        />

        <div style={{ flex: 1, overflow: 'auto', padding: '10px' }}>
          <Preview
            html={previewHtml}
            anchor={anchor}
            onSelectFile={selectFile}
            currentNote={currentNote}
            backLinks={backLinks}
            twoHopLinks={twoHopLinks}
          />
        </div>
      </div>
    </div>
  );
}

