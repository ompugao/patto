import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { downloadFile, extractArchive, isExecutable, isOnPath } from './download';
import {
    archiveName, archiveUrl, BINARY_NAMES, executableName, manualInstallCommand, releaseTarget,
    type BinaryName,
} from './platform';
import { BINARY_VERSION, freshMetadata, staleness, VersionMetadataFile, type VersionMetadata } from './versionMetadata';

/**
 * Finds patto-lsp / patto-preview: a configured path, then PATH, then the
 * copy downloaded into global storage, offering to download when none exists.
 */
export class BinaryManager {
    private readonly binDir: string;
    private readonly metadataFile: VersionMetadataFile;

    constructor(context: vscode.ExtensionContext, private readonly outputChannel: vscode.OutputChannel) {
        const storage = context.globalStorageUri.fsPath;
        this.binDir = path.join(storage, 'bin');
        this.metadataFile = new VersionMetadataFile(path.join(storage, 'version.json'), (line) => this.log(line));
    }

    public async ensureBinary(binaryName: BinaryName, configPath?: string): Promise<string | null> {
        if (configPath) {
            if (isExecutable(configPath)) {
                this.log(`Using configured path: ${configPath}`);
                return configPath;
            }
            this.log(`Configured path not found: ${configPath}`);
        }

        if (isOnPath(binaryName)) {
            this.log(`Found ${binaryName} in PATH`);
            return binaryName;
        }
        this.log(`${binaryName} not found in PATH`);

        const localPath = path.join(this.binDir, executableName(binaryName));
        if (fs.existsSync(localPath)) {
            this.log(`Found cached binary: ${localPath}`);
            const metadata = this.metadataFile.read();
            if (metadata && this.isOutdated(metadata)) {
                const updated = await this.offerUpdate(binaryName, metadata);
                if (updated) return updated;
                // A failed update has already removed the cached binary.
                if (!fs.existsSync(localPath)) return null;
            }
            return localPath;
        }

        return this.offerDownload(binaryName);
    }

    private isOutdated(metadata: VersionMetadata): boolean {
        switch (staleness(metadata)) {
            case 'version-mismatch':
                this.log(`Version mismatch: ${metadata.version} != ${BINARY_VERSION}`);
                return true;
            case 'check-interval-expired':
                this.log('Version check interval expired');
                return true;
            default:
                return false;
        }
    }

    private async offerUpdate(binaryName: BinaryName, metadata: VersionMetadata): Promise<string | null> {
        const choice = await vscode.window.showInformationMessage(
            `A newer version of Patto binaries is available (${BINARY_VERSION}). Update now?`,
            'Update',
            'Skip',
            'Always Skip'
        );
        if (choice === 'Update') {
            this.cleanupBinaries();
            const downloaded = await this.downloadBinary(binaryName);
            if (downloaded) {
                vscode.window.showInformationMessage(`Patto binaries updated to ${BINARY_VERSION}!`);
                return downloaded;
            }
            return null;
        }
        // Both skips defer the next offer by one check interval
        metadata.lastCheckedAt = Date.now();
        this.metadataFile.write(metadata);
        return null;
    }

    private async offerDownload(binaryName: BinaryName): Promise<string | null> {
        const choice = await vscode.window.showInformationMessage(
            'Patto binaries are required but not found. Download from GitHub releases?',
            'Download',
            'Install Manually',
            'Configure Path'
        );

        if (choice === 'Download') {
            const downloaded = await this.downloadBinary(binaryName);
            if (downloaded) {
                vscode.window.showInformationMessage('Patto binaries downloaded successfully!');
                return downloaded;
            }
            vscode.window.showErrorMessage('Failed to download binaries. Please install manually.');
            return null;
        }
        if (choice === 'Configure Path') {
            vscode.commands.executeCommand('workbench.action.openSettings', `patto.${binaryName === 'patto-lsp' ? 'lspPath' : 'previewPath'}`);
            return null;
        }
        const command = manualInstallCommand(binaryName);
        vscode.window.showInformationMessage(`To install manually:\n\n${command}`, 'Copy Command').then(selection => {
            if (selection === 'Copy Command') {
                vscode.env.clipboard.writeText(command);
            }
        });
        return null;
    }

    /** Downloads the release archive, which holds both binaries, and extracts it into binDir. */
    private async downloadBinary(binaryName: BinaryName): Promise<string | null> {
        const target = releaseTarget();
        if (!target) {
            this.log(`Unsupported platform: ${process.platform}-${process.arch}`);
            return null;
        }

        fs.mkdirSync(this.binDir, { recursive: true });
        const downloadUrl = archiveUrl(target, BINARY_VERSION);
        const archivePath = path.join(this.binDir, archiveName(target));
        const extractedBinaryPath = path.join(this.binDir, binaryName + target.exeExt);

        try {
            if (fs.existsSync(extractedBinaryPath)) {
                this.log(`Binary already extracted: ${extractedBinaryPath}`);
                return extractedBinaryPath;
            }

            this.log(`Downloading archive from ${downloadUrl}`);
            this.log(`Archive path: ${archivePath}`);
            this.log(`Expected binary path: ${extractedBinaryPath}`);

            await vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification,
                title: 'Downloading Patto binaries...',
                cancellable: false
            }, async (progress) => {
                progress.report({ message: 'Downloading from GitHub releases...' });
                await downloadFile(downloadUrl, archivePath);
                progress.report({ message: 'Extracting binaries...' });
                extractArchive(archivePath, this.binDir, target.isWindows);
            });

            this.removeArchive(archivePath);

            if (!fs.existsSync(extractedBinaryPath)) {
                throw new Error(`Binary not found after extraction: ${extractedBinaryPath}`);
            }
            if (!target.isWindows) {
                this.markExecutable();
            }

            this.log(`Successfully extracted to ${extractedBinaryPath}`);
            this.metadataFile.write(freshMetadata());
            this.log(`Saved version metadata: ${BINARY_VERSION}`);
            return extractedBinaryPath;
        } catch (error) {
            this.log(`Failed to download/extract: ${error}`);
            this.removeArchive(archivePath);
            return null;
        }
    }

    private removeArchive(archivePath: string): void {
        if (fs.existsSync(archivePath)) {
            this.log(`Cleaning up archive: ${archivePath}`);
            fs.unlinkSync(archivePath);
        }
    }

    private markExecutable(): void {
        this.log('Making binaries executable');
        for (const name of BINARY_NAMES) {
            const binaryPath = path.join(this.binDir, name);
            if (fs.existsSync(binaryPath)) {
                fs.chmodSync(binaryPath, 0o755);
            }
        }
    }

    private cleanupBinaries(): void {
        if (!fs.existsSync(this.binDir)) {
            return;
        }
        for (const file of fs.readdirSync(this.binDir)) {
            const filePath = path.join(this.binDir, file);
            if (fs.statSync(filePath).isFile()) {
                fs.unlinkSync(filePath);
                this.log(`Deleted old binary: ${filePath}`);
            }
        }
    }

    private log(line: string): void {
        this.outputChannel.appendLine(`[BinaryManager] ${line}`);
    }
}
