import type { RefObject } from 'react'
import { FileText } from 'lucide-react'
import type { AstNode } from '../ast'
import { BulletStyleContext, type BulletStyle } from './BulletStyle'
import VirtualRenderer from './VirtualRenderer'
import PrintRenderer from './PrintRenderer'
import type { WikiLinkHandler } from './RenderNode'

interface NoteViewProps {
  ast: AstNode | null;
  selectedFile: string | null;
  isConnected: boolean;
  bulletStyle: BulletStyle;
  scrollRef: RefObject<HTMLDivElement>;
  onWikiLinkClick: WikiLinkHandler;
}

export default function NoteView({ ast, selectedFile, isConnected, bulletStyle, scrollRef, onWikiLinkClick }: NoteViewProps) {
  if (!ast) {
    return (
      <div className="flex items-center justify-center h-full flex-col text-slate-400 gap-3">
        <FileText size={48} className="opacity-30" />
        <p className="text-sm">{selectedFile ? 'Loading...' : (isConnected ? 'Select a file to preview' : 'Connecting to backend...')}</p>
      </div>
    )
  }

  return (
    <BulletStyleContext.Provider value={bulletStyle}>
      <div className="screen-only h-full">
        <VirtualRenderer ast={ast} onWikiLinkClick={onWikiLinkClick} scrollElementRef={scrollRef} />
      </div>
      <PrintRenderer ast={ast} onWikiLinkClick={onWikiLinkClick} />
    </BulletStyleContext.Provider>
  )
}
