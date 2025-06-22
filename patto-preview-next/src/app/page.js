'use client';

import { useState, useCallback, useEffect } from 'react';
import { useClientRouter } from '../lib/router';
import { usePattoWebSocket } from '../lib/websocket';
import Sidebar from '../components/Sidebar';
import Preview from '../components/Preview';
import styles from './page.module.css';

export default function PattoApp() {
  const { currentNote, anchor, navigate, navigateHome } = useClientRouter();
  
  // State
  const [files, setFiles] = useState([]);
  const [fileMetadata, setFileMetadata] = useState({});
  const [previewHtml, setPreviewHtml] = useState('');
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

  // Get WebSocket sendMessage function first
  const { sendMessage } = usePattoWebSocket((data) => {
    console.log('WebSocket message:', data);
    
    switch (data.type) {
      case 'FileList':
        setFiles(data.data.files || []);
        setFileMetadata(data.data.metadata || {});
        
        // If we have a current note from URL, request it
        if (currentNote && sendMessage) {
          sendMessage({ type: 'SelectFile', data: { path: currentNote } });
        }
        break;
        
      case 'FileChanged':
        if (data.data.path === currentNote) {
          setPreviewHtml(data.data.html || '');
        }
        break;
        
      case 'FileAdded':
        setFiles(prev => prev.includes(data.data.path) ? prev : [...prev, data.data.path]);
        setFileMetadata(prev => ({
          ...prev,
          [data.data.path]: data.data.metadata
        }));
        break;
        
      case 'FileRemoved':
        setFiles(prev => prev.filter(f => f !== data.data.path));
        setFileMetadata(prev => {
          const newMetadata = { ...prev };
          delete newMetadata[data.data.path];
          return newMetadata;
        });
        
        // Clear preview if current file was removed
        if (currentNote === data.data.path) {
          setPreviewHtml('');
          navigateHome();
        }
        break;
    }
  });

  // Handle current note changes
  useEffect(() => {
    if (currentNote && sendMessage) {
      setPreviewHtml(''); // Clear previous content
      sendMessage({ type: 'SelectFile', data: { path: currentNote } });
    } else if (!currentNote) {
      setPreviewHtml('');
    }
  }, [currentNote, sendMessage]);

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
        {/*
        <button onClick={navigateHome} style={{ 
          background: 'none', 
          border: 'none', 
          color: 'white', 
          textDecoration: 'none',
          cursor: 'pointer',
          fontSize: 'inherit'
        }}>
        🏠
        </button>
        */}
        <span style={{fontSize: 'larger', height: '1.2em'}} >{currentNote || ''}</span>
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

        <div style={{ flex: 1, overflow: 'auto', padding: '20px' }}>
          <Preview html={previewHtml} anchor={anchor} onSelectFile={navigate} currentNote={currentNote} />
        </div>
      </div>
    </div>
  );
}

