import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import * as https from 'https';
import { execSync } from 'child_process';

const GITHUB_REPO = 'ompugao/patto';
const BINARY_VERSION = '0.4.1'; // Should match Cargo.toml version
const VERSION_CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000; // 24 hours

interface BinaryInfo {
    name: string;
    downloadUrl: string;
    localPath: string;
}

interface VersionMetadata {
    version: string;
    downloadedAt: number;
    lastCheckedAt: number;
}

export class BinaryManager {
    private context: vscode.ExtensionContext;
    private outputChannel: vscode.OutputChannel;

    constructor(context: vscode.ExtensionContext, outputChannel: vscode.OutputChannel) {
        this.context = context;
        this.outputChannel = outputChannel;
    }

    private getBinaryDir(): string {
        return path.join(this.context.globalStorageUri.fsPath, 'bin');
    }

    private getVersionMetadataPath(): string {
        return path.join(this.context.globalStorageUri.fsPath, 'version.json');
    }

    private readVersionMetadata(): VersionMetadata | null {
        const metadataPath = this.getVersionMetadataPath();
        if (fs.existsSync(metadataPath)) {
            try {
                const content = fs.readFileSync(metadataPath, 'utf-8');
                return JSON.parse(content);
            } catch (error) {
                this.outputChannel.appendLine(`[BinaryManager] Failed to read version metadata: ${error}`);
                return null;
            }
        }
        return null;
    }

    private writeVersionMetadata(metadata: VersionMetadata): void {
        const metadataPath = this.getVersionMetadataPath();
        const dir = path.dirname(metadataPath);
        if (!fs.existsSync(dir)) {
            fs.mkdirSync(dir, { recursive: true });
        }
        fs.writeFileSync(metadataPath, JSON.stringify(metadata, null, 2));
    }

    private isVersionOutdated(metadata: VersionMetadata): boolean {
        // Check if version is different
        if (metadata.version !== BINARY_VERSION) {
            this.outputChannel.appendLine(`[BinaryManager] Version mismatch: ${metadata.version} != ${BINARY_VERSION}`);
            return true;
        }

        // Check if last check was more than 24 hours ago
        const now = Date.now();
        if (now - metadata.lastCheckedAt > VERSION_CHECK_INTERVAL_MS) {
            this.outputChannel.appendLine(`[BinaryManager] Version check interval expired`);
            return true;
        }

        return false;
    }

    private getPlatformInfo(): { platform: string; arch: string; ext: string } | null {
        const platform = process.platform;
        const arch = process.arch;

        // Map Node.js platform/arch to GitHub release naming
        const platformMap: { [key: string]: string } = {
            'darwin': 'apple-darwin',
            'linux': 'unknown-linux-gnu',
            'win32': 'pc-windows-msvc'
        };

        const archMap: { [key: string]: string } = {
            'x64': 'x86_64',
            'arm64': 'aarch64'
        };

        const mappedPlatform = platformMap[platform];
        const mappedArch = archMap[arch];

        if (!mappedPlatform || !mappedArch) {
            return null;
        }

        const ext = platform === 'win32' ? '.exe' : '';

        return {
            platform: mappedPlatform,
            arch: mappedArch,
            ext
        };
    }

    private async downloadFile(url: string, destination: string): Promise<void> {
        return new Promise((resolve, reject) => {
            const file = fs.createWriteStream(destination);
            
            https.get(url, (response) => {
                if (response.statusCode === 302 || response.statusCode === 301) {
                    // Follow redirect
                    if (response.headers.location) {
                        https.get(response.headers.location, (redirectResponse) => {
                            redirectResponse.pipe(file);
                            file.on('finish', () => {
                                file.close();
                                resolve();
                            });
                        }).on('error', (err) => {
                            fs.unlinkSync(destination);
                            reject(err);
                        });
                    }
                } else if (response.statusCode === 200) {
                    response.pipe(file);
                    file.on('finish', () => {
                        file.close();
                        resolve();
                    });
                } else {
                    file.close();
                    fs.unlinkSync(destination);
                    reject(new Error(`Failed to download: ${response.statusCode}`));
                }
            }).on('error', (err) => {
                fs.unlinkSync(destination);
                reject(err);
            });
        });
    }

