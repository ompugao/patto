export const GITHUB_REPO = 'ompugao/patto';

export type BinaryName = 'patto-lsp' | 'patto-preview';

export const BINARY_NAMES: BinaryName[] = ['patto-lsp', 'patto-preview'];

export interface ReleaseTarget {
    /** Rust target triple, as the release assets are named. */
    triple: string;
    exeExt: '' | '.exe';
    archiveExt: '.zip' | '.tar.xz';
    isWindows: boolean;
}

const PLATFORM_TRIPLE: { [platform: string]: string } = {
    darwin: 'apple-darwin',
    linux: 'unknown-linux-gnu',
    win32: 'pc-windows-msvc',
};

const ARCH_TRIPLE: { [arch: string]: string } = {
    x64: 'x86_64',
    arm64: 'aarch64',
};

export function releaseTarget(platform: string = process.platform, arch: string = process.arch): ReleaseTarget | null {
    const platformPart = PLATFORM_TRIPLE[platform];
    const archPart = ARCH_TRIPLE[arch];
    if (!platformPart || !archPart) {
        return null;
    }
    const isWindows = platform === 'win32';
    return {
        triple: `${archPart}-${platformPart}`,
        exeExt: isWindows ? '.exe' : '',
        archiveExt: isWindows ? '.zip' : '.tar.xz',
        isWindows,
    };
}

export function archiveName(target: ReleaseTarget): string {
    return `patto-${target.triple}${target.archiveExt}`;
}

export function archiveUrl(target: ReleaseTarget, version: string): string {
    return `https://github.com/${GITHUB_REPO}/releases/download/v${version}/${archiveName(target)}`;
}

export function executableName(name: BinaryName, platform: string = process.platform): string {
    return platform === 'win32' ? `${name}.exe` : name;
}

export function manualInstallCommand(name: BinaryName): string {
    return `cargo install --git https://github.com/${GITHUB_REPO} --bin ${name}`;
}
