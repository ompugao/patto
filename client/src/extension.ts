/* --------------------------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 * ------------------------------------------------------------------------------------------ */

import { window, workspace, ExtensionContext, OutputChannel, commands, Uri, ViewColumn } from 'vscode';
import * as vscode from 'vscode';

import {
	Executable,
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
	ExecuteCommandRequest,
	State,
} from 'vscode-languageclient/node';

import * as fs from 'fs';
import * as path from 'path';
import * as net from 'net';
import { BinaryManager } from './binaryManager';

let client: LanguageClient;
let previewPort: number | null = null;
let previewPanel: vscode.WebviewPanel | null = null;
let taskRefreshTimeout: NodeJS.Timeout | null = null;
let previewLspClient: LanguageClient | null = null;

// Helper function to find available port
async function findAvailablePort(startPort: number = 3000, maxAttempts: number = 100): Promise<number | null> {
	for (let i = 0; i < maxAttempts; i++) {
		const port = startPort + i;
		const isAvailable = await checkPortAvailable(port);
		if (isAvailable) {
			return port;
		}
	}
	return null;
}

function checkPortAvailable(port: number): Promise<boolean> {
	return new Promise((resolve) => {
		const server = net.createServer();
		server.once('error', () => resolve(false));
		server.once('listening', () => {
			server.close();
			resolve(true);
		});
		server.listen(port);
	});
}

// Get patto configuration for LSP server
function getPattoConfiguration() {
	const config = workspace.getConfiguration('patto');
	return {
		markdown: {
			defaultFlavor: config.get<string>('markdown.defaultFlavor', 'standard')
		}
	};
}


// Launch preview server
async function launchPreviewServer(rootPath: string, outputChannel: OutputChannel, command: string): Promise<number | null> {
	const port = await findAvailablePort(3000);
	if (!port) {
		vscode.window.showErrorMessage('Could not find an available port for preview server');
		return null;
	}

	if (previewLspClient) {
		await previewLspClient.stop().catch(() => undefined);
		previewLspClient = null;
	}

	const serverExecutable: Executable = {
		command,
		args: ['--port', port.toString(), '--preview-lsp-stdio'],
		options: {
			cwd: rootPath,
		},
	};

	const serverOptions: ServerOptions = serverExecutable;
	const clientOptions: LanguageClientOptions = {
		documentSelector: [{ language: 'patto' }],
	};

	previewLspClient = new LanguageClient('pattoPreview', 'Patto Preview', serverOptions, clientOptions);

	const clientRef = previewLspClient;
	clientRef.onDidChangeState((event) => {
		if (event.newState === State.Stopped && previewLspClient === clientRef) {
			previewLspClient = null;
			previewPort = null;
			outputChannel.appendLine('[patto] Preview server stopped');
		}
	});

	try {
		await previewLspClient.start();
		outputChannel.appendLine(`[patto] Launching preview server on port ${port} with command: ${command}`);
	} catch (error) {
		previewLspClient = null;
		const message = `Failed to launch patto-preview: ${error}`;
		outputChannel.appendLine(`[patto] ${message}`);
		vscode.window.showErrorMessage(message);
		return null;
	}

	previewPort = port;
	return port;
}

// Stop preview server
function stopPreviewServer() {
	if (previewLspClient) {
		previewLspClient.stop().catch(() => undefined);
		previewLspClient = null;
	}
	previewPort = null;
}

/** Build a rich display label for a task from its structured fields. */
function taskLabel(task: any): string {
	const parts: string[] = [task.text as string];

	// due date chip
	const due = task.due;
	if (due && typeof due === 'object') {
		const dueStr: string = due.Date ?? (due.DateTime ? (due.DateTime as string).slice(0, 10) : '');
		if (dueStr) parts.push(`[due:${dueStr}]`);
	}

	// status chip (only show non-todo)
	if (task.status === 'Doing') parts.push('[doing]');

	// time_spent chip
	const ts = task.time_spent;
	if (ts && typeof ts === 'object') {
		const h: number = ts.hours ?? 0;
		const m: number = ts.minutes ?? 0;
		if (h > 0 && m > 0) parts.push(`[${h}h${m}m]`);
		else if (h > 0)     parts.push(`[${h}h]`);
		else if (m > 0)     parts.push(`[${m}m]`);
	}

	return parts.join(' ');
}

export class TasksProvider implements vscode.TreeDataProvider<Task> {
	private tasks: Task[];
	constructor() {
		this.tasks = [];
	}

	getTreeItem(element: Task): vscode.TreeItem {
		return element;
	}

