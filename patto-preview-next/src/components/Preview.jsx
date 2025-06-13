import parse from 'html-react-parser';
import { useEffect, useCallback, useState, useRef } from 'react';
import styles from './Preview.module.css';

export default function Preview({ html, anchor }) {
  const [twitterEmbeds, setTwitterEmbeds] = useState({});
  const previewRef = useRef(null);

  // Enhanced anchor scrolling with retry mechanism
  const scrollToAnchor = useCallback((anchorId, attempts = 0) => {
    if (!anchorId) return;
    
    const element = document.getElementById(anchorId);
    console.log(`Anchor scroll attempt ${attempts + 1}: Looking for "${anchorId}", found:`, element);
    
    if (element) {
      console.log('Scrolling to anchor:', anchorId);
      element.scrollIntoView({ behavior: "smooth", block: "start" });
      
      // Add visual highlight
      element.style.transition = "background-color 0.3s ease";
      const originalBg = element.style.backgroundColor;
      element.style.backgroundColor = "#fff3cd";
      setTimeout(() => {
        element.style.backgroundColor = originalBg;
      }, 2000);
    } else if (attempts < 10) {
      // Retry with increasing delay
      setTimeout(() => scrollToAnchor(anchorId, attempts + 1), 100 * (attempts + 1));
    } else {
      console.log(`Anchor "${anchorId}" not found after ${attempts + 1} attempts`);
      // Debug: show available anchors
      const anchors = document.querySelectorAll('.anchor, [id]');
      console.log('Available anchors:', Array.from(anchors).map(el => el.id || el.className));
    }
  }, []);

  // Handle anchor scrolling when content or anchor changes
  useEffect(() => {
    if (anchor && html) {
      // Small delay to ensure content is rendered
      setTimeout(() => scrollToAnchor(anchor), 100);
    }
  }, [html, anchor, scrollToAnchor]);

  // Check if idiomorph is loaded
  // Function to load Twitter embed
  const loadTwitterEmbed = useCallback(async (url) => {
    console.log('=== loadTwitterEmbed called ===');
    console.log('URL:', url);
    console.log('Current twitterEmbeds state:', twitterEmbeds);
    
    // Mark as loading to prevent duplicate requests
    setTwitterEmbeds(prev => {
      console.log('Previous state:', prev);
      if (prev[url] !== undefined) {
        console.log('Already exists, skipping:', prev[url]);
        return prev; // Already loaded/loading/failed
      }
      console.log('Setting to loading state');
      return { ...prev, [url]: 'loading' };
    });
    
    try {
      console.log('Making fetch request to:', `/api/twitter-embed?url=${encodeURIComponent(url)}`);
      const response = await fetch(`/api/twitter-embed?url=${encodeURIComponent(url)}`);
      console.log('Twitter embed response status:', response.status);
      console.log('Response headers:', response.headers);
      
      if (response.ok) {
        const data = await response.json();
        console.log('Twitter embed data received:', data);
        console.log('Setting embed HTML:', data.html);
        setTwitterEmbeds(prev => {
          const newState = {
            ...prev,
            [url]: data.html || null
          };
          console.log('New twitterEmbeds state:', newState);
          return newState;
        });
      } else {
        const errorText = await response.text();
        console.error('Twitter embed API error:', response.status, response.statusText, errorText);
        setTwitterEmbeds(prev => ({
          ...prev,
          [url]: null
        }));
      }
    } catch (error) {
      console.error('Error loading Twitter embed:', error);
      setTwitterEmbeds(prev => ({
        ...prev,
        [url]: null
      }));
    }
  }, []); // Remove twitterEmbeds dependency to prevent infinite loop

  // Effect to load Twitter embeds when HTML changes
  useEffect(() => {
    if (!html) return;

    // Create a temporary DOM element to parse HTML and find Twitter placeholders
    const tempDiv = document.createElement('div');
    tempDiv.innerHTML = html;
    const twitterPlaceholders = tempDiv.querySelectorAll('.twitter-placeholder[data-url]');
    
    twitterPlaceholders.forEach(placeholder => {
      const url = placeholder.getAttribute('data-url');
      if (url && twitterEmbeds[url] === undefined) {
        loadTwitterEmbed(url);
      }
    });
  }, [html, loadTwitterEmbed, twitterEmbeds]);

  // Effect to update Twitter embeds in DOM using idiomorph when embed data changes
  useEffect(() => {
    console.log('=== Morphing effect triggered ===');
    console.log('previewRef.current:', previewRef.current);
    console.log('twitterEmbeds state:', twitterEmbeds);
    
    if (!previewRef.current) return;

    // Find Twitter placeholders in the current DOM - convert to array to avoid issues with DOM modification
    const placeholders = Array.from(previewRef.current.querySelectorAll('.twitter-placeholder[data-url]'));
    console.log('Found placeholders:', placeholders.length);
    
    placeholders.forEach((placeholder, index) => {
      const url = placeholder.getAttribute('data-url');
      const embedHtml = twitterEmbeds[url];
      console.log(`Placeholder ${index}: URL=${url}, embedHtml=${embedHtml ? 'has data' : embedHtml}`);
      
      if (embedHtml && embedHtml !== 'loading') {
        // Use idiomorph to replace the placeholder with the actual embed
        if (window.Idiomorph) {
          console.log('Morphing Twitter embed for:', url);
          console.log('Idiomorph available:', typeof window.Idiomorph);
          console.log('Placeholder element:', placeholder);
          console.log('Embed HTML:', embedHtml);
          
          try {
            // Idiomorph.morph expects (fromNode, toHtml)
            window.Idiomorph.morph(placeholder, embedHtml);
          } catch (error) {
            console.error('Idiomorph.morph failed:', error);
            // Fallback to direct replacement
            placeholder.outerHTML = embedHtml;
          }
        } else {
          console.log('Idiomorph not available, using fallback');
          placeholder.outerHTML = embedHtml;
        }
      } else if (twitterEmbeds[url] === null) {
        // Replace with error message using idiomorph
        const errorHtml = `<div style="padding: 10px; border: 1px solid #ccc; border-radius: 4px; background-color: #f9f9f9;"><p>Failed to load Twitter embed</p><a href="${url}">${url}</a></div>`;
        if (window.Idiomorph) {
          console.log('Morphing Twitter error for:', url);
          window.Idiomorph.morph(placeholder, errorHtml);
        } else {
          placeholder.outerHTML = errorHtml;
        }
      } else if (twitterEmbeds[url] === 'loading') {
        // Do nothing
      }
    });
  }, [twitterEmbeds]);

  // Transform function to rewrite links to use file API
  const transformOptions = {
    replace: (domNode) => {
      if (domNode.type === 'tag' && domNode.name === 'a' && domNode.attribs && domNode.attribs.href) {
        const href = domNode.attribs.href;
        
        // Check if this is a relative link to a local file (not starting with http/https/mailto/#)
        if (!href.startsWith('http') && !href.startsWith('mailto:') && !href.startsWith('#') && !href.startsWith('/api/')) {
          // Rewrite the href to use the file API
          const newHref = `/api/files/${href}`;
          
          return (
            <a {...domNode.attribs} href={newHref}>
              {domNode.children && domNode.children.map((child, index) => {
                if (child.type === 'text') {
                  return child.data;
                }
                return parse(child, { key: index });
              })}
            </a>
          );
        }
      }
      
      // Also handle img tags for completeness
      if (domNode.type === 'tag' && domNode.name === 'img' && domNode.attribs && domNode.attribs.src) {
        const src = domNode.attribs.src;
        
        // Check if this is a relative link to a local file
        if (!src.startsWith('http') && !src.startsWith('data:') && !src.startsWith('/api/')) {
          const newSrc = `/api/files/${src}`;
          
          return (
            <img className={styles.preview_img} {...domNode.attribs} src={newSrc} />
          );
        }
      }
    }
  };

  return (
    <div id="preview-content" ref={previewRef}>
      {html ? (
        parse(html, transformOptions)
      ) : (
        <div 
          className="empty-state" 
          style={{ 
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
            color: '#888',
            fontStyle: 'italic'
          }}
        >
          Select a file to preview
        </div>
      )}
    </div>
  );
}
