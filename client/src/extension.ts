/* --------------------------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 * ------------------------------------------------------------------------------------------ */

import { window, workspace, ExtensionContext, OutputChannel } from 'vscode';

import {
	Executable,
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
} from 'vscode-languageclient/node';


let client: LanguageClient;

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

	// Start the client. This will also launch the server
	client.start();
	// context.subscriptions.push(client.start());
}

export function deactivate(): Thenable<void> | undefined {
    return client ? client.stop() : Promise.resolve();
}

