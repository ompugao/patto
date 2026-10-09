import * as vscode from 'vscode';
import {
	Executable,
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
	State,
} from 'vscode-languageclient/node';
import { findAvailablePort } from './ports';

/**
 * The patto-preview process. It is driven as a second language client
 * (`--preview-lsp-stdio`) so it sees unsaved buffers, and it serves the web
 * UI on `port`.
 */
export class PreviewServer {
	private client: LanguageClient | null = null;
	private _port: number | null = null;

	constructor(private readonly outputChannel: vscode.OutputChannel) {}

	get port(): number | null {
		return this._port;
	}

	async launch(rootPath: string, command: string): Promise<number | null> {
		const port = await findAvailablePort(3000);
		if (!port) {
			vscode.window.showErrorMessage('Could not find an available port for preview server');
			return null;
		}

		if (this.client) {
			await this.client.stop().catch(() => undefined);
			this.client = null;
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

		const client = new LanguageClient('pattoPreview', 'Patto Preview', serverOptions, clientOptions);
		this.client = client;
		client.onDidChangeState((event) => {
			if (event.newState === State.Stopped && this.client === client) {
				this.client = null;
				this._port = null;
				this.outputChannel.appendLine('[patto] Preview server stopped');
			}
		});

		try {
			await client.start();
			this.outputChannel.appendLine(`[patto] Launching preview server on port ${port} with command: ${command}`);
		} catch (error) {
			this.client = null;
			const message = `Failed to launch patto-preview: ${error}`;
			this.outputChannel.appendLine(`[patto] ${message}`);
			vscode.window.showErrorMessage(message);
			return null;
		}

		this._port = port;
		return port;
	}

	stop(): void {
		if (this.client) {
			this.client.stop().catch(() => undefined);
			this.client = null;
		}
		this._port = null;
	}
}
