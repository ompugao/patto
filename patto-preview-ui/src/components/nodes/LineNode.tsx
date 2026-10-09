import { nodeText } from '../../ast';
import { deadlineChipClass, deadlineText, taskProperty } from '../../tasks';
import { bulletMarker, useBulletStyle } from '../BulletStyle';
import TaskIcon from '../TaskIcon';
import RenderNode, { NodeList, type NodeProps } from '../RenderNode';

export default function LineNode({ node, onWikiLinkClick, depth }: NodeProps & { depth: number }) {
    const bulletStyle = useBulletStyle();
    const kind = node.value.kind;
    const contents = node.value.contents ?? [];
    const children = node.value.children ?? [];
    const isQuote = kind.type === 'QuoteContent';
    const text = nodeText(node);
    const isEmpty = contents.length === 0 && text.trim() === '' && children.length === 0;

    if (isEmpty) {
        return <div className="min-h-[1.5em]">&nbsp;</div>;
    }

    const task = taskProperty(kind.properties)?.Task ?? null;
    const taskStatus = task?.status ?? null;
    const isDone = taskStatus === 'Done';
    const due = task?.due ?? null;
    // A task's status icon already marks the line
    const marker = taskStatus ? null : bulletMarker(bulletStyle, depth);

    return (
        <div className={`leading-snug min-h-[1.5em]${isQuote ? ' text-slate-500' : ''}`} data-line={node.location.row}>
            {/* Use div instead of span so block-level content nodes (e.g. HorizontalLine) render correctly */}
            <div className="flex items-baseline gap-1 flex-wrap">
                {marker && <span className="text-slate-400 select-none w-3 shrink-0 text-center" aria-hidden>{marker}</span>}
                {taskStatus && <TaskIcon status={taskStatus} />}
                <div className={`flex-1 ${isDone ? 'line-through text-slate-400' : ''}`}>
                    {contents.length > 0
                        ? <NodeList nodes={contents} onWikiLinkClick={onWikiLinkClick} />
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
                <div className={`pl-5 border-l ${bulletStyle === 'guide' ? 'border-slate-100' : 'border-transparent'} ml-1 mt-[0.375em] space-y-[0.375em]`}>
                    {children.map((c, i) => <RenderNode key={i} node={c} onWikiLinkClick={onWikiLinkClick} depth={depth + 1} />)}
                </div>
            )}
        </div>
    );
}
