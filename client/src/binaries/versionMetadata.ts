import * as fs from 'fs';
import * as path from 'path';

export const BINARY_VERSION = '0.6.0'; // Should match Cargo.toml version
export const VERSION_CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000;

export interface VersionMetadata {
    version: string;
    downloadedAt: number;
    lastCheckedAt: number;
}

export type Staleness = 'version-mismatch' | 'check-interval-expired';

export function staleness(
    metadata: VersionMetadata,
    expectedVersion: string = BINARY_VERSION,
    now: number = Date.now(),
): Staleness | null {
    if (metadata.version !== expectedVersion) {
        return 'version-mismatch';
    }
    if (now - metadata.lastCheckedAt > VERSION_CHECK_INTERVAL_MS) {
        return 'check-interval-expired';
    }
    return null;
}

export function freshMetadata(now: number = Date.now()): VersionMetadata {
    return { version: BINARY_VERSION, downloadedAt: now, lastCheckedAt: now };
}

export class VersionMetadataFile {
    constructor(private readonly filePath: string, private readonly log: (line: string) => void) {}

    read(): VersionMetadata | null {
        if (!fs.existsSync(this.filePath)) {
            return null;
        }
        try {
            return JSON.parse(fs.readFileSync(this.filePath, 'utf-8'));
        } catch (error) {
            this.log(`Failed to read version metadata: ${error}`);
            return null;
        }
    }

    write(metadata: VersionMetadata): void {
        fs.mkdirSync(path.dirname(this.filePath), { recursive: true });
        fs.writeFileSync(this.filePath, JSON.stringify(metadata, null, 2));
    }
}
