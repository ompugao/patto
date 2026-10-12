import { workspace, OutputChannel } from 'vscode';
import {
	Executable,
	ExecuteCommandRequest,
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
} from 'vscode-languageclient/node';
import { lspInitializationOptions } from './configuration';
import type { CompletedTask, TaskInformation } from './tasks/taskLabel';

export function createLanguageClient(command: string, outputChannel: OutputChannel): LanguageClient {
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
		outputChannel,
		initializationOptions: lspInitializationOptions()
	};

	return new LanguageClient(
		'patto-language-server',
		'Patto Language Server',
		serverOptions,
		clientOptions
	);
}

/** [note, notes two hops away through it] pairs, as retrieve_two_hop_notes returns them. */
export type TwoHopLinks = [string, string[]][];

/** The custom workspace/executeCommand calls patto-lsp understands. */
export class PattoLsp {
	constructor(private readonly client: LanguageClient) {}

	aggregateTasks(): Promise<TaskInformation[] | null> {
		return this.execute("experimental/aggregate_tasks");
	}

	scanWorkspace(): Promise<unknown> {
		return this.execute("experimental/scan_workspace");
	}

	twoHopLinks(uri: string): Promise<TwoHopLinks | null> {
		return this.execute("experimental/retrieve_two_hop_notes", [uri]);
	}

	snapshotPapers(): Promise<unknown> {
		return this.execute("patto/snapshotPapers");
	}

	/** `["today"]`, `["this_week"]` or `["custom", from, to]` with YYYY-MM-DD dates. */
	tasksReview(timeframe: string[]): Promise<CompletedTask[] | null> {
		return this.execute("experimental/tasks_review", timeframe);
	}

	/** Lines are 0-based and inclusive; both undefined renders the whole document. */
	renderAsMarkdown(uri: string, startLine: number | undefined, endLine: number | undefined, flavor: string): Promise<unknown> {
		return this.execute("patto/renderAsMarkdown", [uri, startLine, endLine, flavor]);
	}

	private execute<T>(command: string, args: unknown[] = []): Promise<T> {
		return this.client.sendRequest(ExecuteCommandRequest.type, { command, arguments: args });
	}
}
