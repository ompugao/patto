// GitConfig component - configure git credentials, workspace, and remote

import { useState } from 'react';
import { useStore, View } from '../lib/store';
import { invoke } from '@tauri-apps/api/core';
import { appDataDir } from '@tauri-apps/api/path';
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

    const [username, setUsername] = useState(gitCredentials.username);
    const [token, setToken] = useState(gitCredentials.token);
    const [manualPath, setManualPath] = useState(workspacePath || '');
    const [remoteUrl, setRemoteUrl] = useState('');
    const [isCloning, setIsCloning] = useState(false);
    const [statusMessage, setStatusMessage] = useState('');

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
            }
        } catch (err) {
            console.error('Failed to select directory:', err);
            setStatusMessage('Browse not available on this platform');
        }
    };

    const handleUseAppDir = async () => {
        try {
            const appDir = await appDataDir();
            // Ensure trailing slash
            const basePath = appDir.endsWith('/') ? appDir : appDir + '/';
            const notesPath = basePath + 'notes';
            setManualPath(notesPath);
            setStatusMessage(`App data path: ${notesPath}`);
        } catch (err) {
            console.error('Failed to get app data dir:', err);
            setStatusMessage('Error: ' + err);
        }
    };

    const handleManualPathSubmit = async () => {
        if (manualPath.trim()) {
            setWorkspacePath(manualPath.trim());
            await loadFiles();
            await loadGitStatus();
            setStatusMessage('Workspace set');
        }
    };

    const handleSaveCredentials = () => {
        setGitCredentials({ username, token });
        setStatusMessage('Credentials saved');
    };

    const handleConfigureRemote = async () => {
        if (!workspacePath || !remoteUrl.trim()) {
            setStatusMessage('Enter workspace path and remote URL first');
            return;
        }

        try {
            await invoke('configure_remote', {
                repoPath: workspacePath,
                remoteUrl: remoteUrl.trim(),
            });
            setStatusMessage('Remote origin configured');
            await loadGitStatus();
        } catch (err) {
            console.error('Failed to configure remote:', err);
            setStatusMessage('Error: ' + err);
        }
    };

    const handleClone = async () => {
        if (!manualPath.trim() || !remoteUrl.trim()) {
            setStatusMessage('Enter path and remote URL first');
            return;
        }

        setIsCloning(true);
        setStatusMessage('Cloning...');

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
        } catch (err) {
            console.error('Clone failed:', err);
            setStatusMessage('Clone failed: ' + err);
        } finally {
            setIsCloning(false);
        }
    };

    const handleInitRepo = async () => {
        if (!workspacePath) {
            setStatusMessage('Set workspace path first');
            return;
        }

        try {
            await invoke('git_init', { repoPath: workspacePath });
            await loadGitStatus();
            setStatusMessage('Git repository initialized');
        } catch (err) {
            console.error('Git init failed:', err);
            setStatusMessage('Init failed: ' + err);
        }
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

                <section className="config-section">
                    <h2>Workspace</h2>

                    <label className="input-group">
                        <span className="label">Path</span>
                        <input
                            type="text"
                            value={manualPath}
                            onChange={(e) => setManualPath(e.target.value)}
                            placeholder="/storage/emulated/0/patto-notes"
                        />
                    </label>

                    <div className="button-row">
                        <button
                            className="secondary-btn"
                            onClick={handleUseAppDir}
                        >
                            App Dir
                        </button>
                        <button
                            className="secondary-btn"
                            onClick={handleSelectWorkspace}
                        >
                            Browse
                        </button>
                        <button
                            className="primary-btn"
                            onClick={handleManualPathSubmit}
                        >
                            Set
                        </button>
                    </div>
                </section>

                <section className="config-section">
                    <h2>Git Remote</h2>
                    <p className="hint">Clone a repo or set remote for existing folder</p>

                    <label className="input-group">
                        <span className="label">Remote URL</span>
                        <input
                            type="text"
                            value={remoteUrl}
                            onChange={(e) => setRemoteUrl(e.target.value)}
                            placeholder="https://github.com/user/notes.git"
                        />
                    </label>

                    <div className="button-row">
                        <button
                            className="secondary-btn"
                            onClick={handleClone}
                            disabled={isCloning}
                        >
                            {isCloning ? 'Cloning...' : 'Clone'}
                        </button>
                        <button
                            className="secondary-btn"
                            onClick={handleInitRepo}
                        >
                            Init
                        </button>
                        <button
                            className="primary-btn"
                            onClick={handleConfigureRemote}
                        >
                            Set Remote
                        </button>
                    </div>
                </section>

                <section className="config-section">
                    <h2>Credentials (HTTPS)</h2>
                    <p className="hint">For GitHub/GitLab authentication</p>

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

                {gitStatus && (
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
            </div>
        </div>
    );
}
