import styles from './BackLinks.module.css';
import { useState } from 'react';

export default function BackLinks({ currentNote, onSelectFile, backLinks = [] }) {
  const [expandedFiles, setExpandedFiles] = useState(new Set());

  if (!currentNote) {
    return null;
  }

  if (backLinks.length === 0) {
    return (
      <div className={styles.container}>
        <h3 className={styles.title}>Back-Links</h3>
        <div className={styles.empty}>No notes reference this page.</div>
      </div>
    );
  }

  const toggleExpanded = (fileName) => {
    setExpandedFiles(prev => {
      const newSet = new Set(prev);
      if (newSet.has(fileName)) {
        newSet.delete(fileName);
      } else {
        newSet.add(fileName);
      }
      return newSet;
    });
  };

  const totalLinks = backLinks.reduce((sum, bl) => sum + bl.locations.length, 0);

  return (
    <div className={styles.container}>
      <h3 className={styles.title}>Back-Links</h3>
      <div className={styles.description}>
        {totalLinks} reference{totalLinks !== 1 ? 's' : ''} from {backLinks.length} note{backLinks.length !== 1 ? 's' : ''}
      </div>
      
      <div className={styles.backLinksList}>
        {backLinks.map((backLink, index) => {
          const isExpanded = expandedFiles.has(backLink.source_file);
          const multipleLocations = backLink.locations.length > 1;
          
          return (
            <div key={index} className={styles.backLinkItem}>
              <div className={styles.backLinkHeader}>
                <button
                  className={styles.backLink}
                  onClick={() => onSelectFile(backLink.source_file + ".pn")}
                  title={`View ${backLink.source_file}`}
                >
                  {backLink.source_file}
                  {multipleLocations && (
                    <span className={styles.linkCount}>
                      ({backLink.locations.length})
                    </span>
                  )}
                </button>
                
                {multipleLocations && (
                  <button
                    className={styles.expandButton}
                    onClick={() => toggleExpanded(backLink.source_file)}
                    aria-label={isExpanded ? "Collapse" : "Expand"}
                  >
                    {isExpanded ? '▼' : '▶'}
                  </button>
                )}
              </div>
              
              {isExpanded && (
                <div className={styles.locationsList}>
                  {backLink.locations.map((loc, locIdx) => (
                    <div key={locIdx} className={styles.locationItem}>
                      <span className={styles.lineNumber}>
                        Line {loc.line + 1}
                      </span>
                      {loc.target_anchor && (
                        <span className={styles.anchor}>
                          → #{loc.target_anchor}
                        </span>
                      )}
                      {loc.context && (
                        <div className={styles.context}>
                          {loc.context}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}