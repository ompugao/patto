// GitConfig component - configure git credentials, workspace, and remote
// Platform-aware: Android shows remote URL only, Desktop shows local path

import { useState, useEffect } from 'react';
import { useStore, View } from '../lib/store';
import { invoke } from '@tauri-apps/api/core';
import { appDataDir } from '@tauri-apps/api/path';
import { type } from '@tauri-apps/plugin-os';
import { open } from '@tauri-apps/plugin-dialog';
import './GitConfig.css';

export function GitConfig() {
    const {
        workspacePath,
        setWorkspacePath,
        gitCredentials,
        setGitCredentials,
        gitStatus,
        loadGitStatus,
        setView,
        loadFiles,
    } = useStore();

    const [platform, setPlatform] = useState('unknown'); // 'android', 'ios', or desktop variants
    const [username, setUsername] = useState(gitCredentials.username);
    const [token, setToken] = useState(gitCredentials.token);
    const [manualPath, setManualPath] = useState(workspacePath || '');
    const [remoteUrl, setRemoteUrl] = useState('');
    const [isCloning, setIsCloning] = useState(false);
    const [statusMessage, setStatusMessage] = useState('');

    const isMobile = platform === 'android' || platform === 'ios';

    // Detect platform on mount
    useEffect(() => {
        const detectPlatform = async () => {
            try {
                const osType = await type();
                setPlatform(osType);

                // Auto-set app data path on mobile
                if (osType === 'android' || osType === 'ios') {
                    const appDir = await appDataDir();
                    const basePath = appDir.endsWith('/') ? appDir : appDir + '/';
                    setManualPath(basePath + 'notes');
                }
            } catch (err) {
                console.error('Failed to detect platform:', err);
                setPlatform('unknown');
            }
        };
        detectPlatform();
    }, []);

    const handleSelectWorkspace = async () => {
        try {
            const selected = await open({
                directory: true,
                multiple: false,
                title: 'Select Patto Workspace',
            });

            if (selected) {
                setWorkspacePath(selected);
                setManualPath(selected);
                await loadFiles();
                await loadGitStatus();
                setStatusMessage('Workspace set');
            }
        } catch (err) {
            console.error('Failed to select directory:', err);
            setStatusMessage('Browse not available on this platform');
        }
    };

    const handleSetPath = async () => {
        if (manualPath.trim()) {
            setWorkspacePath(manualPath.trim());
            await loadFiles();
            await loadGitStatus();
            setStatusMessage('Workspace set: ' + manualPath.trim());
        }
    };

    const handleClone = async () => {
        if (!manualPath.trim() || !remoteUrl.trim()) {
            setStatusMessage('Remote URL is required');
            return;
        }

        if (!username || !token) {
            setStatusMessage('Git credentials required for cloning');
            return;
        }

        setIsCloning(true);
        setStatusMessage('Starting clone...');

        // Listen for progress events
        const { listen } = await import('@tauri-apps/api/event');
        const unlisten = await listen('clone-progress', (event) => {
            const { stage, percent, received, total } = event.payload;
            if (total > 0) {
                setStatusMessage(`${stage}: ${received}/${total} (${percent}%)`);
            } else {
                setStatusMessage(stage);
            }
        });

        try {
            await invoke('git_clone', {
                url: remoteUrl.trim(),
                dest: manualPath.trim(),
                credentials: { username, token },
            });
            setWorkspacePath(manualPath.trim());
            await loadFiles();
            await loadGitStatus();
            setStatusMessage('Clone successful!');
            setView(View.FILE_LIST);
        } catch (err) {
            console.error('Clone failed:', err);
            setStatusMessage('Clone failed: ' + err);
        } finally {
            unlisten();
            setIsCloning(false);
        }
    };

    const handleSaveCredentials = () => {
        setGitCredentials({ username, token });
        setStatusMessage('Credentials saved');
    };

    return (
        <div className="git-config">
            <header className="config-header">
                <button className="back-btn" onClick={() => setView(View.FILE_LIST)}>
                    ← Back
                </button>
                <h1>Settings</h1>
                <div style={{ width: 60 }} />
            </header>

            <div className="config-content">
                {statusMessage && (
                    <div className="status-message">{statusMessage}</div>
                )}

                {/* MOBILE: Show remote URL for cloning */}
                {isMobile && (
                    <section className="config-section">
                        <h2>Remote Repository</h2>
                        <p className="hint">Enter your notes repository URL to clone</p>

                        <label className="input-group">
                            <span className="label">Remote URL</span>
                            <input
                                type="text"
                                value={remoteUrl}
                                onChange={(e) => setRemoteUrl(e.target.value)}
                                placeholder="https://github.com/user/notes.git"
                            />
                        </label>

                        <button
                            className="primary-btn"
                            onClick={handleClone}
                            disabled={isCloning}
                        >
                            {isCloning ? 'Cloning...' : 'Clone Repository'}
                        </button>
                    </section>
                )}

                {/* DESKTOP: Show local path selector */}
                {!isMobile && (
                    <section className="config-section">
                        <h2>Workspace</h2>
                        <p className="hint">Select your local notes folder</p>

                        <label className="input-group">
                            <span className="label">Path</span>
                            <input
                                type="text"
                                value={manualPath}
                                onChange={(e) => setManualPath(e.target.value)}
                                placeholder="/path/to/notes"
                            />
                        </label>

                        <div className="button-row">
                            <button
                                className="secondary-btn"
                                onClick={handleSelectWorkspace}
                            >
                                Browse
                            </button>
                            <button
                                className="primary-btn"
                                onClick={handleSetPath}
                            >
                                Set Path
                            </button>
                        </div>
                    </section>
                )}

                {/* Credentials - shown on both platforms but more prominent on mobile */}
                <section className="config-section">
                    <h2>Git Credentials</h2>
                    <p className="hint">{isMobile ? 'Required for cloning' : 'For push/pull operations'}</p>

                    <label className="input-group">
                        <span className="label">Username</span>
                        <input
                            type="text"
                            value={username}
                            onChange={(e) => setUsername(e.target.value)}
                            placeholder="github_username"
                        />
                    </label>

                    <label className="input-group">
                        <span className="label">Personal Access Token</span>
                        <input
                            type="password"
                            value={token}
                            onChange={(e) => setToken(e.target.value)}
                            placeholder="ghp_xxxxxxxxxxxx"
                        />
                    </label>

                    <button
                        className="primary-btn"
                        onClick={handleSaveCredentials}
                    >
                        Save Credentials
                    </button>
                </section>

                {/* Git status - desktop only */}
                {!isMobile && gitStatus && (
                    <section className="config-section">
                        <h2>Git Status</h2>
                        <div className="status-info">
                            <span className="status-row">
                                <span className="label">Branch:</span>
                                <span className="value">{gitStatus.branch || 'n/a'}</span>
                            </span>
                            <span className="status-row">
                                <span className="label">Modified:</span>
                                <span className="value">{gitStatus.modified}</span>
                            </span>
                            <span className="status-row">
                                <span className="label">Untracked:</span>
                                <span className="value">{gitStatus.untracked}</span>
                            </span>
                            {gitStatus.isClean && (
                                <span className="clean-badge">✓ Working tree clean</span>
                            )}
                        </div>
                    </section>
                )}

                {/* Debug: show platform */}
                <div className="platform-info">
                    Platform: {platform}
                </div>
            </div>
        </div>
    );
}
