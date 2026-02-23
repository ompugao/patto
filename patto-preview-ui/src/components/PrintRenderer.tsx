import { useMemo } from 'react';
import { RenderNode, flattenAst, type AstNode } from './VirtualRenderer';

interface PrintRendererProps {
    ast: AstNode | null;
    onWikiLinkClick: (link: string, anchor?: string) => void;
}

/**
 * Renders the full AST without virtual scrolling.
 * Used for printing — the VirtualRenderer only renders visible rows,
 * so we need this to get the complete document in the print output.
 */
export default function PrintRenderer({ ast, onWikiLinkClick }: PrintRendererProps) {
    const blocks = useMemo(() => {
        if (!ast) return [];
        return flattenAst(ast);
    }, [ast]);

    if (!ast || blocks.length === 0) return null;

    return (
        <div className="print-renderer">
            {blocks.map((block, i) => (
                <div key={i} className="w-full px-8 py-0.5">
                    <RenderNode node={block} onWikiLinkClick={onWikiLinkClick} />
                </div>
            ))}
        </div>
    );
}
