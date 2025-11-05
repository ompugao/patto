'use client';

import { useState, useCallback, useEffect } from 'react';
import { useClientRouter } from '../lib/router';
import { usePattoWebSocket } from '../lib/websocket';
import Sidebar from '../components/Sidebar';
import Preview from '../components/Preview';
import TaskPanel from '../components/TaskPanel';
import styles from './page.module.css';

export default function PattoApp() {
  const { currentNote, anchor, navigate, navigateHome } = useClientRouter();
  
  // State
  const [files, setFiles] = useState([]);
  const [fileMetadata, setFileMetadata] = useState({});
  const [previewHtml, setPreviewHtml] = useState('');
  const [backLinks, setBackLinks] = useState([]);
  const [twoHopLinks, setTwoHopLinks] = useState([]);
  const [sortBy, setSortBy] = useState('modified');
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [tasks, setTasks] = useState([]);
  const [taskPanelOpen, setTaskPanelOpen] = useState(false);
  const [targetLineId, setTargetLineId] = useState(null);

  // Initialize from localStorage after mount
  useEffect(() => {
    if (typeof window !== 'undefined') {
      const savedSort = localStorage.getItem('patto-sort-order');
      if (savedSort) setSortBy(savedSort);
      
      const savedCollapsed = localStorage.getItem('sidebar-collapsed');
      if (savedCollapsed === 'true') setSidebarCollapsed(true);
      
      const savedTaskPanelOpen = localStorage.getItem('task-panel-open');
      if (savedTaskPanelOpen === 'true') setTaskPanelOpen(true);
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
        setFiles(prev => prev.includes(data.data.path) ? prev : [...prev, data.data.path]);
        setFileMetadata(prev => ({
          ...prev,
          [data.data.path]: data.data.metadata
        }));
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
          setBackLinks([]);
          setTwoHopLinks([]);
          navigateHome();
        }
        break;
        
      case 'BackLinksData':
        if (data.data.path === currentNote) {
          setBackLinks(data.data.back_links || []);
        }
        break;
        
      case 'TwoHopLinksData':
        if (data.data.path === currentNote) {
          setTwoHopLinks(data.data.two_hop_links || []);
        }
        break;
        
      case 'TasksUpdated':
        setTasks(data.data.tasks || []);
        break;
    }
  });

  // Handle current note changes
  useEffect(() => {
    if (currentNote && sendMessage) {
      setPreviewHtml(''); // Clear previous content
      setBackLinks([]); // Clear previous back-links
      setTwoHopLinks([]); // Clear previous two-hop links
      sendMessage({ type: 'SelectFile', data: { path: currentNote } });
    } else if (!currentNote) {
      setPreviewHtml('');
      setBackLinks([]);
      setTwoHopLinks([]);
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

  // Scroll to target line after content loads
  useEffect(() => {
    if (previewHtml && targetLineId !== null) {
      setTimeout(() => {
        const { stableId, row } = typeof targetLineId === 'object' ? targetLineId : { stableId: targetLineId, row: null };
        console.log('Attempting to scroll after navigation:', { stableId, row });
        
        let element = null;
        
        // Try stable ID first
        if (stableId !== null && stableId !== undefined) {
          element = document.querySelector(`[data-line-id="${stableId}"]`);
          if (element) {
            console.log('Found element by stable ID');
          }
        }
        
        // Fall back to row position
        if (!element && row !== null && row !== undefined) {
          const preview = document.querySelector('#preview-content');
          if (preview) {
            const lines = preview.querySelectorAll('li');
            if (row < lines.length) {
              element = lines[row];
              console.log(`Found element by row position: ${row}`);
            }
          }
        }
        
        if (element) {
          element.scrollIntoView({ 
            behavior: 'smooth', 
            block: 'center' 
          });
          element.classList.add('highlighted');
          setTimeout(() => element.classList.remove('highlighted'), 2000);
        } else {
          console.warn('Element not found after navigation:', { stableId, row });
        }
        setTargetLineId(null);
      }, 500);
    }
  }, [previewHtml, targetLineId]);

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

  // Handle task panel toggle
  const handleToggleTaskPanel = useCallback(() => {
    setTaskPanelOpen(prev => {
      const newValue = !prev;
      if (typeof window !== 'undefined') {
        localStorage.setItem('task-panel-open', newValue.toString());
      }
      return newValue;
    });
  }, []);

  // Handle task click - navigate to file and scroll to line
  const handleTaskClick = useCallback((filePath, stableId, row) => {
    console.log('Task clicked:', { filePath, stableId, row, currentNote });
    
    // If clicking the same file, just scroll
    if (currentNote === filePath) {
      let element = null;
      
      // Try stable ID first (if available from line-tracked rendering)
      if (stableId !== null && stableId !== undefined) {
        element = document.querySelector(`[data-line-id="${stableId}"]`);
        if (element) {
          console.log('Found element by stable ID');
        }
      }
      
      // Fall back to finding by position (nth li in preview)
      if (!element && row !== null && row !== undefined) {
        const preview = document.querySelector('#preview-content');
        if (preview) {
          const lines = preview.querySelectorAll('li');
          // Row is 0-indexed in parser, find the matching line
          if (row < lines.length) {
            element = lines[row];
            console.log(`Found element by row position: ${row}`);
          }
        }
      }
      
      if (element) {
        element.scrollIntoView({ 
          behavior: 'smooth', 
          block: 'center' 
        });
        element.classList.add('highlighted');
        setTimeout(() => element.classList.remove('highlighted'), 2000);
      } else {
        console.warn('Could not find element to scroll to', { stableId, row });
      }
    } else {
      // Navigate to different file
      navigate(filePath);
      // Store both stableId and row for after navigation
      if (stableId !== null || row !== null) {
        setTargetLineId({ stableId, row });
      }
    }
  }, [navigate, currentNote]);

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

        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden', position: 'relative' }}>
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
          
          {/* Task Panel */}
          <TaskPanel
            tasks={tasks}
            isOpen={taskPanelOpen}
            onToggle={handleToggleTaskPanel}
            onTaskClick={handleTaskClick}
          />
        </div>
      </div>
    </div>
  );
}

