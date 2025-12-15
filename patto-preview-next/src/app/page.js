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
  const connectionLabel = {
    [ConnectionState.CONNECTED]: 'Connected',
    [ConnectionState.CONNECTING]: 'Connecting',
    [ConnectionState.RECONNECTING]: 'Reconnecting',
    [ConnectionState.DISCONNECTED]: 'Disconnected',
  }[connectionState] || 'Disconnected';

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh' }}>
      {/* Header */}
      <div style={{
        backgroundColor: '#333',
        color: 'white',
        padding: '10px 15px',
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        gap: '12px'
      }}>
        <span style={{ fontSize: 'larger', height: '1.2em', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
          {currentNote || ''}
        </span>
        <button
          type="button"
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            padding: '4px 8px',
            borderRadius: '12px',
            border: '1px solid rgba(255, 255, 255, 0.12)',
            backgroundColor: 'rgba(255, 255, 255, 0.04)',
            color: 'rgba(255, 255, 255, 0.82)',
            fontSize: '12px',
            fontWeight: 500,
            cursor: 'default'
          }}
          title={connectionLabel}
          aria-label={`Connection status: ${connectionLabel}`}
        >
          <div style={connectionIndicator} aria-hidden="true" />
          <span aria-live="polite">{connectionLabel}</span>
        </button>
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
