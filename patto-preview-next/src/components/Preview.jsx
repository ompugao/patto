import parse from 'html-react-parser';
import { useEffect, useCallback } from 'react';
import styles from './Preview.module.css';
import { useHtmlTransformer, escapeInvalidTags } from '../lib/useHtmlTransformer';
import TwoHopLinks from './TwoHopLinks.jsx';
import BackLinks from './BackLinks.jsx';
import 'highlight.js/styles/github.min.css';
import { MathJaxContext, MathJax } from 'better-react-mathjax';

/**
 * MathJax configuration for LaTeX rendering
 */
const mathJaxConfig = {
  loader: { load: ["[tex]/html"] },
  tex: {
    packages: { "[+]": ["html"] },
    inlineMath: [["\\(", "\\)"]],
    displayMath: [["\\[", "\\]"]],
    processEscapes: true,
    processEnvironments: true
  },
  options: {
    skipHtmlTags: ["script", "noscript", "style", "textarea", "pre", "code", "a"]
  }
};

/**
 * Preview component for rendering patto note content.
 * 
 * @param {Object} props
 * @param {string} props.html - Raw HTML content to render
 * @param {string|null} props.anchor - Anchor ID to scroll to
 * @param {Function} props.onSelectFile - Callback for navigating to a file
 * @param {string|null} props.currentNote - Currently selected note path
 * @param {Array} props.backLinks - Back-link data for the current note
 * @param {Array} props.twoHopLinks - Two-hop link data for the current note
 */
export default function Preview({ html, anchor, onSelectFile, currentNote, backLinks, twoHopLinks }) {
  // Get memoized transform options from hook
  const transformOptions = useHtmlTransformer(onSelectFile);

  /**
   * Enhanced anchor scrolling with retry mechanism
   */
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
    <MathJaxContext config={mathJaxConfig}>
      <div id="preview-content" className={styles.PreviewContent}>
        {html ? (
          <>
            <MathJax dynamic>
              {parse(escapeInvalidTags(html), transformOptions)}
            </MathJax>
            <BackLinks currentNote={currentNote} onSelectFile={onSelectFile} backLinks={backLinks} />
            <TwoHopLinks currentNote={currentNote} onSelectFile={onSelectFile} twoHopLinks={twoHopLinks} />
          </>
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
    </MathJaxContext>
  );
}
