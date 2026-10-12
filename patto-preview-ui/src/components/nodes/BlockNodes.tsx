import { MathJax } from 'better-react-mathjax';
import { joinedText, nodeText } from '../../ast';
import CodeBlock from '../CodeBlock';
import { NodeList, type NodeProps } from '../RenderNode';

export function QuoteNode({ node, onWikiLinkClick }: NodeProps) {
    return (
        <blockquote className="border-l-3 border-slate-200 pl-3 my-1 text-slate-500 bg-slate-50/50 py-1 rounded-r">
            <NodeList nodes={node.value.children ?? []} onWikiLinkClick={onWikiLinkClick} />
        </blockquote>
    );
}

function inlineText(node: NodeProps['node']): string {
    const contents = node.value.contents ?? [];
    return contents.length > 0 ? joinedText(contents, '') : nodeText(node);
}

export function CodeNode({ node }: NodeProps) {
    if (node.value.kind.inline) {
        return <CodeBlock code={inlineText(node)} inline />;
    }
    return <CodeBlock code={joinedText(node.value.children ?? [], '\n')} language={node.value.kind.lang} />;
}

export function MathNode({ node }: NodeProps) {
    if (node.value.kind.inline) {
        return (
            <MathJax inline dynamic className="bg-amber-50 text-amber-800 px-1 rounded text-sm font-mono inline-block">
                {`\\(${inlineText(node)}\\)`}
            </MathJax>
        );
    }
    const blockContent = joinedText(node.value.children ?? [], '\n') || nodeText(node);
    return (
        <MathJax dynamic className="bg-amber-50 border border-amber-200 text-amber-900 p-3 rounded-lg my-3 font-mono text-sm overflow-x-auto">
            {`$$ \n ${blockContent} \n $$`}
        </MathJax>
    );
}

export function TableNode({ node, onWikiLinkClick }: NodeProps) {
    const { caption } = node.value.kind;
    return (
        <div className="overflow-x-auto my-3">
            {caption && <p className="text-sm text-slate-500 mb-1">{caption}</p>}
            <table className="border-collapse w-full text-sm">
                <tbody>
                    {(node.value.children ?? []).map((row, i) => (
                        <tr key={i} className={i % 2 === 0 ? 'bg-white' : 'bg-slate-50'}>
                            {(row.value?.contents ?? []).map((col, j) => (
                                <td key={j} className="border border-slate-200 px-3 py-1.5">
                                    <NodeList nodes={col.value?.contents ?? []} onWikiLinkClick={onWikiLinkClick} />
                                </td>
                            ))}
                        </tr>
                    ))}
                </tbody>
            </table>
        </div>
    );
}
