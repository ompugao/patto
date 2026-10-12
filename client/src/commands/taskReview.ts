import * as vscode from 'vscode';
import type { PattoLsp } from '../languageClient';
import { groupByCompletedDate, timeSpentText, type CompletedTask } from '../tasks/taskLabel';

const DATE_PATTERN = /^\d{4}-\d{2}-\d{2}$/;

async function pickTimeframe(): Promise<string[] | undefined> {
	const choice = await vscode.window.showQuickPick(
		[
			{ label: "Today", value: "today" },
			{ label: "This Week (Mon–today)", value: "this_week" },
			{ label: "Custom date range…", value: "custom" },
		],
		{ placeHolder: "Select timeframe for completed task review" }
	);
	if (!choice) return undefined;
	if (choice.value !== "custom") return [choice.value];

	const from = await vscode.window.showInputBox({
		prompt: "From date (YYYY-MM-DD)",
		placeHolder: "e.g. 2024-03-01",
		validateInput: (v) => DATE_PATTERN.test(v) ? null : "Enter date as YYYY-MM-DD",
	});
	if (!from) return undefined;
	const to = await vscode.window.showInputBox({
		prompt: "To date (YYYY-MM-DD)",
		placeHolder: "e.g. 2024-03-31",
		validateInput: (v) => DATE_PATTERN.test(v) ? null : "Enter date as YYYY-MM-DD",
	});
	if (!to) return undefined;
	return ["custom", from, to];
}

function completedTaskItems(tasks: CompletedTask[]): vscode.QuickPickItem[] {
	const items: vscode.QuickPickItem[] = [];
	for (const [date, group] of groupByCompletedDate(tasks)) {
		items.push({ label: `📅 ${date}`, kind: vscode.QuickPickItemKind.Separator });
		for (const t of group) {
			const spent = timeSpentText(t.time_spent);
			const fsPath = vscode.Uri.parse(t.location.uri).fsPath;
			items.push({
				label: `  ✓ ${t.completed_at}  ${t.text}${spent ? ` ⏱ ${spent}` : ''}`,
				description: fsPath.split('/').pop(),
				detail: fsPath,
			});
		}
	}
	return items;
}

async function revealTask(selected: vscode.QuickPickItem, tasks: CompletedTask[]): Promise<void> {
	const doc = await vscode.workspace.openTextDocument(selected.detail!);
	const task = tasks.find(t =>
		vscode.Uri.parse(t.location.uri).fsPath === selected.detail &&
		selected.label.includes(t.text)
	);
	const line = task?.location?.range?.start?.line ?? 0;
	const editor = await vscode.window.showTextDocument(doc);
	editor.revealRange(new vscode.Range(line, 0, line, 0), vscode.TextEditorRevealType.InCenter);
	editor.selection = new vscode.Selection(line, 0, line, 0);
}

/** patto.taskReview: list the tasks completed in a chosen period and jump to one. */
export async function reviewCompletedTasks(lsp: PattoLsp, outputChannel: vscode.OutputChannel): Promise<void> {
	const timeframe = await pickTimeframe();
	if (!timeframe) return;

	try {
		const tasks = await lsp.tasksReview(timeframe);
		if (!tasks || tasks.length === 0) {
			vscode.window.showInformationMessage("No completed tasks found for the selected period");
			return;
		}

		const selected = await vscode.window.showQuickPick(completedTaskItems(tasks), {
			placeHolder: `${tasks.length} completed task(s)`,
			matchOnDescription: true,
			matchOnDetail: true,
		});
		if (selected && selected.detail) {
			await revealTask(selected, tasks);
		}
	} catch (error) {
		outputChannel.appendLine("[patto] Error in task review: " + error);
		vscode.window.showErrorMessage("Failed to retrieve completed tasks: " + error);
	}
}
