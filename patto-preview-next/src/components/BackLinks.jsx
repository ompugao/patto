import { useState, useEffect } from 'react';
import styles from './BackLinks.module.css';

export default function BackLinks({ currentNote, onSelectFile }) {
  const [backLinks, setBackLinks] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);

  useEffect(() => {
    if (!currentNote) {
      setBackLinks([]);
      return;
    }

    const fetchBackLinks = async () => {
      setLoading(true);
      setError(null);
      
      try {
        const response = await fetch(`/api/back-links/${encodeURIComponent(currentNote)}`);
        if (!response.ok) {
          throw new Error('Failed to fetch back-links');
        }
        
        const data = await response.json();
        setBackLinks(data.backLinks || []);
      } catch (err) {
        console.error('Error fetching back-links:', err);
        setError(err.message);
        setBackLinks([]);
      } finally {
        setLoading(false);
      }
    };

    fetchBackLinks();
  }, [currentNote]);

  if (!currentNote) {
    return null;
  }

  if (loading) {
    return (
      <div className={styles.container}>
        <h3 className={styles.title}>Back-Links</h3>
        <div className={styles.loading}>Loading...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className={styles.container}>
        <h3 className={styles.title}>Back-Links</h3>
        <div className={styles.error}>Error: {error}</div>
      </div>
    );
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