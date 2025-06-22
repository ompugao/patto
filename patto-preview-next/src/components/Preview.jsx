import parse, { domToReact } from 'html-react-parser';
import { useEffect, useCallback, useState, createElement } from 'react';
import styles from './Preview.module.css';
import Link from 'next/link';
import { useClientRouter } from '../lib/router';
import { useRouter } from 'next/navigation';
//import Image from 'next/image';
import Tweet, {extractTwitterId} from './Tweet.jsx';
import {MermaidDiagram} from "@lightenna/react-mermaid-diagram";
import LazyCode from './LazyCode.jsx';
import TwoHopLinks from './TwoHopLinks.jsx';
import 'highlight.js/styles/github.min.css';
import { MathJaxContext, MathJax } from 'better-react-mathjax';

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

export default function Preview({ html, anchor, onSelectFile, currentNote }) {
  const router = useRouter();

  // Helper function to get stable React key from DOM node
  const getStableKey = (domNode, fallbackKey) => {
    // Use data-line-id if available for stable keys
    if (domNode.attribs && domNode.attribs['data-line-id']) {
      return domNode.attribs['data-line-id'];
    }
    // Fallback to content hash or position for non-line elements
    return fallbackKey;
  };

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
        if (id !== undefined && id !== null) {
          return <Tweet key={`tweet-${id}`} id={id}/>
        } else {
          return domNode;
        }
      }

      // Handle elements with stable keys from data-line-id
      // if (domNode.type === 'tag' && domNode.attribs?.['data-line-id']) {
      //   const stableKey = getStableKey(domNode, null);
      //   delete domNode.attribs.class;
      //   return createElement(
      //     domNode.name,
      //     { key: stableKey, ...domNode.attribs },
      //     domToReact(domNode.children, transformOptions)
      //   );
      // }

      if (domNode.type === 'tag' && domNode.name === 'a' && domNode.attribs && domNode.attribs.class == "patto-selflink" && domNode.attribs.href) {
		// nothing required
		//return domNode;
        domNode.attribs.className = domNode.attribs.class;
        delete domNode.attribs.class;
        return (
          <Link className={styles.PattoWikiLink} {...domNode.attribs}>
            {domNode.children && domNode.children.map((child, index) => {
              if (child.type === 'text') {
                return child.data;
              }
              return parse(child, { key: getStableKey(child, `child-${index}`) });
            })}
          </Link>
        );
	  } else if (domNode.type === 'tag' && domNode.name === 'a' && domNode.attribs && domNode.attribs.class == "patto-wikilink" && domNode.attribs.href) {
        const url_split = domNode.attribs.href.split('#');
        const notename = url_split[0];
        const anchor = (url_split.length > 1) ? url_split[1] : null;
        const newHref = `/?note=${notename}`;
        domNode.attribs.className = domNode.attribs.class;
        delete domNode.attribs.class;
        delete domNode.attribs.href;
        // setting href reloads the whole page somehow. use onSelectFile instead for loading the preview content via websocket
        return (
          <Link className={styles.PattoWikiLink} {...domNode.attribs} href="#" onClick={evt => {
            evt.preventDefault();
            onSelectFile(notename, anchor);
            }} >
            {domNode.children && domNode.children.map((child, index) => {
              if (child.type === 'text') {
                return child.data;
              }
              return parse(child, { key: getStableKey(child, `child-${index}`) });
            })}
          </Link>
        );
      } else if (domNode.type === 'tag' && domNode.name === 'a' && domNode.attribs && domNode.attribs.href) {
        const href = domNode.attribs.href;
        
        // Check if this is a relative link to a local file (not starting with http/https/mailto/#)
        if (!href.startsWith('http') && !href.startsWith('zotero:') && !href.startsWith('mailto:') && !href.startsWith('#') && !href.startsWith('/api/')) {
          // Rewrite the href to use the file API
          const newHref = `/api/files/${href}`;
          
          return (
            <a {...domNode.attribs} href={newHref}>
              {domNode.children && domNode.children.map((child, index) => {
                if (child.type === 'text') {
                  return child.data;
                }
                return parse(child, { key: getStableKey(child, `child-${index}`) });
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

          //domNode.attribs.className = domNode.attribs.class;
          delete domNode.attribs.class;
          return (
            <img className={styles.PreviewImage} {...domNode.attribs} src={newSrc} />
          );
        } else {
          delete domNode.attribs.class;
          return <img className={styles.PreviewImage} {...domNode.attribs} />;
        }
      }
      // Handle patto-line elements with stable keys
      if (domNode.type === 'tag' && domNode.name === 'li' && domNode.attribs && domNode.attribs.class === 'patto-line') {
        const stableKey = getStableKey(domNode, null);
        delete domNode.attribs.class;
        delete domNode.attribs.style;
        return <li key={stableKey} className={styles.PattoLine} {...domNode.attribs} >{domToReact(domNode.children, transformOptions)}</li>;
      }
      
      // Handle patto-item elements with stable keys
      if (domNode.type === 'tag' && domNode.name === 'li' && domNode.attribs && domNode.attribs.class === 'patto-item') {
        const stableKey = getStableKey(domNode, null);
        delete domNode.attribs.class;
        delete domNode.attribs.style;
        return <li key={stableKey} className={styles.PattoItem} {...domNode.attribs} >{domToReact(domNode.children, transformOptions)}</li>;
      }
      // Handle mermaid diagrams
      if (domNode.type === 'tag' && domNode.name === 'pre') {
        // Check if this is a mermaid code block
        if (domNode.attribs && domNode.attribs.class && domNode.attribs.class.includes('mermaid')) {
          const mermaidCode = domNode.children && domNode.children[0] && domNode.children[0].data;
          if (mermaidCode) {
            return <MermaidDiagram>{mermaidCode}</MermaidDiagram>;
          }
        }
      }
      // Handle tables with Pure CSS classes
      if (domNode.type === 'tag' && domNode.name === 'table') {
        delete domNode.attribs.class;
        delete domNode.attribs.style;
        return (
          <table className="pure-table pure-table-striped" {...domNode.attribs}>
            {domToReact(domNode.children, transformOptions)}
          </table>
        );
      }

      // Handle buttons with Pure CSS classes
      if (domNode.type === 'tag' && domNode.name === 'button') {
        delete domNode.attribs.class;
        delete domNode.attribs.style;
        return (
          <button className="pure-button" {...domNode.attribs}>
            {domToReact(domNode.children, transformOptions)}
          </button>
        );
      }

      // Handle forms with Pure CSS classes
      if (domNode.type === 'tag' && domNode.name === 'form') {
        delete domNode.attribs.class;
        delete domNode.attribs.style;
        return (
          <form className="pure-form" {...domNode.attribs}>
            {domToReact(domNode.children, transformOptions)}
          </form>
        );
      }

      // Handle task elements (checkboxes) first
      if (domNode.type === 'tag' && domNode.name === 'input' && domNode.attribs?.type === 'checkbox') {
        delete domNode.attribs.class;
        delete domNode.attribs.style;
        let defaultChecked = false;
        if (domNode.attribs.checked === '') {
            defaultChecked = true;
        }
        if (domNode.attribs.checked === '') {
            delete domNode.attribs.checked;
        }
        if (domNode.attribs.unchecked === '') {
            delete domNode.attribs.unchecked;
        }
        return (
          <input className="pure-checkbox" defaultChecked={defaultChecked} {...domNode.attribs} />
        );
      }

      // Handle other input elements with Pure CSS classes
      if (domNode.type === 'tag' && domNode.name === 'input') {
        delete domNode.attribs.class;
        delete domNode.attribs.style;
        return (
          <input className="pure-input" {...domNode.attribs} />
        );
      }

      // Handle blockquotes with enhanced styling
      if (domNode.type === 'tag' && domNode.name === 'blockquote') {
        delete domNode.attribs.class;
        delete domNode.attribs.style;
        return (
          <blockquote {...domNode.attribs}>
            {domToReact(domNode.children, transformOptions)}
          </blockquote>
        );
      }

      // Handle anchor spans with stable keys
      if (domNode.type === 'tag' && domNode.name === 'span' && domNode.attribs?.class === 'anchor') {
        const stableKey = getStableKey(domNode, `anchor-${domNode.attribs.id}`);
        delete domNode.attribs.class;
        delete domNode.attribs.style;
        return (
          <span key={stableKey} {...domNode.attribs}>
            {domToReact(domNode.children, transformOptions)}
          </span>
        );
      }

      // Handle task deadline marks
      if (domNode.type === 'tag' && domNode.name === 'mark' && domNode.attribs?.class === 'task-deadline') {
        delete domNode.attribs.class;
        return (
          <mark className="task-deadline" {...domNode.attribs}>
            {domToReact(domNode.children, transformOptions)}
          </mark>
        );
      }

      // Handle aside elements (task metadata)
      if (domNode.type === 'tag' && domNode.name === 'aside') {
        delete domNode.attribs.class;
        delete domNode.attribs.style;
        return (
          <aside {...domNode.attribs}>
            {domToReact(domNode.children, transformOptions)}
          </aside>
        );
      }
      
      if (domNode.type === 'tag' && domNode.name === 'code') {
        const codeText = domNode.children[0]?.data || '';
        const language = domNode.attribs?.class?.replace('language-', '') || '';

        return <LazyCode code={codeText} language={language} />;
      }
    }
  };

  return (
    <MathJaxContext config={mathJaxConfig}>
      <div id="preview-content" className={styles.PreviewContent}>
        {html ? (
          <>
            <MathJax dynamic>
              {parse(html, transformOptions)}
            </MathJax>
            <TwoHopLinks currentNote={currentNote} onSelectFile={onSelectFile} />
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
