import styles from './BackLinks.module.css';

export default function BackLinks({ currentNote, onSelectFile, backLinks = [] }) {
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

  return (
    <div className={styles.container}>
      <h3 className={styles.title}>Back-Links</h3>
      <div className={styles.description}>
        Notes that reference this page ({backLinks.length})
      </div>
      
      <div className={styles.backLinksList}>
        {backLinks.map((backLink, index) => (
          <div key={index} className={styles.backLinkItem}>
            <button
              className={styles.backLink}
              onClick={() => onSelectFile(backLink.source_file + ".pn")}
              title={`View ${backLink.source_file}`}
            >
              {backLink.source_file}
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
