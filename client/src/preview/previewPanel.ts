import * as vscode from 'vscode';

/** The webview that frames the preview server's page beside the editor. */
export class PreviewPanel {
	private panel: vscode.WebviewPanel | null = null;

	get isOpen(): boolean {
		return this.panel !== null;
	}

	show(port: number): void {
		if (this.panel) {
			this.panel.reveal(vscode.ViewColumn.Beside);
			return;
		}

		const panel = vscode.window.createWebviewPanel(
			'pattoPreview',
			'Patto Preview',
			vscode.ViewColumn.Beside,
			{
				enableScripts: true,
				retainContextWhenHidden: true,
			}
		);
		this.panel = panel;
		panel.webview.html = previewHtml(port);
		panel.onDidDispose(() => {
			this.panel = null;
		});

		vscode.window.onDidChangeActiveTextEditor((editor) => {
			if (editor && editor.document.languageId === 'patto' && this.panel) {
				this.panel.webview.postMessage({
					type: 'navigateTo',
					note: vscode.workspace.asRelativePath(editor.document.uri)
				});
			}
		});
	}

	dispose(): void {
		this.panel?.dispose();
	}
}

function previewHtml(port: number): string {
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