    private async downloadBinary(binaryName: 'patto-lsp' | 'patto-preview'): Promise<string | null> {
        const platformInfo = this.getPlatformInfo();
        if (!platformInfo) {
            this.outputChannel.appendLine(`[BinaryManager] Unsupported platform: ${process.platform}-${process.arch}`);
            return null;
        }

        const { platform, arch, ext } = platformInfo;
        const target = `${arch}-${platform}`;
        
        // Download bundled archive (contains both binaries)
        // Windows uses .zip, others use .tar.xz
        const isWindows = process.platform === 'win32';
        const archiveExt = isWindows ? '.zip' : '.tar.xz';
        const archiveName = `patto-${target}${archiveExt}`;
        const downloadUrl = `https://github.com/${GITHUB_REPO}/releases/download/v${BINARY_VERSION}/${archiveName}`;
        
        // Ensure binary directory exists
        const binDir = this.getBinaryDir();
        if (!fs.existsSync(binDir)) {
            fs.mkdirSync(binDir, { recursive: true });
        }

        const archivePath = path.join(binDir, archiveName);
        const extractedBinaryPath = path.join(binDir, binaryName + ext);

        try {
            // Check if already extracted
            if (fs.existsSync(extractedBinaryPath)) {
                this.outputChannel.appendLine(`[BinaryManager] Binary already extracted: ${extractedBinaryPath}`);
                return extractedBinaryPath;
            }

            this.outputChannel.appendLine(`[BinaryManager] Downloading archive from ${downloadUrl}`);
            this.outputChannel.appendLine(`[BinaryManager] Archive path: ${archivePath}`);
            this.outputChannel.appendLine(`[BinaryManager] Expected binary path: ${extractedBinaryPath}`);
            
            await vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification,
                title: `Downloading Patto binaries...`,
                cancellable: false
            }, async (progress) => {
                progress.report({ message: 'Downloading from GitHub releases...' });
                await this.downloadFile(downloadUrl, archivePath);
                
                progress.report({ message: 'Extracting binaries...' });
                await this.extractArchive(archivePath, binDir, isWindows);
            });

            // Clean up archive first
            if (fs.existsSync(archivePath)) {
                this.outputChannel.appendLine(`[BinaryManager] Cleaning up archive: ${archivePath}`);
                fs.unlinkSync(archivePath);
            }

            // Verify extraction succeeded
            if (!fs.existsSync(extractedBinaryPath)) {
                throw new Error(`Binary not found after extraction: ${extractedBinaryPath}`);
            }

            // Make executable on Unix-like systems
            if (process.platform !== 'win32') {
                this.outputChannel.appendLine(`[BinaryManager] Making binaries executable`);
                if (fs.existsSync(extractedBinaryPath)) {
                    fs.chmodSync(extractedBinaryPath, 0o755);
                }
                // Also chmod the other binary if it was extracted
                const otherBinary = binaryName === 'patto-lsp' ? 'patto-preview' : 'patto-lsp';
                const otherBinaryPath = path.join(binDir, otherBinary);
                if (fs.existsSync(otherBinaryPath)) {
                    fs.chmodSync(otherBinaryPath, 0o755);
                }
            }

            this.outputChannel.appendLine(`[BinaryManager] Successfully extracted to ${extractedBinaryPath}`);
            
            // Write version metadata
            const metadata: VersionMetadata = {
                version: BINARY_VERSION,
                downloadedAt: Date.now(),
                lastCheckedAt: Date.now()
            };
            this.writeVersionMetadata(metadata);
            this.outputChannel.appendLine(`[BinaryManager] Saved version metadata: ${BINARY_VERSION}`);
            
