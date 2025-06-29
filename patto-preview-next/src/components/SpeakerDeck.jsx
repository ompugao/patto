import React, { useEffect, useRef, useState } from 'react';

export default function SpeakerDeck({ url, id }) {
  const [embedHtml, setEmbedHtml] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const containerRef = useRef(null);

  useEffect(() => {
    // If we have a direct hash ID, create embed HTML directly
    if (id) {
      const embedScript = `<script async class="speakerdeck-embed" data-id="${id}" data-ratio="1.33333333333333" src="//speakerdeck.com/assets/embed.js"></script>`;
      setEmbedHtml(embedScript);
      setLoading(false);
      return;
    }

    // Otherwise fetch from our API
    const fetchEmbed = async () => {
      try {
        setLoading(true);
        setError(null);
        
        const response = await fetch(`/api/speakerdeck-embed?url=${encodeURIComponent(url)}`);
        
        if (!response.ok) {
          throw new Error('Failed to fetch SpeakerDeck embed');
        }
        
        const data = await response.json();
        
        if (data.html) {
          setEmbedHtml(data.html);
        } else {
          throw new Error('No embed HTML received');
        }
      } catch (err) {
        console.error('SpeakerDeck embed error:', err);
        setError(err.message);
      } finally {
        setLoading(false);
      }
    };

    if (url) {
      fetchEmbed();
    }
  }, [url, id]);

  useEffect(() => {
    if (embedHtml && containerRef.current) {
      // Clear previous content
      containerRef.current.innerHTML = '';
      
      // Insert the embed HTML
      containerRef.current.innerHTML = embedHtml;
      
      // Execute any scripts in the embed HTML
      const scripts = containerRef.current.querySelectorAll('script');
      scripts.forEach(script => {
        const newScript = document.createElement('script');
        if (script.src) {
          newScript.src = script.src;
          newScript.async = script.async;
        } else {
          newScript.textContent = script.textContent;
        }
        // Copy attributes
        Array.from(script.attributes).forEach(attr => {
          if (attr.name !== 'src') {
            newScript.setAttribute(attr.name, attr.value);
          }
        });
        script.parentNode.replaceChild(newScript, script);
      });
    }
  }, [embedHtml]);

  if (loading) {
    return (
      <div style={{ 
        padding: '20px', 
        textAlign: 'center', 
        border: '1px solid #ddd', 
        borderRadius: '4px',
        backgroundColor: '#f9f9f9'
      }}>
        Loading SpeakerDeck presentation...
      </div>
    );
  }

  if (error) {
    return (
      <div style={{ 
        padding: '20px', 
        textAlign: 'center', 
        border: '1px solid #f5c6cb', 
        borderRadius: '4px',
        backgroundColor: '#f8d7da',
        color: '#721c24'
      }}>
        Error loading SpeakerDeck presentation: {error}
        <br />
        <a href={url} target="_blank" rel="noopener noreferrer" style={{ color: '#721c24' }}>
          View on SpeakerDeck
        </a>
      </div>
    );
  }

  return (
    <div 
      ref={containerRef} 
      style={{ 
        margin: '0',
        border: '1px solid #ddd',
        borderRadius: '4px',
        overflow: 'hidden'
      }} 
    />
  );
}

export function extractSpeakerDeckId(url) {
  // SpeakerDeck URLs are in format: https://speakerdeck.com/username/presentation-title
  // We'll return the full URL since SpeakerDeck embed needs the complete URL
  const match = url.match(/https:\/\/speakerdeck\.com\/[^\/]+\/[^\/\?]+/);
  return match ? match[0] : null;
}
