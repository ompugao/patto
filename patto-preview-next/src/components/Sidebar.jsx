import { useState } from 'react';
import styles from './Sidebar.module.css';

export default function Sidebar({
  files,
  fileMetadata,
  currentFile,
  onSelectFile,
  sortBy,
  onSortChange,
  collapsed,
  onToggle,
}) {
  const [searchTerm, setSearchTerm] = useState('');

  // Fuzzy search algorithm
  const fuzzyMatch = (text, searchTerm) => {
    if (!searchTerm) return true;
    
    const textLower = text.toLowerCase();
    const searchLower = searchTerm.toLowerCase();
    
    let searchIndex = 0;
    for (let i = 0; i < textLower.length && searchIndex < searchLower.length; i++) {
      if (textLower[i] === searchLower[searchIndex]) {
        searchIndex++;
      }
    }
    
    return searchIndex === searchLower.length;
  };

  // File list sorting
  const sortedFiles = [...files];
  switch (sortBy) {
    case "title":
      sortedFiles.sort();
      break;
    case "modified":
      sortedFiles.sort(
        (a, b) =>
          (fileMetadata[b]?.modified || 0) -
          (fileMetadata[a]?.modified || 0)
      );
      break;
    case "created":
      sortedFiles.sort(
        (a, b) =>
          (fileMetadata[b]?.created || 0) -
          (fileMetadata[a]?.created || 0)
      );
      break;
    case "linked":
      sortedFiles.sort(
        (a, b) =>
          (fileMetadata[b]?.linkCount || 0) -
          (fileMetadata[a]?.linkCount || 0)
      );
      break;
    default:
      break;
  }

  // Apply fuzzy search filter
  const filteredFiles = sortedFiles.filter(file => fuzzyMatch(file, searchTerm));

  return (
    <>
      <button
        className={styles.SidebarToggle}
        id="sidebar-toggle"
        onClick={onToggle}
        style={{
          position: 'fixed',
          bottom: '20px',
          left: '15px',
          background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
          color: 'white',
          border: 'none',
          padding: '12px 10px',
          cursor: 'pointer',
          borderRadius: '8px',
          boxShadow: '0 4px 12px rgba(0, 0, 0, 0.15)',
          zIndex: 1000,
          transition: 'all 0.3s ease',
          fontSize: '16px',
          width: '44px',
          height: '44px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center'
        }}
      >☰</button>
      <div
        className={styles.Sidebar}
        id="sidebar"
        style={{
          width: collapsed ? 250 : 250,
          overflowY: "auto",
          transition: "margin-left 0.5s ease, padding-left 0.5s, padding-right 0.5s",
          marginLeft: collapsed ? '-250px' : '0',
          backgroundColor: '#f5f5f5',
          paddingTop: '15px',
          paddingLeft: collapsed ? 0 : '15px',
          paddingRight: collapsed ? 0 : '15px',
          borderRight: '1px solid #e0e0e0',
        }}
      >
        <div style={{ padding: "6px 4px" }}>
          <label htmlFor="sort-select" style={{ fontSize: 12, marginRight: 6 }}>
            Sort by:
          </label>
          <select
            id="sort-select"
            style={{ fontSize: 12, padding: 0 }}
            value={sortBy}
            onChange={e => onSortChange(e.target.value)}
          >
            <option value="modified">Most Recently Modified</option>
            <option value="created">Most Recently Created</option>
            <option value="linked">Most Linked</option>
            <option value="title">Title</option>
          </select>
        </div>
        <div style={{ marginBottom: 0 }}>
          <input
            type="text"
            placeholder="Search..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            style={{
              width: '100%',
              padding: '4px 8px',
              fontSize: 12,
              border: '1px solid #ccc',
              borderRadius: '4px',
              boxSizing: 'border-box'
            }}
          />
        </div>
        <ul id="file-list" style={{ listStyle: "none", padding: 0, marginTop: "5px"}}>
          {filteredFiles.map((file) => (
            <li
              key={file}
              className={file === currentFile ? "active" : ""}
              style={{
                background: file === currentFile ? "#ddeeff" : undefined,
                fontWeight: file === currentFile ? "bold" : undefined,
                borderBottom: "1px solid #e0e0e0",
              }}
            >
              <a
                href={`/?note=${encodeURIComponent(file)}`}
                onClick={(e) => {
                  e.preventDefault();
                  onSelectFile(file);
                }}
                style={{
                  display: "block",
                  padding: "6px 4px",
                  textDecoration: "none",
                  color: "inherit",
                  cursor: "pointer",
                }}
              >
                {file}
              </a>
            </li>
          ))}
        </ul>
      </div>
    </>
  );
}
