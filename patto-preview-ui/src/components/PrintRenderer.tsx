import { useMemo } from 'react';
import { flattenAst, type AstNode } from '../ast';
import RenderNode, { type WikiLinkHandler } from './RenderNode';

interface PrintRendererProps {
    ast: AstNode | null;
    onWikiLinkClick: WikiLinkHandler;
}

/**
 * Renders the full AST without virtual scrolling.
 * Used for printing — the VirtualRenderer only renders visible rows,
 * so we need this to get the complete document in the print output.
 */
export default function PrintRenderer({ ast, onWikiLinkClick }: PrintRendererProps) {
    const blocks = useMemo(() => (ast ? flattenAst(ast) : []), [ast]);

    if (!ast || blocks.length === 0) return null;

    return (
        <div className="print-renderer">
            {blocks.map((block, i) => (
                <div key={i} className="w-full px-8 py-[0.1875em]">
                    <RenderNode node={block} onWikiLinkClick={onWikiLinkClick} />
                </div>
            ))}
        </div>
    );
}
