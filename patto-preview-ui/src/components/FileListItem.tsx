import { FileText, Pin, PinOff } from 'lucide-react'
import { fileName } from '../fileList'

interface FileListItemProps {
  path: string;
  isSelected: boolean;
  isHighlighted: boolean;
  isPinned: boolean;
  isHovered: boolean;
  onSelect: () => void;
  onTogglePin: () => void;
  onHover: (hovered: boolean) => void;
}

export default function FileListItem({
  path, isSelected, isHighlighted, isPinned, isHovered, onSelect, onTogglePin, onHover,
}: FileListItemProps) {
  return (
    <div
      ref={el => { if (isHighlighted && el) el.scrollIntoView({ block: 'nearest' }); }}
      onClick={onSelect}
      onMouseEnter={() => onHover(true)}
      onMouseLeave={() => onHover(false)}
      className={`flex items-center gap-2 px-3 py-1.5 cursor-pointer rounded-md transition-colors ${isSelected
          ? 'bg-blue-100 text-blue-700 font-medium'
          : isHighlighted
            ? 'bg-slate-200 text-slate-800'
            : 'hover:bg-slate-200 text-slate-600'
        }`}
    >
      <FileText size={14} className={isSelected ? 'text-blue-500' : 'text-slate-400 min-w-4 max-w-4'} />
      <span className="truncate flex-1" title={path}>{fileName(path)}</span>
      {(isHovered || isPinned) && (
        <button
          onClick={e => { e.stopPropagation(); onTogglePin(); }}
          title={isPinned ? 'Unpin' : 'Pin to top'}
          className={`shrink-0 transition-colors ${isPinned ? 'text-blue-500 hover:text-slate-400' : 'text-slate-300 hover:text-slate-500'}`}
        >
          {isPinned ? <Pin size={12} /> : <PinOff size={12} />}
        </button>
      )}
    </div>
  )
}
