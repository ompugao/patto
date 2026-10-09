import { useState } from 'react'
import { PanelLeftOpen } from 'lucide-react'
import { useNotePreview } from './hooks/useNotePreview'
import { usePersistedBulletStyle } from './components/BulletStyle'
import Sidebar from './components/Sidebar'
import NoteView from './components/NoteView'

function App() {
  const [sidebarOpen, setSidebarOpen] = useState(true)
  const [bulletStyle, setBulletStyle] = usePersistedBulletStyle()
  const preview = useNotePreview()

  return (
    <div className="flex h-screen w-screen bg-white overflow-hidden text-slate-800">
      <Sidebar
        open={sidebarOpen}
        onClose={() => setSidebarOpen(false)}
        isConnected={preview.isConnected}
        files={preview.files}
        pinnedFiles={preview.pinnedFiles}
        selectedFile={preview.selectedFile}
        onSelectFile={preview.selectFile}
        onTogglePin={preview.togglePin}
        bulletStyle={bulletStyle}
        onBulletStyleChange={setBulletStyle}
      />

      <div className="flex-1 overflow-hidden h-full relative">
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
        <NoteView
          ast={preview.ast}
          selectedFile={preview.selectedFile}
          isConnected={preview.isConnected}
          bulletStyle={bulletStyle}
          scrollRef={preview.scrollRef}
          onWikiLinkClick={preview.openWikiLink}
        />
      </div>
    </div>
  )
}

export default App
