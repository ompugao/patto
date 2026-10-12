import { useMemo, useRef, type RefObject } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { flattenAst, type AstNode } from '../ast';
import RenderNode, { type WikiLinkHandler } from './RenderNode';

interface VirtualRendererProps {
    ast: AstNode | null;
    onWikiLinkClick: WikiLinkHandler;
    /** Lets the parent read/write the scroll offset, e.g. to restore it on Back. */
    scrollElementRef?: RefObject<HTMLDivElement>;
}

export default function VirtualRenderer({ ast, onWikiLinkClick, scrollElementRef }: VirtualRendererProps) {
    const internalRef = useRef<HTMLDivElement>(null);
    const parentRef = scrollElementRef ?? internalRef;

    const blocks = useMemo(() => (ast ? flattenAst(ast) : []), [ast]);

    // This app does not run the React Compiler, so the non-memoizable return
    // value the rule warns about is never memoized in the first place.
    // eslint-disable-next-line react-hooks/incompatible-library
    const rowVirtualizer = useVirtualizer({
        count: blocks.length,
        getScrollElement: () => parentRef.current,
        estimateSize: () => 32,
        overscan: 15,
    });

    if (!ast || blocks.length === 0) {
        return (
            <div className="flex items-center justify-center h-full text-slate-400 text-sm">
                Empty document
            </div>
        );
    }

    return (
        <div ref={parentRef} className="h-full overflow-y-auto w-full">
            <div
                style={{
                    height: `${rowVirtualizer.getTotalSize()}px`,
                    width: '100%',
                    position: 'relative',
                }}
            >
                {rowVirtualizer.getVirtualItems().map(virtualItem => (
                    <div
                        key={virtualItem.key}
                        data-index={virtualItem.index}
                        ref={rowVirtualizer.measureElement}
                        style={{
                            position: 'absolute',
                            top: 0,
                            left: 0,
                            width: '100%',
                            transform: `translateY(${virtualItem.start}px)`,
                        }}
                        className="w-full px-8 py-[0.1875em]"
                    >
                        <RenderNode node={blocks[virtualItem.index]} onWikiLinkClick={onWikiLinkClick} />
                    </div>
                ))}
            </div>
        </div>
    );
}
