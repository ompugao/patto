import * as assert from 'assert';
import { groupByCompletedDate, taskLabel, timeSpentText, type CompletedTask, type TaskInformation } from '../tasks/taskLabel';

const location = { uri: 'file:///notes/a.pn', range: { start: { line: 1, character: 0 }, end: { line: 1, character: 4 } } };

function task(overrides: Partial<TaskInformation>): TaskInformation {
	return { location, text: 'write report', status: 'Todo', ...overrides };
}

describe('taskLabel', () => {
	it('shows a bare todo as its text', () => {
		assert.strictEqual(taskLabel(task({})), 'write report');
	});

	it('puts the due date first, as a date for both Date and DateTime deadlines', () => {
		assert.strictEqual(taskLabel(task({ due: { Date: '2024-03-01' } })), '[due:2024-03-01] write report');
		assert.strictEqual(taskLabel(task({ due: { DateTime: '2024-03-01T09:30:00' } })), '[due:2024-03-01] write report');
	});

	it('omits a deadline the server could not interpret', () => {
		assert.strictEqual(taskLabel(task({ due: { Uninterpretable: 'someday' } })), 'write report');
	});

	it('marks doing and paused tasks, but not todo or done', () => {
		assert.strictEqual(taskLabel(task({ status: 'Doing' })), 'write report [doing]');
		assert.strictEqual(taskLabel(task({ status: 'Paused' })), 'write report [paused]');
		assert.strictEqual(taskLabel(task({ status: 'Done' })), 'write report');
	});

	it('appends the time spent after the status', () => {
		assert.strictEqual(
			taskLabel(task({ status: 'Doing', due: { Date: '2024-03-01' }, time_spent: { hours: 1, minutes: 30 } })),
			'[due:2024-03-01] write report [doing] [1h30m]',
		);
	});
});

describe('timeSpentText', () => {
	it('drops the zero part and is empty for no time at all', () => {
		assert.strictEqual(timeSpentText({ hours: 2, minutes: 0 }), '2h');
		assert.strictEqual(timeSpentText({ hours: 0, minutes: 45 }), '45m');
		assert.strictEqual(timeSpentText({ hours: 0, minutes: 0 }), '');
		assert.strictEqual(timeSpentText(null), '');
	});
});

describe('groupByCompletedDate', () => {
	it('groups tasks by completion date in ascending date order', () => {
		const done = (text: string, completed_at: string): CompletedTask => ({ ...task({ text, status: 'Done' }), completed_at });
		const groups = groupByCompletedDate([done('b', '2024-03-02'), done('a', '2024-03-01'), done('c', '2024-03-02')]);
		assert.deepStrictEqual(
			groups.map(([date, tasks]) => [date, tasks.map(t => t.text)]),
			[['2024-03-01', ['a']], ['2024-03-02', ['b', 'c']]],
		);
	});
});
