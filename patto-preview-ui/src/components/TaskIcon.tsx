import type { TaskStatus } from '../tasks';

export default function TaskIcon({ status }: { status: TaskStatus }) {
    if (status === 'Done') return <span className="mr-1 text-green-500 font-bold select-none">✓</span>;
    if (status === 'Doing') return <span className="mr-1 text-blue-500 select-none">◑</span>;
    return <span className="mr-1 text-slate-400 select-none">○</span>;
}
