import type { AstNode } from '../ast';
import EmbedBlock from './EmbedBlock';
import LineNode from './nodes/LineNode';
import { CodeNode, MathNode, QuoteNode, TableNode } from './nodes/BlockNodes';
import { DecorationNode, ImageNode, LinkNode, TextNode, WikiLinkNode } from './nodes/InlineNodes';

export type WikiLinkHandler = (link: string, anchor?: string) => void;

export interface NodeProps {
    node: AstNode;
    onWikiLinkClick: WikiLinkHandler;
}

export function NodeList({ nodes, onWikiLinkClick }: { nodes: AstNode[]; onWikiLinkClick: WikiLinkHandler }) {
    return (
        <>
            {nodes.map((n, i) => <RenderNode key={i} node={n} onWikiLinkClick={onWikiLinkClick} />)}
        </>
    );
}

export default function RenderNode({ node, onWikiLinkClick, depth = 0 }: NodeProps & {
    /** Nesting level of a line; 0 for top-level lines. */
    depth?: number;
}) {
    const kind = node.value?.kind;
    if (!kind) return null;

    switch (kind.type) {
        case 'Line':
        case 'QuoteContent':
            return <LineNode node={node} onWikiLinkClick={onWikiLinkClick} depth={depth} />;
        case 'Quote':
            return <QuoteNode node={node} onWikiLinkClick={onWikiLinkClick} />;
        case 'Code':
            return <CodeNode node={node} onWikiLinkClick={onWikiLinkClick} />;
        case 'Math':
            return <MathNode node={node} onWikiLinkClick={onWikiLinkClick} />;
        case 'Image':
            return <ImageNode node={node} onWikiLinkClick={onWikiLinkClick} />;
        case 'WikiLink':
            return <WikiLinkNode node={node} onWikiLinkClick={onWikiLinkClick} />;
        case 'Link':
            return <LinkNode node={node} onWikiLinkClick={onWikiLinkClick} />;
        case 'Embed':
            return <EmbedBlock link={kind.link!} title={kind.title ?? null} />;
        case 'Decoration':
            return <DecorationNode node={node} onWikiLinkClick={onWikiLinkClick} />;
        case 'Text':
            return <TextNode node={node} onWikiLinkClick={onWikiLinkClick} />;
        case 'HorizontalLine':
            return <hr className="w-full border-0 border-t border-slate-300 my-1" />;
        case 'Table':
            return <TableNode node={node} onWikiLinkClick={onWikiLinkClick} />;
        case 'Dummy':
            return <NodeList nodes={node.value.children ?? []} onWikiLinkClick={onWikiLinkClick} />;
        default:
            return null;
    }
}
