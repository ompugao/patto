/* --------------------------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 * ------------------------------------------------------------------------------------------ */

import { window, workspace, ExtensionContext, OutputChannel, commands } from 'vscode';
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

let client: LanguageClient;

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

  refresh(result): void {
    this.tasks = [];
    for (let i = 0; i < result.length; ++i) {
      this.tasks.push(new Task(
        result[i]['text'],
        vscode.TreeItemCollapsibleState.Collapsed
      ));
    }
    this._onDidChangeTreeData.fire();
  }

  //private pathExists(p: string): boolean {
  //  try {
  //    fs.accessSync(p);
  //  } catch (err) {
  //    return false;
  //  }
  //  return true;
  //}
}

class Task extends vscode.TreeItem {
  constructor(
    public readonly label: string,
    public readonly collapsibleState: vscode.TreeItemCollapsibleState
  ) {
    super(label, collapsibleState);
    this.tooltip = `${this.label}`;
    this.description = this.label;
  }

  //iconPath = {
  //  light: path.join(__filename, '..', '..', 'resources', 'light', 'task.svg'),
  //  dark: path.join(__filename, '..', '..', 'resources', 'dark', 'task.svg')
  //};
}



export function activate(context: ExtensionContext): void {
	const command = process.env.SERVER_PATH || "tabton-lsp";

	const traceOutputChannel: OutputChannel = window.createOutputChannel("Tabton-Language-Server-trace");
	traceOutputChannel.appendLine("[tabton-lsp-extension] Start running " + command);
	traceOutputChannel.show(true);

	const run: Executable = {
		command,
		options: {
			env: {
				...process.env,
				// eslint-disable-next-line @typescript-eslint/naming-convention
				RUST_LOG: "debug",
				RUST_BACKTRACE: 1,
			},
		},
	};
	const serverOptions: ServerOptions = {
		run,
		debug: run,
	};

	// If the extension is launched in debug mode then the debug server options are used
	// Otherwise the run options are used

	// Options to control the language client
	const clientOptions: LanguageClientOptions = {
		// Register the server for plain text documents
		documentSelector: [
			{ scheme: "file", language: "tabton" },
			{ scheme: "untitled", language: "tabton" },
		],
		synchronize: {
			// Notify the server about file changes to '.clientrc files contained in the workspace
			fileEvents: workspace.createFileSystemWatcher('**/.clientrc')
		},
		outputChannel: traceOutputChannel
	};

	// Create the language client and start the client.
	client = new LanguageClient(
		'tabton-language-server',
		'Tabton Language Server',
		serverOptions,
		clientOptions
	);

	const tasksProvider = new TasksProvider();
	context.subscriptions.push(window.createTreeView('tabtonTasks', {
		treeDataProvider: tasksProvider
	}));

	// Start the client. This will also launch the server
	//client.start();
	// context.subscriptions.push(client.start());
	context.subscriptions.push(
		commands.registerCommand("tabton.tasks", async () => {
			await client.start(),
			await client.sendRequest(ExecuteCommandRequest.type, {
				command: "experimental/aggregate_tasks",
				arguments: [],
			}).then((response) => {
				traceOutputChannel.appendLine("[tabton-lsp-extension] " + response);
				tasksProvider.refresh(response);
			} , (error) => {
				traceOutputChannel.appendLine("[tabton-lsp-extension] error! " + error);
			});
		})
	);
}

export function deactivate(): Thenable<void> | undefined {
    return client ? client.stop() : Promise.resolve();
}

