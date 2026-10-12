import type { CSSProperties } from 'react';
import { nodeText } from '../../ast';
import ImageLightbox from '../ImageLightbox';
import { NodeList, type NodeProps } from '../RenderNode';

export function DecorationNode({ node, onWikiLinkClick }: NodeProps) {
    const kind = node.value.kind;
    const fs = kind.fontsize ?? 0;
    const style: CSSProperties = { fontSize: `${100 + Math.max(fs - 1, 0) * 20}%` };
    if (fs > 0) style.fontWeight = 'bold';
    let cls = '';
    if (kind.italic) cls += ' italic';
    if (kind.underline) cls += ' underline';
    if (kind.deleted) cls += ' line-through text-slate-400';
    return (
        <span className={cls.trim()} style={style}>
            <NodeList nodes={node.value.contents ?? []} onWikiLinkClick={onWikiLinkClick} />
        </span>
    );
}

export function TextNode({ node, onWikiLinkClick }: NodeProps) {
    const contents = node.value.contents ?? [];
    if (contents.length > 0) {
        return <NodeList nodes={contents} onWikiLinkClick={onWikiLinkClick} />;
    }
    return <span className="whitespace-pre-wrap">{nodeText(node)}</span>;
}

export function WikiLinkNode({ node, onWikiLinkClick }: NodeProps) {
    const { link, anchor } = node.value.kind;
    return (
        <a
            href="#"
            onClick={e => { e.preventDefault(); onWikiLinkClick(link!, anchor ?? undefined); }}
            className="text-blue-600 hover:text-blue-800 hover:underline cursor-pointer font-medium"
        >
            {link}{anchor ? `#${anchor}` : ''}
        </a>
    );
}

export function LinkNode({ node }: NodeProps) {
    const { link, title } = node.value.kind;
    return (
        <a href={link} target="_blank" rel="noopener noreferrer" className="text-blue-500 hover:underline break-all">
            {title || link}
        </a>
    );
}

export function ImageNode({ node }: NodeProps) {
    let src = node.value.kind.src ?? '';
    if (src && !src.startsWith('http') && !src.startsWith('data:')) {
        src = `/api/files/${encodeURIComponent(src)}`;
    }
    return <ImageLightbox src={src} alt={node.value.kind.alt || undefined} />;
}
