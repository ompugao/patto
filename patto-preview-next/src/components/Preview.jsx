import parse from 'html-react-parser';
import { useEffect, useCallback, useState } from 'react';
import styles from './Preview.module.css';
import Script from 'next/script';
import Tweet, {extractTwitterId} from './Tweet.jsx';

export default function Preview({ html, anchor }) {
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

  // Transform function to handle Twitter embeds and rewrite links
  const transformOptions = {
    replace: (domNode) => {
      // Handle Twitter placeholders
      if (domNode.type === 'tag' && domNode.name === 'div' && 
          domNode.attribs && domNode.attribs.class === 'twitter-placeholder') {
        const url = domNode.attribs['data-url'];
        const id = extractTwitterId(url);
        if (id !== undefined) {
          return <Tweet id={id}/>
        } else {
          return domNode;
        }
      }

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
    <div id="preview-content">
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

      <Script
        id="twitter-embed-script"
        src="https://platform.twitter.com/widgets.js"
        strategy="beforeInteractive"
      />
    </div>
  );
}
