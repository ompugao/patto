// GitConfig component - configure git credentials and workspace

import { useState } from 'react';
import { useStore, View } from '../lib/store';
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

    const handleSelectWorkspace = async () => {
        try {
            const selected = await open({
                directory: true,
                multiple: false,
                title: 'Select Patto Workspace',
            });

            if (selected) {
                setWorkspacePath(selected);
                await loadFiles();
                await loadGitStatus();
            }
        } catch (err) {
            console.error('Failed to select directory:', err);
        }
    };

    const handleSaveCredentials = () => {
        setGitCredentials({ username, token });
        setView(View.FILE_LIST);
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
                <section className="config-section">
                    <h2>Workspace</h2>
                    <div className="workspace-path">
                        {workspacePath ? (
                            <span className="path">{workspacePath}</span>
                        ) : (
                            <span className="no-path">No workspace selected</span>
                        )}
                    </div>
                    <button
                        className="primary-btn"
                        onClick={handleSelectWorkspace}
                    >
                        Select Workspace
                    </button>
                </section>

                <section className="config-section">
                    <h2>Git Credentials (HTTPS)</h2>
                    <p className="hint">For syncing with GitHub/GitLab via HTTPS</p>

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
