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
        {backLinks.map((linkName, index) => (
          <button
            key={index}
            className={styles.backLink}
            onClick={() => onSelectFile(linkName + ".pn")}
            title={`View ${linkName}`}
          >
            {linkName}
          </button>
        ))}
      </div>
    </div>
  );
}