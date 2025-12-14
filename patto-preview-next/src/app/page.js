'use client';

import { useState, useCallback, useEffect, useMemo } from 'react';
import { useClientRouter } from '../lib/router';
import { usePattoWebSocket, ConnectionState } from '../lib/websocket';
import { usePattoStore, createMessageHandler, createSelectFileMessage } from '../lib/usePattoStore';
import Sidebar from '../components/Sidebar';
import Preview from '../components/Preview';
import styles from './page.module.css';

export default function PattoApp() {
  const { currentNote, anchor, navigate, navigateHome } = useClientRouter();

  // Use centralized store for patto state
  const { state, dispatch, actions } = usePattoStore(currentNote);
  const { files, fileMetadata, previewHtml, backLinks, twoHopLinks } = state;

  // UI state (not related to patto data)
  const [sortBy, setSortBy] = useState('modified');
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

  // Initialize from localStorage after mount
  useEffect(() => {
    if (typeof window !== 'undefined') {
      const savedSort = localStorage.getItem('patto-sort-order');
      if (savedSort) setSortBy(savedSort);

      const savedCollapsed = localStorage.getItem('sidebar-collapsed');
      if (savedCollapsed === 'true') setSidebarCollapsed(true);
    }
  }, []);

  // Create message handler that updates store
  const handleMessage = useCallback((data) => {
    dispatch({ type: data.type, data: data.data });
  }, [dispatch]);

  // Get WebSocket connection
  const { sendMessage, connectionState } = usePattoWebSocket(handleMessage);

  // Track if files have been loaded (for initial file request)
  const hasFilesLoaded = files.length > 0;

  // Request current note on FileList receive (initial load)
  useEffect(() => {
    if (hasFilesLoaded && currentNote && sendMessage) {
      // Small delay to ensure connection is stable
      const timer = setTimeout(() => {
        sendMessage(createSelectFileMessage(currentNote));
      }, 100);
      return () => clearTimeout(timer);
    }
  }, [hasFilesLoaded, currentNote, sendMessage]);

  // Handle current note changes
  useEffect(() => {
    if (currentNote && sendMessage) {
      actions.clearPreview();
      sendMessage(createSelectFileMessage(currentNote));
    } else if (!currentNote) {
      actions.clearPreview();
    }
  }, [currentNote, sendMessage, actions]);

  // Update browser tab title
  useEffect(() => {
    if (typeof window !== 'undefined') {
      if (currentNote) {
        document.title = `${currentNote} - Patto Preview`;
      } else {
        document.title = 'Patto Preview';
      }
    }
  }, [currentNote]);

  // Set initial title from URL on first load
  useEffect(() => {
    if (typeof window !== 'undefined') {
      const urlParams = new URLSearchParams(window.location.search);
      const noteParam = urlParams.get('note');
      if (noteParam && !currentNote) {
        document.title = `${noteParam} - Patto Preview`;
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // Run only once on mount

  // Handle sort preference changes
  const handleSortChange = useCallback((newSort) => {
    setSortBy(newSort);
    if (typeof window !== 'undefined') {
      localStorage.setItem('patto-sort-order', newSort);
    }
  }, []);

  // Handle sidebar toggle
  const handleToggleSidebar = useCallback(() => {
    setSidebarCollapsed(prev => {
      const newValue = !prev;
      if (typeof window !== 'undefined') {
        localStorage.setItem('sidebar-collapsed', newValue.toString());
      }
      return newValue;
    });
  }, []);

  // Connection status indicator style
  const connectionIndicator = useMemo(() => {
    const colors = {
      [ConnectionState.CONNECTED]: '#4caf50',
      [ConnectionState.CONNECTING]: '#ff9800',
      [ConnectionState.RECONNECTING]: '#ff9800',
      [ConnectionState.DISCONNECTED]: '#f44336',
    };
    return {
      width: '8px',
      height: '8px',
      borderRadius: '50%',
      backgroundColor: colors[connectionState] || '#888',
      marginRight: '8px',
    };
  }, [connectionState]);

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
        <span style={{ fontSize: 'larger', height: '1.2em' }} >{currentNote || ''}</span>
      </div>

      {/* Main content */}
      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        <Sidebar
          files={files}
          fileMetadata={fileMetadata}
          currentFile={currentNote}
          onSelectFile={navigate}
          sortBy={sortBy}
          onSortChange={handleSortChange}
          collapsed={sidebarCollapsed}
          onToggle={handleToggleSidebar}
        />

        <div style={{ flex: 1, overflow: 'auto', padding: '10px' }}>
          <Preview
            html={previewHtml}
            anchor={anchor}
            onSelectFile={navigate}
            currentNote={currentNote}
            backLinks={backLinks}
            twoHopLinks={twoHopLinks}
          />
        </div>
      </div>
    </div>
  );
}
