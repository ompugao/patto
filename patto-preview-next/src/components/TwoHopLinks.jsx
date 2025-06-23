import { useState, useEffect } from 'react';
import styles from './TwoHopLinks.module.css';

export default function TwoHopLinks({ currentNote, onSelectFile }) {
  const [twoHopLinks, setTwoHopLinks] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);

  useEffect(() => {
    if (!currentNote) {
      setTwoHopLinks([]);
      return;
    }

    const fetchTwoHopLinks = async () => {
      setLoading(true);
      setError(null);
      
      try {
        const response = await fetch(`/api/two-hop-links/${encodeURIComponent(currentNote)}`);
        if (!response.ok) {
          throw new Error('Failed to fetch two-hop links');
        }
        
        const data = await response.json();
        setTwoHopLinks(data.twoHopLinks || []);
      } catch (err) {
        console.error('Error fetching two-hop links:', err);
        setError(err.message);
        setTwoHopLinks([]);
      } finally {
        setLoading(false);
      }
    };

    fetchTwoHopLinks();
  }, [currentNote]);

  if (!currentNote) {
    return null;
  }

  if (loading) {
    return (
      <div className={styles.container}>
        <h3 className={styles.title}>Two-Hop Linked Pages</h3>
        <div className={styles.loading}>Loading...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className={styles.container}>
        <h3 className={styles.title}>Two-Hop Linked Pages</h3>
        <div className={styles.error}>Error: {error}</div>
      </div>
    );
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