	getChildren(element?: Task): Thenable<Task[]> {
		if (element) {
			return Promise.resolve([]);
		} else {
			return Promise.resolve(this.tasks);
		}
	}

	private _onDidChangeTreeData: vscode.EventEmitter<Task | undefined | null | void> = new vscode.EventEmitter<Task | undefined | null | void>();
	readonly onDidChangeTreeData: vscode.Event<Task | undefined | null | void> = this._onDidChangeTreeData.event;

	refresh(result: any[]): void {
		this.tasks = [];
		for (let i = 0; i < result.length; ++i) {
			this.tasks.push(new Task(
				taskLabel(result[i]),
				result[i]['location'],
				vscode.TreeItemCollapsibleState.None
			));
		}
		this._onDidChangeTreeData.fire();
	}
}

class Task extends vscode.TreeItem {
	constructor(
		public readonly label: string,
		public readonly location: any,
		public readonly collapsibleState: vscode.TreeItemCollapsibleState
	) {
		super(label, collapsibleState);
		this.tooltip = `${this.label}`;
		//this.description = this.label;

		// Make tasks clickable
		if (location && location.uri) {
			this.command = {
				command: 'vscode.open',
				title: 'Open Task',
				arguments: [
					Uri.parse(location.uri),
					{
						selection: new vscode.Range(
							location.range.start.line,
							location.range.start.character,
							location.range.end.line,
							location.range.end.character
						)
					}
				]
			};
		}
	}
}


export function activate(context: ExtensionContext): void {
	const traceOutputChannel: OutputChannel = window.createOutputChannel("Patto Language Server");
	const binaryManager = new BinaryManager(context, traceOutputChannel);

	// Get binary path from configuration
	const config = vscode.workspace.getConfiguration('patto');
	const configuredLspPath = config.get<string>('lspPath');

	// Ensure LSP binary is available
	binaryManager.ensureBinary('patto-lsp', configuredLspPath !== 'patto-lsp' ? configuredLspPath : undefined)
		.then((command) => {
			if (!command) {
				traceOutputChannel.appendLine("[patto-lsp] Binary not available, extension will not activate");
				return;
			}

			traceOutputChannel.appendLine(`[patto-lsp] Using binary: ${command}`);
			startLanguageClient(context, command, traceOutputChannel, binaryManager);
		});
}

