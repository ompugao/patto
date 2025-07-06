import styles from './TwoHopLinks.module.css';

export default function TwoHopLinks({ currentNote, onSelectFile, twoHopLinks = [] }) {

  if (!currentNote) {
    return null;
  }

  if (twoHopLinks.length === 0) {
    return (
      <div className={styles.container}>
        <h3 className={styles.title}>Two-Hop Linked Pages</h3>
        <div className={styles.empty}>No two-hop linked pages found.</div>
      </div>
    );
  }

  return (
    <div className={styles.container}>
      <h3 className={styles.title}>Two-Hop Linked Pages</h3>
      <div className={styles.description}>
        Pages that link to the same pages as this note
      </div>
      
      {twoHopLinks.map(([bridgePage, connectedPages], index) => (
        <div key={index} className={styles.group}>
          <div className={styles.bridgeTitle}>
            <span className={styles.bridgeLabel}>via</span>
            <button
              className={styles.bridgeLink}
              onClick={() => onSelectFile(bridgePage + ".pn")}
              title={`View ${bridgePage}`}
            >
              {bridgePage}
            </button>
            <span className={styles.connectionCount}>({connectedPages.length})</span>
          </div>
          
          <div className={styles.connectedPages}>
            {connectedPages.map((page, pageIndex) => (
              <button
                key={pageIndex}
                className={styles.pageLink}
                onClick={() => onSelectFile(page + ".pn")}
                title={`View ${page}`}
              >
                {page}
              </button>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
