import React, { useMemo, useRef } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import EmbedBlock from './EmbedBlock';
import { MathJax } from 'better-react-mathjax';
import CodeBlock from './CodeBlock';
import ImageLightbox from './ImageLightbox';
import TaskIcon, { type Property, type TaskStatus, type Deadline, deadlineText, deadlineChipClass } from './TaskIcon';

// Matches the actual JSON shape from the Rust backend:
// AstNode is #[serde(transparent)] -> Annotation<AstNodeInternal>
// So: { value: { contents, children, kind, stable_id }, location: { row, span, input } }

export interface AstNodeKind {
    type: string;
    // Line / QuoteContent
    properties?: Property[];
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

export const RenderNode: React.FC<{ node: AstNode; onWikiLinkClick: (l: string, a?: string) => void }> = ({ node, onWikiLinkClick }) => {
    const kind = node.value?.kind;
    const contents = node.value?.contents ?? [];
    const children = node.value?.children ?? [];

    if (!kind) return null;

    switch (kind.type) {
        case 'Line':
        case 'QuoteContent': {
            const isQuote = kind.type === 'QuoteContent';
            const text = nodeText(node);
            const isEmpty = contents.length === 0 && text.trim() === '' && children.length === 0;

            if (isEmpty) {
                return <div className="min-h-[1.5rem]">&nbsp;</div>;
            }

            // Extract task property (first Task found in properties)
            const taskProp = (kind.properties ?? []).find((p): p is { Task: { status: TaskStatus; due: Deadline } } => 'Task' in p);
            const taskStatus = taskProp?.Task.status ?? null;
            const isDone = taskStatus === 'Done';
            const due = taskProp?.Task.due ?? null;

            const inner = (
                <div className={`leading-relaxed min-h-[1.5rem]${isQuote ? ' text-slate-500' : ''}`} data-line={node.location.row}>
                    {/* Use div instead of span so block-level content nodes (e.g. HorizontalLine) render correctly */}
                    <div className="flex items-baseline gap-1 flex-wrap">
                        {taskStatus && <TaskIcon status={taskStatus} />}
                        <div className={`flex-1 ${isDone ? 'line-through text-slate-400' : ''}`}>
                            {contents.length > 0
                                ? <InlineContents nodes={contents} onWikiLinkClick={onWikiLinkClick} />
                                : <span className="whitespace-pre-wrap">{text}</span>
                            }
                        </div>
                        {due && !isDone && (
                            <span className={`text-xs px-1.5 py-0.5 rounded-full font-medium ${deadlineChipClass(due)}`}>
                                {deadlineText(due)}
                            </span>
                        )}
                    </div>
                    {children.length > 0 && (
                        <div className="pl-5 border-l border-slate-100 ml-1 mt-0.5">
                            {children.map((c, i) => <RenderNode key={i} node={c} onWikiLinkClick={onWikiLinkClick} />)}
                        </div>
                    )}
                </div>
            );
            return isQuote
                ? <blockquote className="border-l-3 border-slate-200 pl-3 my-1 bg-slate-50/50 py-1 rounded-r">{inner}</blockquote>
                : inner;
        }

        case 'Quote': {
            return (
                <blockquote className="border-l-3 border-slate-200 pl-3 my-1 text-slate-500 bg-slate-50/50 py-1 rounded-r">
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
            return <ImageLightbox src={src} alt={kind.alt || undefined} />;
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
            const fs = kind.fontsize ?? 0;
            const style: React.CSSProperties = { fontSize: `${100 + fs * 20}%` };
            if (fs > 0) style.fontWeight = 'bold';
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
            return <hr className="w-full border-0 border-t border-slate-300 my-1" />;
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
                </div >
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

export function flattenAst(node: AstNode): AstNode[] {
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