function startLanguageClient(
	context: ExtensionContext,
	command: string,
	traceOutputChannel: OutputChannel,
	binaryManager: BinaryManager
): void {

	const run: Executable = {
		command,
		options: {
			env: {
				...process.env,
				RUST_LOG: "info",
			},
		},
	};
	const serverOptions: ServerOptions = {
		run,
		debug: run,
	};

	const clientOptions: LanguageClientOptions = {
		documentSelector: [
			{ scheme: "file", language: "patto" },
			{ scheme: "untitled", language: "patto" },
		],
		synchronize: {
			fileEvents: workspace.createFileSystemWatcher('**/*.pn'),
			configurationSection: 'patto'
		},
		outputChannel: traceOutputChannel,
		initializationOptions: getPattoConfiguration()
	};

	// Create the language client and start the client.
	client = new LanguageClient(
		'patto-language-server',
		'Patto Language Server',
		serverOptions,
		clientOptions
	);

	const tasksProvider = new TasksProvider();
	const tasksTreeView = window.createTreeView('pattoTasks', {
		treeDataProvider: tasksProvider
	});
	context.subscriptions.push(tasksTreeView);

	// Start the LSP client
	client.start();

	// Wait for client initialization, then auto-aggregate tasks
	setTimeout(async () => {
		traceOutputChannel.appendLine("[patto-lsp] Attempting to auto-load tasks");

		// Auto-aggregate tasks on startup
		try {
			const response = await client.sendRequest(ExecuteCommandRequest.type, {
				command: "experimental/aggregate_tasks",
				arguments: [],
			});
			if (response && Array.isArray(response) && response.length > 0) {
				tasksProvider.refresh(response as any[]);
				traceOutputChannel.appendLine(`[patto] Auto-loaded ${response.length} tasks`);
			}
		} catch (error) {
			traceOutputChannel.appendLine("[patto] Error auto-loading tasks: " + error);
		}
	}, 2000); // Wait 2 seconds for LSP to initialize

	// Also refresh tasks when files change
	const refreshTasks = async () => {
		try {
			const response = await client.sendRequest(ExecuteCommandRequest.type, {
				command: "experimental/aggregate_tasks",
				arguments: [],
			});
			if (response && Array.isArray(response)) {
				tasksProvider.refresh(response as any[]);
			}
		} catch (error) {
			traceOutputChannel.appendLine("[patto] Error refreshing tasks: " + error);
		}
	};

	// Debounced refresh for typing events
	const debouncedRefreshTasks = () => {
		if (taskRefreshTimeout) {
			clearTimeout(taskRefreshTimeout);
		}
		taskRefreshTimeout = setTimeout(refreshTasks, 1000); // Wait 1 second after typing stops
	};

	// Refresh tasks when .pn files are saved
	context.subscriptions.push(
		vscode.workspace.onDidSaveTextDocument(async (document) => {
			if (document.languageId === 'patto') {
				await refreshTasks();
			}
		})
	);

	// Refresh tasks when .pn files are changed (debounced to avoid too many requests)
	context.subscriptions.push(
		vscode.workspace.onDidChangeTextDocument((event) => {
			if (event.document.languageId === 'patto') {
				debouncedRefreshTasks();
			}
		})
	);

	// Refresh tasks when switching between files
	context.subscriptions.push(
		vscode.window.onDidChangeActiveTextEditor(async (editor) => {
			if (editor && editor.document.languageId === 'patto') {
				await refreshTasks();
			}
		})
	);

	// Register commands
	context.subscriptions.push(
		commands.registerCommand("patto.tasks", async () => {
			try {
				traceOutputChannel.appendLine("[patto] Requesting tasks...");
				const response = await client.sendRequest(ExecuteCommandRequest.type, {
					command: "experimental/aggregate_tasks",
					arguments: [],
				});
				traceOutputChannel.appendLine("[patto] Tasks response: " + JSON.stringify(response));
				if (!response || (Array.isArray(response) && response.length === 0)) {
					vscode.window.showInformationMessage('No tasks found in workspace');
					tasksProvider.refresh([]);
				} else {
					tasksProvider.refresh(response as any[]);
					vscode.window.showInformationMessage(`Found ${(response as any[]).length} tasks`);
				}
			} catch (error) {
				traceOutputChannel.appendLine("[patto] Error aggregating tasks: " + error);
				vscode.window.showErrorMessage('Failed to aggregate tasks: ' + error);
			}
		})
	);

	context.subscriptions.push(
		commands.registerCommand("patto.scanWorkspace", async () => {
			try {
				await client.sendRequest(ExecuteCommandRequest.type, {
					command: "experimental/scan_workspace",
					arguments: [],
				});
				vscode.window.showInformationMessage('Workspace scan initiated');
			} catch (error) {
				traceOutputChannel.appendLine("[patto] Error scanning workspace: " + error);
			}
		})
	);

	context.subscriptions.push(
		commands.registerCommand("patto.twoHopLinks", async () => {
			const editor = vscode.window.activeTextEditor;
			if (!editor || editor.document.languageId !== 'patto') {
				vscode.window.showWarningMessage('No active Patto document');
				return;
			}

			try {
				const uri = editor.document.uri.toString();
				const response = await client.sendRequest(ExecuteCommandRequest.type, {
					command: "experimental/retrieve_two_hop_notes",
					arguments: [uri],
				}) as any[];

				if (!response || response.length === 0) {
					vscode.window.showInformationMessage('No 2-hop links found');
					return;
				}

				// Show in quickpick
				const items: vscode.QuickPickItem[] = [];
				for (const [nearestNode, twoHopLinks] of response) {
					const nodeName = Uri.parse(nearestNode).fsPath.split('/').pop() || nearestNode;
					items.push({ label: `→ ${nodeName}`, kind: vscode.QuickPickItemKind.Separator });
					for (const link of twoHopLinks) {
						const linkName = Uri.parse(link).fsPath.split('/').pop() || link;
						items.push({
							label: `  • ${linkName}`,
							description: link,
						});
					}
				}

				const selected = await vscode.window.showQuickPick(items, {
					placeHolder: 'Two-hop links from current note'
				});

				if (selected && selected.description) {
					const doc = await vscode.workspace.openTextDocument(Uri.parse(selected.description));
					await vscode.window.showTextDocument(doc);
				}
			} catch (error) {
				traceOutputChannel.appendLine("[patto] Error retrieving 2-hop links: " + error);
			}
		})
	);

	context.subscriptions.push(
		commands.registerCommand("patto.snapshotPapers", async () => {
			try {
				await client.sendRequest(ExecuteCommandRequest.type, {
					command: "patto/snapshotPapers",
					arguments: [],
				});
				vscode.window.showInformationMessage('Snapshot papers initiated');
			} catch (error) {
				traceOutputChannel.appendLine("[patto] Error snapshotting papers: " + error);
			}
		})
	);

	// Review completed tasks by timeframe
	context.subscriptions.push(
		commands.registerCommand("patto.taskReview", async () => {
			const timeframeChoice = await vscode.window.showQuickPick(
				[
					{ label: "Today", value: "today" },
					{ label: "This Week (Mon–today)", value: "this_week" },
					{ label: "Custom date range…", value: "custom" },
				],
				{ placeHolder: "Select timeframe for completed task review" }
			);
			if (!timeframeChoice) return;

			let args: string[];
			if (timeframeChoice.value === "custom") {
				const from = await vscode.window.showInputBox({
					prompt: "From date (YYYY-MM-DD)",
					placeHolder: "e.g. 2024-03-01",
					validateInput: (v) => /^\d{4}-\d{2}-\d{2}$/.test(v) ? null : "Enter date as YYYY-MM-DD",
				});
				if (!from) return;
				const to = await vscode.window.showInputBox({
					prompt: "To date (YYYY-MM-DD)",
					placeHolder: "e.g. 2024-03-31",
					validateInput: (v) => /^\d{4}-\d{2}-\d{2}$/.test(v) ? null : "Enter date as YYYY-MM-DD",
				});
				if (!to) return;
				args = ["custom", from, to];
			} else {
				args = [timeframeChoice.value];
			}

			try {
				const response = await client.sendRequest(ExecuteCommandRequest.type, {
					command: "experimental/tasks_review",
					arguments: args,
				}) as any[];

				if (!response || response.length === 0) {
					vscode.window.showInformationMessage("No completed tasks found for the selected period");
					return;
				}

				// Group by completed_at date
				const byDate = new Map<string, any[]>();
				for (const task of response) {
					const d = task.completed_at as string;
					if (!byDate.has(d)) byDate.set(d, []);
					byDate.get(d)!.push(task);
				}

				// Build quick pick items
				const items: vscode.QuickPickItem[] = [];
				for (const [date, tasks] of [...byDate.entries()].sort()) {
					items.push({ label: `📅 ${date}`, kind: vscode.QuickPickItemKind.Separator });
					for (const t of tasks) {
						const ts = t.time_spent;
						let timeChip = '';
						if (ts && typeof ts === 'object') {
							const h: number = ts.hours ?? 0;
							const m: number = ts.minutes ?? 0;
							if (h > 0 && m > 0) timeChip = ` ⏱ ${h}h${m}m`;
							else if (h > 0)     timeChip = ` ⏱ ${h}h`;
							else if (m > 0)     timeChip = ` ⏱ ${m}m`;
						}
						items.push({
							label: `  ✓ ${t.completed_at}  ${t.text}${timeChip}`,
							description: Uri.parse(t.location.uri).fsPath.split('/').pop(),
							detail: Uri.parse(t.location.uri).fsPath,
						});
					}
				}

				const selected = await vscode.window.showQuickPick(items, {
					placeHolder: `${response.length} completed task(s)`,
					matchOnDescription: true,
					matchOnDetail: true,
				});

				if (selected && selected.detail) {
					const doc = await vscode.workspace.openTextDocument(selected.detail);
					const task = response.find((t: any) =>
						Uri.parse(t.location.uri).fsPath === selected.detail &&
						selected.label.includes(t.text)
					);
					const line = task?.location?.range?.start?.line ?? 0;
					const ed = await vscode.window.showTextDocument(doc);
					ed.revealRange(new vscode.Range(line, 0, line, 0), vscode.TextEditorRevealType.InCenter);
					ed.selection = new vscode.Selection(line, 0, line, 0);
				}
			} catch (error) {
				traceOutputChannel.appendLine("[patto] Error in task review: " + error);
				vscode.window.showErrorMessage("Failed to retrieve completed tasks: " + error);
			}
		})
	);

	// Copy as Markdown command (uses configured default flavor)
	context.subscriptions.push(
		commands.registerCommand("patto.copyAsMarkdown", async () => {
			const editor = vscode.window.activeTextEditor;
			if (!editor || editor.document.languageId !== 'patto') {
				vscode.window.showWarningMessage('No active Patto document');
				return;
			}

			// Use configured default flavor (LSP will also fall back to its settings)
			const config = workspace.getConfiguration('patto');
			const flavor = config.get<string>('markdown.defaultFlavor', 'standard');

			try {
				const uri = editor.document.uri.toString();
				const selection = editor.selection;
				const args: (string | number | undefined)[] = [uri];

				// If there's a selection, include the range
				if (!selection.isEmpty) {
					args.push(selection.start.line);
					args.push(selection.end.line);
				} else {
					args.push(undefined);
					args.push(undefined);
				}
				args.push(flavor);

				const response = await client.sendRequest(ExecuteCommandRequest.type, {
					command: "patto/renderAsMarkdown",
					arguments: args,
				});

				if (response && typeof response === 'string') {
					await vscode.env.clipboard.writeText(response);
					const rangeInfo = selection.isEmpty ? 'document' : 'selection';
					vscode.window.showInformationMessage(`Copied ${rangeInfo} as ${flavor} markdown`);
				} else {
					vscode.window.showWarningMessage('Failed to render markdown');
				}
			} catch (error) {
				traceOutputChannel.appendLine("[patto] Error copying as markdown: " + error);
				vscode.window.showErrorMessage('Failed to copy as markdown: ' + error);
			}
		})
	);

	// Preview command
	context.subscriptions.push(
		commands.registerCommand("patto.openPreview", async () => {
			const workspaceFolders = vscode.workspace.workspaceFolders;
			if (!workspaceFolders) {
				vscode.window.showErrorMessage('No workspace folder open');
				return;
			}

			const rootPath = workspaceFolders[0].uri.fsPath;

			// Ensure preview binary is available
			const config = vscode.workspace.getConfiguration('patto');
			const configuredPreviewPath = config.get<string>('previewPath');
			const previewCommand = await binaryManager.ensureBinary(
				'patto-preview',
				configuredPreviewPath !== 'patto-preview' ? configuredPreviewPath : undefined
			);

			if (!previewCommand) {
				vscode.window.showErrorMessage('patto-preview binary not available');
				return;
			}

			// Launch preview server if not running
			if (!previewPort) {
				const port = await launchPreviewServer(rootPath, traceOutputChannel, previewCommand);
				if (!port) {
					return;
				}
			}

			// Create or show preview panel
			if (previewPanel) {
				previewPanel.reveal(ViewColumn.Beside);
			} else {
				previewPanel = vscode.window.createWebviewPanel(
					'pattoPreview',
					'Patto Preview',
					ViewColumn.Beside,
					{
						enableScripts: true,
						retainContextWhenHidden: true,
					}
				);

				previewPanel.webview.html = getPreviewHtml(previewPort!);

				previewPanel.onDidDispose(() => {
					previewPanel = null;
				});

				// Update preview when active editor changes
				vscode.window.onDidChangeActiveTextEditor((editor) => {
					if (editor && editor.document.languageId === 'patto' && previewPanel) {
						const relativePath = vscode.workspace.asRelativePath(editor.document.uri);
						previewPanel.webview.postMessage({
							type: 'navigateTo',
							note: relativePath
						});
					}
				});
			}
		})
	);

	// Auto-open preview when .pn file is opened
	vscode.workspace.onDidOpenTextDocument((document) => {
		if (document.languageId === 'patto' && !previewPanel) {
			const config = vscode.workspace.getConfiguration('patto');
			if (config.get('autoOpenPreview', false)) {
				commands.executeCommand('patto.openPreview');
			}
		}
	});

	context.subscriptions.push({
		dispose: () => {
			stopPreviewServer();
			if (previewPanel) {
				previewPanel.dispose();
			}
		}
	});

	traceOutputChannel.appendLine("[patto-lsp] Extension activated");
}

function getPreviewHtml(port: number): string {
	return `<!DOCTYPE html>
<html lang="en">
<head>
	<meta charset="UTF-8">
	<meta name="viewport" content="width=device-width, initial-scale=1.0">
	<title>Patto Preview</title>
	<style>
		body, html {
			margin: 0;
			padding: 0;
			width: 100%;
			height: 100%;
			overflow: hidden;
		}
		iframe {
			width: 100%;
			height: 100%;
			border: none;
		}
	</style>
</head>
<body>
	<iframe id="preview-frame" src="http://localhost:${port}"></iframe>
	<script>
		const vscode = acquireVsCodeApi();
		const iframe = document.getElementById('preview-frame');
		
		window.addEventListener('message', event => {
			const message = event.data;
			if (message.type === 'navigateTo') {
				iframe.src = 'http://localhost:${port}?note=' + encodeURIComponent(message.note);
			}
		});
	</script>
</body>
</html>`;
}

export function deactivate(): Thenable<void> | undefined {
	stopPreviewServer();
	if (previewPanel) {
		previewPanel.dispose();
	}
	return client ? client.stop() : Promise.resolve();
}

