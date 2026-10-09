import { useState } from 'react'
import { Search } from 'lucide-react'
import type { FileEntry } from '../protocol'
import { useFileSearch } from '../hooks/useFileSearch'
import FileListItem from './FileListItem'

interface FileListProps {
  files: FileEntry[];
  pinnedFiles: string[];
  selectedFile: string | null;
  isConnected: boolean;
  onSelectFile: (path: string) => void;
  onTogglePin: (path: string) => void;
}

export default function FileList({ files, pinnedFiles, selectedFile, isConnected, onSelectFile, onTogglePin }: FileListProps) {
  const { query, highlightedIndex, displayFiles, onQueryChange, onKeyDown } = useFileSearch(files, pinnedFiles, onSelectFile)
  const [hoveredFile, setHoveredFile] = useState<string | null>(null)

  return (
    <>
      <div className="p-2 border-b border-slate-200 min-w-[17rem]">
        <div className="relative">
          <Search size={14} className="absolute left-2.5 top-2.5 text-slate-400" />
          <input
            type="text"
            placeholder="Fuzzy find files..."
            value={query}
            onChange={onQueryChange}
            onKeyDown={onKeyDown}
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
          displayFiles.map((file, idx) => (
            <FileListItem
              key={file.path}
              path={file.path}
              isSelected={selectedFile === file.path}
              isHighlighted={idx === highlightedIndex}
              isPinned={pinnedFiles.includes(file.path)}
              isHovered={hoveredFile === file.path}
              onSelect={() => onSelectFile(file.path)}
              onTogglePin={() => onTogglePin(file.path)}
              onHover={hovered => setHoveredFile(hovered ? file.path : null)}
            />
          ))
        )}
      </div>
    </>
  )
}
