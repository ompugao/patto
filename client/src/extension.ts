/* --------------------------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 * ------------------------------------------------------------------------------------------ */

import { window, ExtensionContext, OutputChannel, Disposable } from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';

import { BinaryManager } from './binaries/binaryManager';
import { registerCommands } from './commands';
import { configuredBinaryPath } from './configuration';
import { createLanguageClient, PattoLsp } from './languageClient';
import { registerPreview } from './preview';
import { registerTasksView } from './tasks/tasksView';

let client: LanguageClient | undefined;
let preview: Disposable | undefined;

export function activate(context: ExtensionContext): void {
	const traceOutputChannel: OutputChannel = window.createOutputChannel("Patto Language Server");
	const binaryManager = new BinaryManager(context, traceOutputChannel);

	binaryManager.ensureBinary('patto-lsp', configuredBinaryPath('patto-lsp'))
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
	client = createLanguageClient(command, traceOutputChannel);
	client.start();

	const lsp = new PattoLsp(client);
	const tasksProvider = registerTasksView(context, lsp, traceOutputChannel);
	registerCommands(context, lsp, tasksProvider, traceOutputChannel);
	preview = registerPreview(context, binaryManager, traceOutputChannel);

	traceOutputChannel.appendLine("[patto-lsp] Extension activated");
}

export function deactivate(): Thenable<void> | undefined {
	preview?.dispose();
	return client ? client.stop() : Promise.resolve();
}
