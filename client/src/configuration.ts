import { workspace } from 'vscode';
import type { BinaryName } from './binaries/platform';

const config = () => workspace.getConfiguration('patto');

export function markdownDefaultFlavor(): string {
	return config().get<string>('markdown.defaultFlavor', 'standard');
}

/** What the server reads as initializationOptions. */
export function lspInitializationOptions() {
	return {
		markdown: {
			defaultFlavor: markdownDefaultFlavor()
		}
	};
}

/** The user's explicit binary path, or undefined when the setting is still the bare command name. */
export function configuredBinaryPath(binaryName: BinaryName): string | undefined {
	const setting = binaryName === 'patto-lsp' ? 'lspPath' : 'previewPath';
	const value = config().get<string>(setting);
	return value !== binaryName ? value : undefined;
}

export function autoOpenPreview(): boolean {
	return config().get('autoOpenPreview', false);
}
