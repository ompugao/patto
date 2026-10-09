import * as assert from 'assert';
import { archiveName, archiveUrl, executableName, releaseTarget } from '../binaries/platform';
import { staleness, VERSION_CHECK_INTERVAL_MS, type VersionMetadata } from '../binaries/versionMetadata';

describe('releaseTarget', () => {
	it('names the release asset by Rust target triple', () => {
		const linux = releaseTarget('linux', 'x64')!;
		assert.strictEqual(archiveName(linux), 'patto-x86_64-unknown-linux-gnu.tar.xz');
		assert.strictEqual(archiveName(releaseTarget('darwin', 'arm64')!), 'patto-aarch64-apple-darwin.tar.xz');
		assert.strictEqual(archiveUrl(linux, '0.6.0'), 'https://github.com/ompugao/patto/releases/download/v0.6.0/patto-x86_64-unknown-linux-gnu.tar.xz');
	});

	it('uses a zip and .exe suffix on Windows', () => {
		const windows = releaseTarget('win32', 'x64')!;
		assert.strictEqual(archiveName(windows), 'patto-x86_64-pc-windows-msvc.zip');
		assert.strictEqual(windows.exeExt, '.exe');
		assert.strictEqual(executableName('patto-lsp', 'win32'), 'patto-lsp.exe');
		assert.strictEqual(executableName('patto-lsp', 'linux'), 'patto-lsp');
	});

	it('has no asset for an unsupported platform or architecture', () => {
		assert.strictEqual(releaseTarget('freebsd', 'x64'), null);
		assert.strictEqual(releaseTarget('linux', 'ia32'), null);
	});
});

describe('staleness', () => {
	const now = 1_700_000_000_000;
	const metadata = (overrides: Partial<VersionMetadata>): VersionMetadata =>
		({ version: '0.6.0', downloadedAt: now, lastCheckedAt: now, ...overrides });

	it('is fresh right after a download of the expected version', () => {
		assert.strictEqual(staleness(metadata({}), '0.6.0', now), null);
	});

	it('reports a version mismatch before anything else', () => {
		assert.strictEqual(staleness(metadata({ version: '0.5.0', lastCheckedAt: 0 }), '0.6.0', now), 'version-mismatch');
	});

	it('expires once the check interval has passed', () => {
		assert.strictEqual(staleness(metadata({}), '0.6.0', now + VERSION_CHECK_INTERVAL_MS), null);
		assert.strictEqual(staleness(metadata({}), '0.6.0', now + VERSION_CHECK_INTERVAL_MS + 1), 'check-interval-expired');
	});
});
