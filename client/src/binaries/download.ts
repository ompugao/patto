import * as fs from 'fs';
import * as https from 'https';
import { execSync } from 'child_process';

export function downloadFile(url: string, destination: string): Promise<void> {
    return new Promise((resolve, reject) => {
        const file = fs.createWriteStream(destination);
        const fail = (err: Error) => {
            fs.unlinkSync(destination);
            reject(err);
        };
        const save = (response: NodeJS.ReadableStream) => {
            response.pipe(file);
            file.on('finish', () => {
                file.close();
                resolve();
            });
        };

        https.get(url, (response) => {
            if (response.statusCode === 302 || response.statusCode === 301) {
                if (response.headers.location) {
                    https.get(response.headers.location, save).on('error', fail);
                }
            } else if (response.statusCode === 200) {
                save(response);
            } else {
                file.close();
                fail(new Error(`Failed to download: ${response.statusCode}`));
            }
        }).on('error', fail);
    });
}

export function extractArchive(archivePath: string, destDir: string, isWindows: boolean): void {
    try {
        if (isWindows) {
            execSync(`powershell -command "Expand-Archive -Path '${archivePath}' -DestinationPath '${destDir}' -Force"`, { stdio: 'ignore' });
        } else {
            // The tarball wraps the binaries in a top-level directory
            execSync(`tar -xf "${archivePath}" -C "${destDir}" --strip-components=1`, { stdio: 'ignore' });
        }
    } catch (error) {
        throw new Error(`Failed to extract archive: ${error}`);
    }
}

export function isOnPath(command: string): boolean {
    try {
        const lookup = process.platform === 'win32' ? 'where' : 'which';
        execSync(`${lookup} ${command}`, { stdio: 'ignore' });
        return true;
    } catch {
        return false;
    }
}

export function isExecutable(filePath: string): boolean {
    try {
        fs.accessSync(filePath, fs.constants.X_OK);
        return true;
    } catch {
        return false;
    }
}
