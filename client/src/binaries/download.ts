import * as fs from 'fs';
import * as https from 'https';
import { execSync } from 'child_process';

const MAX_REDIRECTS = 5;

export function downloadFile(url: string, destination: string): Promise<void> {
    return new Promise((resolve, reject) => {
        const file = fs.createWriteStream(destination);
        const fail = (err: Error) => {
            file.close();
            fs.rmSync(destination, { force: true });
            reject(err);
        };

        const get = (from: string, redirectsLeft: number) => {
            https.get(from, (response) => {
                const { statusCode, headers } = response;
                if (statusCode === 301 || statusCode === 302) {
                    response.resume();
                    if (!headers.location || redirectsLeft === 0) {
                        fail(new Error(`Failed to download: redirect from ${from} could not be followed`));
                    } else {
                        get(headers.location, redirectsLeft - 1);
                    }
                } else if (statusCode === 200) {
                    response.pipe(file);
                    file.on('finish', () => {
                        file.close();
                        resolve();
                    });
                } else {
                    response.resume();
                    fail(new Error(`Failed to download: ${statusCode}`));
                }
            }).on('error', fail);
        };
        get(url, MAX_REDIRECTS);
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
