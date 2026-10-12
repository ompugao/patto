import * as vscode from 'vscode';
import type { PattoLsp, TwoHopLinks } from '../languageClient';

const noteName = (uri: string) => vscode.Uri.parse(uri).fsPath.split('/').pop() || uri;

function twoHopItems(links: TwoHopLinks): vscode.QuickPickItem[] {
	const items: vscode.QuickPickItem[] = [];
	for (const [nearestNode, twoHopLinks] of links) {
		items.push({ label: `→ ${noteName(nearestNode)}`, kind: vscode.QuickPickItemKind.Separator });
		for (const link of twoHopLinks) {
			items.push({
				label: `  • ${noteName(link)}`,
				description: link,
			});
		}
	}
	return items;
}

/** patto.twoHopLinks: notes reachable through the current note's links, opened on pick. */
export async function showTwoHopLinks(lsp: PattoLsp, outputChannel: vscode.OutputChannel): Promise<void> {
	const editor = vscode.window.activeTextEditor;
	if (!editor || editor.document.languageId !== 'patto') {
		vscode.window.showWarningMessage('No active Patto document');
		return;
	}

	try {
		const links = await lsp.twoHopLinks(editor.document.uri.toString());
		if (!links || links.length === 0) {
			vscode.window.showInformationMessage('No 2-hop links found');
			return;
		}

		const selected = await vscode.window.showQuickPick(twoHopItems(links), {
			placeHolder: 'Two-hop links from current note'
		});
		if (selected && selected.description) {
			const doc = await vscode.workspace.openTextDocument(vscode.Uri.parse(selected.description));
			await vscode.window.showTextDocument(doc);
		}
	} catch (error) {
		outputChannel.appendLine("[patto] Error retrieving 2-hop links: " + error);
	}
}
