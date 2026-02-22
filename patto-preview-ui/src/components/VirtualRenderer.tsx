import React, { useMemo, useRef } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import EmbedBlock from './EmbedBlock';
import { MathJax } from 'better-react-mathjax';
import CodeBlock from './CodeBlock';
import 'highlight.js/styles/github.min.css';

// Matches the actual JSON shape from the Rust backend:
// AstNode is #[serde(transparent)] -> Annotation<AstNodeInternal>
// So: { value: { contents, children, kind, stable_id }, location: { row, span, input } }

export interface AstNodeKind {
    type: string;
    // Line / QuoteContent
    properties?: any[];
    // Math / Code
    inline?: boolean;
    lang?: string;
    // Table
    caption?: string | null;
    // Image
    src?: string;
    alt?: string | null;
    // WikiLink
    link?: string;
    anchor?: string | null;
    // Link / Embed
    title?: string | null;
    // Decoration
    fontsize?: number;
    italic?: boolean;
    underline?: boolean;
    deleted?: boolean;
}

export interface AstNode {
    location: {
        row: number;
        span: [number, number];
        input: string;
    };
    value: {
        kind: AstNodeKind;
        contents: AstNode[];
        children: AstNode[];
        stable_id: number | null;
    };
}

interface VirtualRendererProps {
    ast: AstNode | null;
    onWikiLinkClick: (link: string, anchor?: string) => void;
}

// Rust generates span offsets as raw UTF-8 byte offsets (not JS UTF-16 characters)
// To extract the correct substring (especially necessary for Japanese CJK chars),
// we must convert the string to a UTF-8 ArrayBuffer, slice the byte range, and decode.
const decoder = new TextDecoder();
const encoder = new TextEncoder();

/** Extract raw text slice for a node */
function nodeText(node: AstNode): string {
    if (!node.location.span) return '';
    const [start, end] = node.location.span;
    const bytes = encoder.encode(node.location.input);
    return decoder.decode(bytes.slice(start, end));
}

const InlineContents: React.FC<{ nodes: AstNode[]; onWikiLinkClick: (l: string, a?: string) => void }> = ({ nodes, onWikiLinkClick }) => (
    <>
        {nodes.map((n, i) => <RenderNode key={i} node={n} onWikiLinkClick={onWikiLinkClick} />)}
    </>
);

