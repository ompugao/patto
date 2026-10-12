import * as vscode from 'vscode';
import type { BinaryManager } from '../binaries/binaryManager';
import { autoOpenPreview, configuredBinaryPath } from '../configuration';
import { PreviewPanel } from './previewPanel';
import { PreviewServer } from './previewServer';

/** patto.openPreview and the optional auto-open on .pn files; the returned disposable shuts both down. */
export function registerPreview(
	context: vscode.ExtensionContext,
	binaryManager: BinaryManager,
	outputChannel: vscode.OutputChannel,
): vscode.Disposable {
	const server = new PreviewServer(outputChannel);
	const panel = new PreviewPanel();

	context.subscriptions.push(
		vscode.commands.registerCommand("patto.openPreview", async () => {
			const workspaceFolders = vscode.workspace.workspaceFolders;
			if (!workspaceFolders) {
				vscode.window.showErrorMessage('No workspace folder open');
				return;
			}
			const rootPath = workspaceFolders[0].uri.fsPath;

			const previewCommand = await binaryManager.ensureBinary('patto-preview', configuredBinaryPath('patto-preview'));
			if (!previewCommand) {
				vscode.window.showErrorMessage('patto-preview binary not available');
				return;
			}

			const port = server.port ?? await server.launch(rootPath, previewCommand);
			if (!port) {
				return;
			}
			panel.show(port);
		}),

		vscode.workspace.onDidOpenTextDocument((document) => {
			if (document.languageId === 'patto' && !panel.isOpen && autoOpenPreview()) {
				vscode.commands.executeCommand('patto.openPreview');
			}
		}),
	);

	const shutdown = new vscode.Disposable(() => {
		server.stop();
		panel.dispose();
	});
	context.subscriptions.push(shutdown);
	return shutdown;
}
