// FileList component - displays patto files with sorting

import { useStore, SortBy, View } from '../lib/store';
import './FileList.css';

export function FileList() {
    const {
        files,
        sortBy,
        setSortBy,
        openNote,
        isLoadingFiles,
        setView,
        gitSync,
        isGitSyncing,
    } = useStore();

    const formatDate = (timestamp) => {
        if (!timestamp) return '';
        const date = new Date(timestamp * 1000);
        return date.toLocaleDateString('en-US', {
            month: 'short',
            day: 'numeric',
            year: date.getFullYear() !== new Date().getFullYear() ? 'numeric' : undefined
        });
    };

    return (
        <div className="file-list">
            <header className="file-list-header">
                <h1>Notes</h1>
                <div className="header-actions">
                    <button
                        onClick={gitSync}
                        disabled={isGitSyncing}
                        className="icon-btn"
                        title="Sync with Git"
                    >
                        {isGitSyncing ? '⟳' : '↻'}
                    </button>
                    <button
                        onClick={() => setView(View.TASKS)}
                        className="icon-btn"
                        title="View Tasks"
                    >
                        ☑
                    </button>
                    <button
                        onClick={() => setView(View.GIT_CONFIG)}
                        className="icon-btn"
                        title="Git Settings"
                    >
                        ⚙
                    </button>
                </div>
            </header>

            <div className="sort-controls">
                {Object.entries({
                    [SortBy.LAST_MODIFIED]: 'Modified',
                    [SortBy.LAST_CREATED]: 'Created',
                    [SortBy.MOST_LINKED]: 'Links',
                    [SortBy.ALPHABETICAL]: 'A-Z',
                }).map(([key, label]) => (
                    <button
                        key={key}
                        className={`sort-btn ${sortBy === key ? 'active' : ''}`}
                        onClick={() => setSortBy(key)}
                    >
                        {label}
                    </button>
                ))}
            </div>

            {isLoadingFiles ? (
                <div className="loading">Loading...</div>
            ) : (
                <ul className="files">
                    {files.map((file) => (
                        <li
                            key={file.path}
                            className="file-item"
                            onClick={() => openNote(file.path)}
                        >
                            <span className="file-name">{file.name}</span>
                            <span className="file-meta">
                                {sortBy === SortBy.MOST_LINKED && file.backlinkCount > 0 && (
                                    <span className="backlinks">{file.backlinkCount} links</span>
                                )}
                                <span className="date">{formatDate(file.modifiedTime)}</span>
                            </span>
                        </li>
                    ))}
                </ul>
            )}
        </div>
    );
}