            return extractedBinaryPath;

        } catch (error) {
            this.outputChannel.appendLine(`[BinaryManager] Failed to download/extract: ${error}`);
            // Clean up on error
            if (fs.existsSync(archivePath)) {
                fs.unlinkSync(archivePath);
            }
            return null;
        }
    }

    private async extractArchive(archivePath: string, destDir: string, isWindows: boolean): Promise<void> {
        const { execSync } = require('child_process');
        
        try {
            if (isWindows) {
                // Use PowerShell Expand-Archive on Windows for .zip files
                execSync(`powershell -command "Expand-Archive -Path '${archivePath}' -DestinationPath '${destDir}' -Force"`, { stdio: 'ignore' });
            } else {
                // Use tar on Unix-like systems for .tar.xz files
                // Extract directly to destDir, stripping the first directory component
                execSync(`tar -xf "${archivePath}" -C "${destDir}" --strip-components=1`, { stdio: 'ignore' });
            }
        } catch (error) {
            throw new Error(`Failed to extract archive: ${error}`);
        }
    }

    public async ensureBinary(binaryName: 'patto-lsp' | 'patto-preview', configPath?: string): Promise<string | null> {
        // 1. Check if configured path exists
        if (configPath) {
            try {
                fs.accessSync(configPath, fs.constants.X_OK);
                this.outputChannel.appendLine(`[BinaryManager] Using configured path: ${configPath}`);
                return configPath;
            } catch {
                this.outputChannel.appendLine(`[BinaryManager] Configured path not found: ${configPath}`);
            }
        }

        // 2. Check if binary is in PATH
        try {
            const command = process.platform === 'win32' ? 'where' : 'which';
            execSync(`${command} ${binaryName}`, { stdio: 'ignore' });
            this.outputChannel.appendLine(`[BinaryManager] Found ${binaryName} in PATH`);
            return binaryName;
        } catch {
            this.outputChannel.appendLine(`[BinaryManager] ${binaryName} not found in PATH`);
        }

        // 3. Check if already downloaded/extracted
        const localPath = path.join(this.getBinaryDir(), binaryName + (process.platform === 'win32' ? '.exe' : ''));
        if (fs.existsSync(localPath)) {
            this.outputChannel.appendLine(`[BinaryManager] Found cached binary: ${localPath}`);
            
            // Check if version is outdated
            const metadata = this.readVersionMetadata();
            if (metadata && this.isVersionOutdated(metadata)) {
                // Offer to update
                const choice = await vscode.window.showInformationMessage(
                    `A newer version of Patto binaries is available (${BINARY_VERSION}). Update now?`,
                    'Update',
                    'Skip',
                    'Always Skip'
                );

                if (choice === 'Update') {
                    // Delete old binaries
                    this.cleanupBinaries();
                    // Download new version
                    const downloaded = await this.downloadBinary(binaryName);
                    if (downloaded) {
                        vscode.window.showInformationMessage(`Patto binaries updated to ${BINARY_VERSION}!`);
                        return downloaded;
                    }
                } else if (choice === 'Always Skip') {
                    // Update lastCheckedAt to skip for next 24 hours
                    metadata.lastCheckedAt = Date.now();
                    this.writeVersionMetadata(metadata);
                } else {
                    // Skip this time
                    metadata.lastCheckedAt = Date.now();
                    this.writeVersionMetadata(metadata);
                }
            }
            
            return localPath;
        }

        // 4. Ask user if they want to download (only once for both binaries)
        const choice = await vscode.window.showInformationMessage(
            `Patto binaries are required but not found. Download from GitHub releases?`,
            'Download',
            'Install Manually',
            'Configure Path'
        );

        if (choice === 'Download') {
            const downloaded = await this.downloadBinary(binaryName);
            if (downloaded) {
                vscode.window.showInformationMessage(`Patto binaries downloaded successfully!`);
                return downloaded;
            } else {
                vscode.window.showErrorMessage(`Failed to download binaries. Please install manually.`);
                return null;
            }
        } else if (choice === 'Configure Path') {
            vscode.commands.executeCommand('workbench.action.openSettings', `patto.${binaryName === 'patto-lsp' ? 'lspPath' : 'previewPath'}`);
            return null;
        } else {
            // Install Manually
            vscode.window.showInformationMessage(
                `To install manually:\n\ncargo install --git https://github.com/${GITHUB_REPO} --bin ${binaryName}`,
                'Copy Command'
            ).then(selection => {
                if (selection === 'Copy Command') {
                    vscode.env.clipboard.writeText(`cargo install --git https://github.com/${GITHUB_REPO} --bin ${binaryName}`);
                }
            });
            return null;
        }
    }

    private cleanupBinaries(): void {
        const binDir = this.getBinaryDir();
        if (fs.existsSync(binDir)) {
            const files = fs.readdirSync(binDir);
            for (const file of files) {
                const filePath = path.join(binDir, file);
                if (fs.statSync(filePath).isFile()) {
                    fs.unlinkSync(filePath);
                    this.outputChannel.appendLine(`[BinaryManager] Deleted old binary: ${filePath}`);
                }
            }
        }
    }
}
