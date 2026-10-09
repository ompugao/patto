import * as vscode from 'vscode';
import type { PattoLsp } from '../languageClient';
import { TasksProvider } from './tasksProvider';

const INITIAL_LOAD_DELAY_MS = 2000;
const TYPING_DEBOUNCE_MS = 1000;

/** The "Patto Note Tasks" explorer view, refreshed as notes are edited. */
export function registerTasksView(
	context: vscode.ExtensionContext,
	lsp: PattoLsp,
	outputChannel: vscode.OutputChannel,
): TasksProvider {
	const tasksProvider = new TasksProvider();
	context.subscriptions.push(vscode.window.createTreeView('pattoTasks', {
		treeDataProvider: tasksProvider
	}));

	// Wait for the server to initialize before the first request
	setTimeout(async () => {
		outputChannel.appendLine("[patto-lsp] Attempting to auto-load tasks");
		try {
			const tasks = await lsp.aggregateTasks();
			if (tasks && Array.isArray(tasks) && tasks.length > 0) {
				tasksProvider.refresh(tasks);
				outputChannel.appendLine(`[patto] Auto-loaded ${tasks.length} tasks`);
			}
		} catch (error) {
			outputChannel.appendLine("[patto] Error auto-loading tasks: " + error);
		}
	}, INITIAL_LOAD_DELAY_MS);

	const refreshTasks = async () => {
		try {
			const tasks = await lsp.aggregateTasks();
			if (tasks && Array.isArray(tasks)) {
				tasksProvider.refresh(tasks);
			}
		} catch (error) {
			outputChannel.appendLine("[patto] Error refreshing tasks: " + error);
		}
	};

	let typingTimeout: NodeJS.Timeout | null = null;
	const refreshAfterTyping = () => {
		if (typingTimeout) {
			clearTimeout(typingTimeout);
		}
		typingTimeout = setTimeout(refreshTasks, TYPING_DEBOUNCE_MS);
	};

	context.subscriptions.push(
		vscode.workspace.onDidSaveTextDocument(async (document) => {
			if (document.languageId === 'patto') {
				await refreshTasks();
			}
		}),
		vscode.workspace.onDidChangeTextDocument((event) => {
			if (event.document.languageId === 'patto') {
				refreshAfterTyping();
			}
		}),
		vscode.window.onDidChangeActiveTextEditor(async (editor) => {
			if (editor && editor.document.languageId === 'patto') {
				await refreshTasks();
			}
		})
	);

	return tasksProvider;
}
