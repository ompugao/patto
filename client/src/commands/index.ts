import * as vscode from 'vscode';
import type { PattoLsp } from '../languageClient';
import type { TasksProvider } from '../tasks/tasksProvider';
import { copyAsMarkdown } from './copyAsMarkdown';
import { reviewCompletedTasks } from './taskReview';
import { showTwoHopLinks } from './twoHopLinks';

/** The commands in package.json `contributes.commands`, except patto.openPreview (see preview/). */
export function registerCommands(
	context: vscode.ExtensionContext,
	lsp: PattoLsp,
	tasksProvider: TasksProvider,
	outputChannel: vscode.OutputChannel,
): void {
	context.subscriptions.push(
		vscode.commands.registerCommand("patto.tasks", async () => {
			try {
				outputChannel.appendLine("[patto] Requesting tasks...");
				const tasks = await lsp.aggregateTasks();
				outputChannel.appendLine("[patto] Tasks response: " + JSON.stringify(tasks));
				if (!tasks || tasks.length === 0) {
					vscode.window.showInformationMessage('No tasks found in workspace');
					tasksProvider.refresh([]);
				} else {
					tasksProvider.refresh(tasks);
					vscode.window.showInformationMessage(`Found ${tasks.length} tasks`);
				}
			} catch (error) {
				outputChannel.appendLine("[patto] Error aggregating tasks: " + error);
				vscode.window.showErrorMessage('Failed to aggregate tasks: ' + error);
			}
		}),

		vscode.commands.registerCommand("patto.scanWorkspace", async () => {
			try {
				await lsp.scanWorkspace();
				vscode.window.showInformationMessage('Workspace scan initiated');
			} catch (error) {
				outputChannel.appendLine("[patto] Error scanning workspace: " + error);
			}
		}),

		vscode.commands.registerCommand("patto.snapshotPapers", async () => {
			try {
				await lsp.snapshotPapers();
				vscode.window.showInformationMessage('Snapshot papers initiated');
			} catch (error) {
				outputChannel.appendLine("[patto] Error snapshotting papers: " + error);
			}
		}),

		vscode.commands.registerCommand("patto.twoHopLinks", () => showTwoHopLinks(lsp, outputChannel)),
		vscode.commands.registerCommand("patto.taskReview", () => reviewCompletedTasks(lsp, outputChannel)),
		vscode.commands.registerCommand("patto.copyAsMarkdown", () => copyAsMarkdown(lsp, outputChannel)),
	);
}
