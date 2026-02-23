import React from 'react';

// Property types (externally-tagged serde enums from Rust)
export type TaskStatus = 'Todo' | 'Doing' | 'Done';
export type Deadline =
    | { Date: string }
    | { DateTime: string }
    | { Uninterpretable: string };

export type Property =
    | { Task: { status: TaskStatus; due: Deadline } }
    | { Anchor: { name: string } };

export function deadlineText(due: Deadline): string {
    if ('Date' in due) return due.Date;
    if ('DateTime' in due) return due.DateTime.replace('T', ' ');
    return due.Uninterpretable;
}

export function deadlineChipClass(due: Deadline): string {
    let dateStr: string | null = null;
    if ('Date' in due) dateStr = due.Date;
    else if ('DateTime' in due) dateStr = due.DateTime.split('T')[0];
    if (!dateStr) return 'bg-slate-100 text-slate-600';
    const diff = (new Date(dateStr).getTime() - Date.now()) / 86400000;
    if (diff < 0) return 'bg-red-100 text-red-700';
    if (diff <= 7) return 'bg-amber-100 text-amber-700';
    return 'bg-slate-100 text-slate-600';
}

const TaskIcon: React.FC<{ status: TaskStatus }> = ({ status }) => {
    if (status === 'Done') return <span className="mr-1 text-green-500 font-bold select-none">✓</span>;
    if (status === 'Doing') return <span className="mr-1 text-blue-500 select-none">◑</span>;
    return <span className="mr-1 text-slate-400 select-none">○</span>;
};

export default TaskIcon;
