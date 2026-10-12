import * as vscode from 'vscode';
import { markdownDefaultFlavor } from '../configuration';
import type { PattoLsp } from '../languageClient';

/** patto.copyAsMarkdown: the selection, or the whole document, rendered in the configured flavor. */
export async function copyAsMarkdown(lsp: PattoLsp, outputChannel: vscode.OutputChannel): Promise<void> {
	const editor = vscode.window.activeTextEditor;
	if (!editor || editor.document.languageId !== 'patto') {
		vscode.window.showWarningMessage('No active Patto document');
		return;
	}

	// The server falls back to its own settings when the flavor is omitted; sending
	// ours keeps the two in step when VS Code's configuration is ahead of it.
	const flavor = markdownDefaultFlavor();
	const selection = editor.selection;

	try {
		const response = await lsp.renderAsMarkdown(
			editor.document.uri.toString(),
			selection.isEmpty ? undefined : selection.start.line,
			selection.isEmpty ? undefined : selection.end.line,
			flavor,
		);

		if (response && typeof response === 'string') {
			await vscode.env.clipboard.writeText(response);
			const rangeInfo = selection.isEmpty ? 'document' : 'selection';
			vscode.window.showInformationMessage(`Copied ${rangeInfo} as ${flavor} markdown`);
		} else {
			vscode.window.showWarningMessage('Failed to render markdown');
		}
	} catch (error) {
		outputChannel.appendLine("[patto] Error copying as markdown: " + error);
		vscode.window.showErrorMessage('Failed to copy as markdown: ' + error);
	}
}
