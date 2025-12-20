// Patto Mobile - Main App Component

import { useStore, View } from './lib/store';
import { FileList } from './components/FileList';
import { NoteView } from './components/NoteView';
import { NoteEditor } from './components/NoteEditor';
import { TaskPanel } from './components/TaskPanel';
import { GitConfig } from './components/GitConfig';
import { useEffect } from 'react';
import './App.css';

function App() {
  const {
    currentView,
    workspacePath,
    loadFiles,
    setView,
  } = useStore();

  // Load files when workspace is set
  useEffect(() => {
    if (workspacePath) {
      loadFiles();
    }
  }, [workspacePath, loadFiles]);

  // Show config if no workspace selected
  useEffect(() => {
    if (!workspacePath) {
      setView(View.GIT_CONFIG);
    }
  }, [workspacePath, setView]);

  const renderView = () => {
    switch (currentView) {
      case View.FILE_LIST:
        return <FileList />;
      case View.NOTE_VIEW:
        return <NoteView />;
      case View.NOTE_EDIT:
        return <NoteEditor />;
      case View.TASKS:
        return <TaskPanel />;
      case View.GIT_CONFIG:
        return <GitConfig />;
      default:
        return <FileList />;
    }
  };

  return (
    <div className="app">
      {renderView()}
    </div>
  );
}

export default App;
