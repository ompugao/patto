import parse, { domToReact } from 'html-react-parser';
import { useEffect, useCallback, useState } from 'react';
import styles from './Preview.module.css';
import Script from 'next/script';
import Link from 'next/link';
import { useClientRouter } from '../lib/router';
import { useRouter } from 'next/navigation';
//import Image from 'next/image';
import Tweet, {extractTwitterId} from './Tweet.jsx';
import hljs from 'highlight.js'
import 'highlight.js/styles/github.min.css';

export default function Preview({ html, anchor, onSelectFile }) {
  const router = useRouter();

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

      if (domNode.type === 'tag' && domNode.name === 'a' && domNode.attribs && domNode.attribs.class == "patto-selflink" && domNode.attribs.href) {
		// nothing required
		return domNode;
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
          <Link {...domNode.attribs} href="#" onClick={evt => {
            evt.preventDefault();
            onSelectFile(notename, anchor);
            }} >
            {domNode.children && domNode.children.map((child, index) => {
              if (child.type === 'text') {
                return child.data;
              }
              return parse(child, { key: index });
            })}
          </Link>
        );
      } else if (domNode.type === 'tag' && domNode.name === 'a' && domNode.attribs && domNode.attribs.href) {
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

          //domNode.attribs.className = domNode.attribs.class;
          delete domNode.attribs.class;
          return (
            <img className={styles.PreviewImage} {...domNode.attribs} src={newSrc} />
          );
        }
      }
      if (domNode.type === 'tag' && domNode.name === 'li' && domNode.attribs && domNode.attribs.class === 'patto-item') {
        delete domNode.attribs.class;
        delete domNode.attribs.style;
        return <li className={styles.PattoItem} {...domNode.attribs} >{domToReact(domNode.children, transformOptions)}</li>;
      }
      if (domNode.type === 'tag' && domNode.name === 'code') {
        const result = hljs.highlightAuto(domNode.children[0].data);
        const dom = parse(result.value);
        
        return (
          <code className='hljs'>
            {dom}
          </code>
        )
      }
    }
  };

  return (
    <div id="preview-content" className={styles.PreviewContent}>
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
