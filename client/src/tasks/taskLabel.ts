// The task JSON patto-lsp returns from aggregate_tasks and tasks_review
// (TaskInformation in src/lsp/backend.rs; externally tagged serde enums).

export type TaskStatus = 'Todo' | 'Doing' | 'Paused' | 'Done';

export type Deadline =
	| { Date: string }
	| { DateTime: string }
	| { Uninterpretable: string };

export interface TimeSpent {
	hours: number;
	minutes: number;
}

export interface TaskLocation {
	uri: string;
	range: {
		start: { line: number; character: number };
		end: { line: number; character: number };
	};
}

export interface TaskInformation {
	location: TaskLocation;
	text: string;
	status: TaskStatus;
	due?: Deadline | null;
	time_spent?: TimeSpent | null;
}

export interface CompletedTask extends TaskInformation {
	/** YYYY-MM-DD */
	completed_at: string;
}

export function dueDate(due: Deadline | null | undefined): string {
	if (!due || typeof due !== 'object') return '';
	if ('Date' in due) return due.Date;
	if ('DateTime' in due) return due.DateTime.slice(0, 10);
	return '';
}

export function timeSpentText(timeSpent: TimeSpent | null | undefined): string {
	if (!timeSpent || typeof timeSpent !== 'object') return '';
	const h = timeSpent.hours ?? 0;
	const m = timeSpent.minutes ?? 0;
	if (h > 0 && m > 0) return `${h}h${m}m`;
	if (h > 0) return `${h}h`;
	if (m > 0) return `${m}m`;
	return '';
}

/** The tree item label: `[due:2024-03-01] text [doing] [1h30m]`, omitting absent parts. */
export function taskLabel(task: TaskInformation): string {
	const parts: string[] = [];

	const due = dueDate(task.due);
	if (due) parts.push(`[due:${due}]`);

	parts.push(task.text);

	if (task.status === 'Doing') parts.push('[doing]');
	if (task.status === 'Paused') parts.push('[paused]');

	const spent = timeSpentText(task.time_spent);
	if (spent) parts.push(`[${spent}]`);

	return parts.join(' ');
}

/** Completed tasks grouped by completion date, dates ascending. */
export function groupByCompletedDate(tasks: CompletedTask[]): [string, CompletedTask[]][] {
	const byDate = new Map<string, CompletedTask[]>();
	for (const task of tasks) {
		const group = byDate.get(task.completed_at);
		if (group) group.push(task);
		else byDate.set(task.completed_at, [task]);
	}
	return [...byDate.entries()].sort();
}
