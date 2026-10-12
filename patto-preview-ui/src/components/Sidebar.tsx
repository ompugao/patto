import { Folder, List, PanelLeftClose } from 'lucide-react'
import type { FileEntry } from '../protocol'
import { BULLET_STYLES, type BulletStyle } from './BulletStyle'
import FileList from './FileList'

interface SidebarProps {
  open: boolean;
  onClose: () => void;
  isConnected: boolean;
  files: FileEntry[];
  pinnedFiles: string[];
  selectedFile: string | null;
  onSelectFile: (path: string) => void;
  onTogglePin: (path: string) => void;
  bulletStyle: BulletStyle;
  onBulletStyleChange: (style: BulletStyle) => void;
}

export default function Sidebar({
  open, onClose, isConnected, files, pinnedFiles, selectedFile, onSelectFile, onTogglePin, bulletStyle, onBulletStyleChange,
}: SidebarProps) {
  return (
    <div
      className="border-r border-slate-200 bg-slate-50 flex flex-col overflow-hidden transition-all duration-200"
      style={{ width: open ? '17rem' : '0', minWidth: open ? '17rem' : '0' }}
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
          <button onClick={onClose} title="Close sidebar" className="text-slate-400 hover:text-slate-600">
            <PanelLeftClose size={16} />
          </button>
        </div>
      </div>

      <FileList
        files={files}
        pinnedFiles={pinnedFiles}
        selectedFile={selectedFile}
        isConnected={isConnected}
        onSelectFile={onSelectFile}
        onTogglePin={onTogglePin}
      />

      <div className="px-3 py-2 border-t border-slate-200 min-w-[17rem] flex items-center gap-2 text-xs text-slate-500">
        <List size={14} className="text-slate-400" />
        <label htmlFor="bullet-style">Bullets</label>
        <select
          id="bullet-style"
          value={bulletStyle}
          onChange={e => onBulletStyleChange(e.target.value as BulletStyle)}
          className="ml-auto px-1.5 py-0.5 bg-white border border-slate-200 rounded-md text-slate-700 focus:outline-none focus:ring-1 focus:ring-blue-500"
        >
          {BULLET_STYLES.map(s => <option key={s.value} value={s.value}>{s.label}</option>)}
        </select>
      </div>
    </div>
  )
}