const RenderNode: React.FC<{ node: AstNode; onWikiLinkClick: (l: string, a?: string) => void }> = ({ node, onWikiLinkClick }) => {
    const kind = node.value?.kind;
    const contents = node.value?.contents ?? [];
    const children = node.value?.children ?? [];

    if (!kind) return null;

    switch (kind.type) {
        case 'Line':
        case 'QuoteContent': {
            const isQuote = kind.type === 'QuoteContent';
            const inner = (
                <div className={`leading-relaxed min-h-[1.5rem]${isQuote ? ' text-slate-600 italic' : ''}`} data-line={node.location.row}>
                    {contents.length > 0
                        ? <InlineContents nodes={contents} onWikiLinkClick={onWikiLinkClick} />
                        : <span className="whitespace-pre-wrap">{nodeText(node)}</span>
                    }
                    {children.length > 0 && (
                        <div className="pl-5 border-l-2 border-slate-200 ml-1 mt-0.5">
                            {children.map((c, i) => <RenderNode key={i} node={c} onWikiLinkClick={onWikiLinkClick} />)}
                        </div>
                    )}
                </div>
            );
            return isQuote
                ? <blockquote className="border-l-4 border-slate-300 pl-3 my-1">{inner}</blockquote>
                : inner;
        }

        case 'Quote': {
            return (
                <blockquote className="border-l-4 border-slate-300 pl-3 my-1 text-slate-600 italic">
                    {children.map((c, i) => <RenderNode key={i} node={c} onWikiLinkClick={onWikiLinkClick} />)}
                </blockquote>
            );
        }

        case 'Code': {
            if (kind.inline) {
                const innerText = contents.length > 0 ? contents.map(c => nodeText(c)).join('') : nodeText(node);
                return <CodeBlock code={innerText} inline />;
            }
            const lines = children.map(c => nodeText(c)).join('\n');
            return <CodeBlock code={lines} language={kind.lang} />;
        }

        case 'Math': {
            if (kind.inline) {
                const innerText = contents.length > 0 ? contents.map(c => nodeText(c)).join('') : nodeText(node);
                return (
                    <MathJax inline dynamic className="bg-amber-50 text-amber-800 px-1 rounded text-sm font-mono inline-block">
                        {`\\(${innerText}\\)`}
                    </MathJax>
                );
            }
            const mathLines = children.map(c => nodeText(c)).join('\n');
            const blockContent = mathLines || nodeText(node);
            return (
                <MathJax dynamic className="bg-amber-50 border border-amber-200 text-amber-900 p-3 rounded-lg my-3 font-mono text-sm overflow-x-auto">
                    {`$$ \n ${blockContent} \n $$`}
                </MathJax>
            );
        }

        case 'Image': {
            let src = kind.src ?? '';
            if (src && !src.startsWith('http') && !src.startsWith('data:')) {
                src = `/api/files/${encodeURIComponent(src)}`;
            }
            return (
                <figure className="my-3">
                    <img src={src} alt={kind.alt || ''} className="max-w-full rounded-lg shadow-sm" loading="lazy" />
                    {kind.alt && <figcaption className="text-xs text-slate-500 mt-1 text-center">{kind.alt}</figcaption>}
                </figure>
            );
        }

        case 'WikiLink': {
            return (
                <a
                    href="#"
                    onClick={e => { e.preventDefault(); onWikiLinkClick(kind.link!, kind.anchor ?? undefined); }}
                    className="text-blue-600 hover:text-blue-800 hover:underline cursor-pointer font-medium"
                >
                    {kind.link}{kind.anchor ? `#${kind.anchor}` : ''}
                </a>
            );
        }

        case 'Link': {
            return (
                <a href={kind.link} target="_blank" rel="noopener noreferrer" className="text-blue-500 hover:underline break-all">
                    {kind.title || kind.link}
                </a>
            );
        }

        case 'Embed': {
            return <EmbedBlock link={kind.link!} title={kind.title ?? null} />;
        }

        case 'Decoration': {
            const style: React.CSSProperties = {};
            if (kind.fontsize && kind.fontsize !== 1) style.fontSize = `${kind.fontsize}em`;
            let cls = '';
            if (kind.italic) cls += ' italic';
            if (kind.underline) cls += ' underline';
            if (kind.deleted) cls += ' line-through text-slate-400';
            return (
                <span className={cls.trim()} style={style}>
                    <InlineContents nodes={contents} onWikiLinkClick={onWikiLinkClick} />
                </span>
            );
        }

        case 'Text': {
            if (contents.length > 0) {
                return <InlineContents nodes={contents} onWikiLinkClick={onWikiLinkClick} />;
            }
            return <span className="whitespace-pre-wrap">{nodeText(node)}</span>;
        }

        case 'HorizontalLine': {
            return <hr className="my-4 border-slate-200" />;
        }

        case 'Table': {
            return (
                <div className="overflow-x-auto my-3">
                    {kind.caption && <p className="text-sm text-slate-500 mb-1">{kind.caption}</p>}
                    <table className="border-collapse w-full text-sm">
                        <tbody>
                            {children.map((row, i) => (
                                <tr key={i} className={i % 2 === 0 ? 'bg-white' : 'bg-slate-50'}>
                                    {(row.value?.children ?? []).map((col, j) => (
                                        <td key={j} className="border border-slate-200 px-3 py-1.5">
                                            <InlineContents nodes={col.value?.contents ?? []} onWikiLinkClick={onWikiLinkClick} />
                                        </td>
                                    ))}
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            );
        }

        case 'Dummy': {
            return (
                <>
                    {children.map((c, i) => <RenderNode key={i} node={c} onWikiLinkClick={onWikiLinkClick} />)}
                </>
            );
        }

        default:
            return null;
    }
};

function flattenAst(node: AstNode): AstNode[] {
    if (!node) return [];
    if (node.value?.kind?.type === 'Dummy') {
        return node.value.children ?? [];
    }
    return [node];
}

export default function VirtualRenderer({ ast, onWikiLinkClick }: VirtualRendererProps) {
    const parentRef = useRef<HTMLDivElement>(null);

    const blocks = useMemo(() => {
        if (!ast) return [];
        return flattenAst(ast);
    }, [ast]);

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
                {rowVirtualizer.getVirtualItems().map(virtualItem => {
                    const block = blocks[virtualItem.index];
                    return (
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
                            className="w-full px-8 py-0.5"
                        >
                            <RenderNode node={block} onWikiLinkClick={onWikiLinkClick} />
                        </div>
                    );
                })}
            </div>
        </div>
    );
}
