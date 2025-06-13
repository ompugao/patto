import parse from 'html-react-parser';
import { useEffect, useCallback } from 'react';

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

  return (
    <div id="preview-content">
      {html ? (
        parse(html)
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
