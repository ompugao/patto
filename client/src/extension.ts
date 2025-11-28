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
} from 'vscode-languageclient/node';

import * as fs from 'fs';
import * as path from 'path';
import * as net from 'net';
import { ChildProcess, spawn } from 'child_process';
import { BinaryManager } from './binaryManager';

let client: LanguageClient;
let previewServer: ChildProcess | null = null;
let previewPort: number | null = null;
let previewPanel: vscode.WebviewPanel | null = null;
let taskRefreshTimeout: NodeJS.Timeout | null = null;
let previewBridgeClient: LanguageClient | null = null;
let previewLspPort: number | null = null;

const delay = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

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

async function startPreviewBridge(port: number, outputChannel: OutputChannel): Promise<void> {
	if (previewBridgeClient) {
		await previewBridgeClient.stop();
		previewBridgeClient = null;
	}

	const serverOptions: ServerOptions = () =>
		new Promise((resolve, reject) => {
			const socket = net.connect(port, '127.0.0.1', () => {
				resolve({ reader: socket, writer: socket });
			});
			socket.on('error', reject);
		});

	const clientOptions: LanguageClientOptions = {
		documentSelector: [{ language: 'patto' }],
	};

	previewBridgeClient = new LanguageClient(
		'pattoPreviewBridge',
		'Patto Preview Bridge',
		serverOptions,
		clientOptions
	);

	try {
		await previewBridgeClient.start();
		outputChannel.appendLine(`[patto] Connected preview bridge on port ${port}`);
	} catch (error) {
		previewBridgeClient = null;
		throw error;
	}
}

function stopPreviewBridge() {
	if (previewBridgeClient) {
		previewBridgeClient.stop().catch(() => undefined);
		previewBridgeClient = null;
	}
	previewLspPort = null;
}

// Launch preview server
async function launchPreviewServer(rootPath: string, outputChannel: OutputChannel, command: string): Promise<number | null> {
	const port = await findAvailablePort(3000);
	if (!port) {
		vscode.window.showErrorMessage('Could not find an available port for preview server');
		return null;
	}

	let lspPort = await findAvailablePort(9250);
	if (!lspPort) {
		vscode.window.showErrorMessage('Could not find an available port for preview bridge');
		return null;
	}

	if (lspPort === port) {
		const nextPort = await findAvailablePort(lspPort + 1);
		if (!nextPort) {
			vscode.window.showErrorMessage('Could not find a distinct port for preview bridge');
			return null;
		}
		lspPort = nextPort;
	}

	outputChannel.appendLine(`[patto] Launching preview server on port ${port} with bridge ${lspPort} using command: ${command}`);

	try {
		previewServer = spawn(command, ['--port', port.toString(), '--preview-lsp-port', lspPort.toString()], {
			cwd: rootPath,
			stdio: ['ignore', 'pipe', 'pipe']
		});
	} catch (error) {
		vscode.window.showErrorMessage(
			`Failed to launch patto-preview: ${error}\n\n` +
			`Please try downloading it again or install manually.`
		);
		return null;
	}

	if (previewServer.stdout) {
		previewServer.stdout.on('data', (data) => {
			outputChannel.appendLine(`[preview-server] ${data.toString()}`);
		});
	}

	if (previewServer.stderr) {
		previewServer.stderr.on('data', (data) => {
			outputChannel.appendLine(`[preview-server] ${data.toString()}`);
		});
	}

	previewServer.on('close', (code) => {
		outputChannel.appendLine(`[preview-server] exited with code ${code}`);
		previewServer = null;
		previewPort = null;
		stopPreviewBridge();
	});

	previewPort = port;
	previewLspPort = lspPort;

	await delay(500);
	try {
		await startPreviewBridge(lspPort, outputChannel);
	} catch (error) {
		outputChannel.appendLine(`[patto] Failed to connect preview bridge: ${error}`);
		vscode.window.showWarningMessage('Patto preview live updates unavailable. Falling back to save events.');
	}

	return port;
}

// Stop preview server
function stopPreviewServer() {
	if (previewServer) {
		previewServer.kill();
		previewServer = null;
		previewPort = null;
	}
	stopPreviewBridge();
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
        result[i]['text'],
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
			fileEvents: workspace.createFileSystemWatcher('**/*.pn')
		},
		outputChannel: traceOutputChannel
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

