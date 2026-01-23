'use client';

/**
 * AstRenderer - Direct AST to React rendering component.
 * Replaces html-react-parser for improved performance (~4x faster).
 */
import { memo, useMemo } from 'react';
import Link from 'next/link';
import { MermaidDiagram } from '@lightenna/react-mermaid-diagram';
import LazyCode from './LazyCode.jsx';
import Tweet from './Tweet.jsx';
import SpeakerDeck from './SpeakerDeck.jsx';
import styles from './Preview.module.css';

/**
 * Render a single AST node
 */
const AstNodeRenderer = memo(function AstNodeRenderer({ node, onSelectFile, keyPrefix = '' }) {
  if (!node || typeof node !== 'object') {
    return null;
  }

  const { kind, text, contents, children, stableId, location } = node;
  const nodeKey = stableId ? `node-${stableId}` : `${keyPrefix}-${Math.random().toString(36).substr(2, 9)}`;

  // Render contents recursively
  const renderContents = (contentsList, prefix = '') => {
    if (!contentsList || contentsList.length === 0) return null;
    return contentsList.map((child, idx) => (
      <AstNodeRenderer
        key={`${prefix}content-${idx}`}
        node={child}
        onSelectFile={onSelectFile}
        keyPrefix={`${prefix}content-${idx}`}
      />
    ));
  };

  // Render children recursively (nested list items)
  const renderChildren = (childrenList, prefix = '') => {
    if (!childrenList || childrenList.length === 0) return null;
    return (
      <ul className={styles.PattoList}>
        {childrenList.map((child, idx) => (
          <AstNodeRenderer
            key={`${prefix}child-${idx}`}
            node={child}
            onSelectFile={onSelectFile}
            keyPrefix={`${prefix}child-${idx}`}
          />
        ))}
      </ul>
    );
  };

  // Handle different node kinds
  if (typeof kind === 'string') {
    // Simple kinds like Text, Dummy, HorizontalLine
    switch (kind) {
      case 'Text':
        return <>{text || ''}</>;

      case 'Dummy':
        // Root node - render contents and children
        return (
          <>
            {contents && contents.length > 0 && (
              <ul className={styles.PattoList}>
                {renderContents(contents, nodeKey)}
              </ul>
            )}
            {renderChildren(children, nodeKey)}
          </>
        );

      case 'HorizontalLine':
        return <hr key={nodeKey} />;

      case 'CodeContent':
      case 'MathContent':
        return <>{text || ''}</>;

      case 'Quote':
        return (
          <blockquote key={nodeKey}>
            {contents && contents.length > 0 && renderContents(contents, nodeKey)}
            {children && children.length > 0 && (
              <ul className={styles.PattoList}>
                {children.map((child, idx) => (
                  <AstNodeRenderer
                    key={`${nodeKey}-qchild-${idx}`}
                    node={child}
                    onSelectFile={onSelectFile}
                    keyPrefix={`${nodeKey}-qchild-${idx}`}
                  />
                ))}
              </ul>
            )}
          </blockquote>
        );

      case 'TableRow':
        return (
          <tr key={nodeKey}>
            {contents?.map((col, idx) => (
              <AstNodeRenderer
                key={`${nodeKey}-col-${idx}`}
                node={col}
                onSelectFile={onSelectFile}
                keyPrefix={`${nodeKey}-col-${idx}`}
              />
            ))}
          </tr>
        );

      case 'TableColumn':
        return (
          <td key={nodeKey}>
            {renderContents(contents, nodeKey)}
          </td>
        );

      default:
        console.warn('Unknown simple AST node kind:', kind);
        return null;
    }
  }

  // Complex kinds (objects with type and fields)
  if (typeof kind === 'object' && kind !== null) {
    const kindType = Object.keys(kind)[0];
    const kindData = kind[kindType];

    switch (kindType) {
      case 'Line': {
        const { properties = [] } = kindData || {};
        const lineId = stableId?.toString();
        const anchor = properties.find(p => p.Anchor)?.Anchor;
        const task = properties.find(p => p.Task)?.Task;

        return (
          <li
            key={nodeKey}
            className={styles.PattoLine}
            data-line-id={lineId}
            id={anchor?.name}
          >
            {renderContents(contents, nodeKey)}
            {task && <PropertyRenderer property={{ Task: task }} />}
            {renderChildren(children, nodeKey)}
          </li>
        );
      }

      case 'QuoteContent': {
        const { properties = [] } = kindData || {};
        const anchor = properties.find(p => p.Anchor)?.Anchor;
        return (
          <li
            key={nodeKey}
            className={styles.PattoLine}
            id={anchor?.name}
          >
            {renderContents(contents, nodeKey)}
            {renderChildren(children, nodeKey)}
          </li>
        );
      }

      case 'Code': {
        const { lang, inline } = kindData || {};
        // Code content is in children as CodeContent nodes - join all of them
        const codeLines = children?.map(c => c.text).filter(Boolean) || [];
        const codeContent = codeLines.length > 0 
          ? codeLines.join('\n') 
          : (contents?.[0]?.text || '');

        // Check for mermaid diagrams
        if (lang === 'mermaid' && !inline) {
          return <MermaidDiagram key={nodeKey}>{codeContent}</MermaidDiagram>;
        }

        if (inline) {
          return (
            <code key={nodeKey} className={lang ? `language-${lang}` : undefined}>
              {codeContent}
            </code>
          );
        }

        return (
          <pre key={nodeKey}>
            <LazyCode code={codeContent} language={lang || ''} />
          </pre>
        );
      }

      case 'Math': {
        const { inline } = kindData || {};
        // Math content can be in children (MathContent) or contents (Text nodes)
        const extractText = (nodes) => {
          if (!nodes) return '';
          return nodes.map(n => n.text || extractText(n.contents) || extractText(n.children)).join('');
        };
        // Try children first (for [@math] blocks), then contents (for inline [$ $])
        const mathContent = extractText(children) || extractText(contents);

        if (inline) {
          return <span key={nodeKey}>{'\\(' + mathContent + '\\)'}</span>;
        }
        return <div key={nodeKey}>{'\\[' + mathContent + '\\]'}</div>;
      }

      case 'WikiLink': {
        const { link, anchor: linkAnchor } = kindData || {};
        const displayText = contents?.map(c => c.text).join('') || link || linkAnchor;

        // Self-link: empty link with anchor - creates a link to anchor on same page
        if (!link && linkAnchor) {
          return (
            <a 
              key={nodeKey} 
              className="patto-selflink" 
              href={`#${linkAnchor}`}
            >
              #{linkAnchor}
            </a>
          );
        }

        // Check if this is a special embed link
        if (link?.includes('twitter.com') || link?.includes('x.com')) {
          return <Tweet key={nodeKey} tweetUrl={link} />;
        }
        if (link?.includes('speakerdeck.com')) {
          const match = link.match(/speakerdeck\.com\/([^/]+)\/([^/]+)/);
          if (match) {
            return <SpeakerDeck key={nodeKey} username={match[1]} slideId={match[2]} />;
          }
        }

        return (
          <Link
            key={nodeKey}
            className={styles.PattoWikiLink}
            href="#"
            onClick={(evt) => {
              evt.preventDefault();
              onSelectFile(link, linkAnchor);
            }}
          >
            {displayText}
          </Link>
        );
      }

      case 'Link': {
        const { link, title } = kindData || {};
        const displayText = contents?.map(c => c.text).join('') || title || link;

        // Handle special link types
        if (link?.startsWith('zotero:')) {
          return (
            <a key={nodeKey} href={link} title={title}>
              {displayText}
            </a>
          );
        }
        if (link?.startsWith('mailto:')) {
          return (
            <a key={nodeKey} href={link} title={title}>
              {displayText}
            </a>
          );
        }

        // External links open in new tab
        const isExternal = link?.startsWith('http://') || link?.startsWith('https://');
        const finalHref = isExternal ? link : `/api/files/${link}`;

        return (
          <a
            key={nodeKey}
            href={finalHref}
            title={title}
            target={isExternal ? '_blank' : undefined}
            rel={isExternal ? 'noopener noreferrer' : undefined}
          >
            {displayText}
          </a>
        );
      }

      case 'Image': {
        const { src, alt } = kindData || {};
        const imageSrc = src?.startsWith('http') ? src : `/api/files/${src}`;
        return <img key={nodeKey} src={imageSrc} alt={alt || ''} />;
      }

      case 'Decoration': {
        const { fontsize = 0, italic, underline, deleted } = kindData || {};

        let element = <>{renderContents(contents, nodeKey)}</>;

        if (deleted) {
          element = <del>{element}</del>;
        }
        if (underline) {
          element = <u>{element}</u>;
        }
        if (italic) {
          element = <em>{element}</em>;
        }
        if (fontsize > 0) {
          // Map fontsize to heading levels
          const HeadingTag = fontsize >= 3 ? 'strong' : fontsize >= 2 ? 'strong' : 'span';
          const fontSize = fontsize >= 3 ? '1.5em' : fontsize >= 2 ? '1.25em' : fontsize >= 1 ? '1.1em' : undefined;
          element = <HeadingTag style={fontSize ? { fontSize } : undefined}>{element}</HeadingTag>;
        } else if (fontsize < 0) {
          const fontSize = fontsize <= -2 ? '0.75em' : '0.85em';
          element = <small style={{ fontSize }}>{element}</small>;
        }

        return <span key={nodeKey}>{element}</span>;
      }

      case 'Table': {
        const { caption } = kindData || {};
        return (
          <table key={nodeKey}>
            {caption && <caption>{caption}</caption>}
            <tbody>
              {children?.map((row, idx) => (
                <AstNodeRenderer
                  key={`${nodeKey}-row-${idx}`}
                  node={row}
                  onSelectFile={onSelectFile}
                  keyPrefix={`${nodeKey}-row-${idx}`}
                />
              ))}
            </tbody>
          </table>
        );
      }

      default:
        console.warn('Unknown AST node kind:', kindType, kindData);
        return null;
    }
  }

  return null;
});

/**
 * Render property annotations (tasks, anchors)
 */
const PropertyRenderer = memo(function PropertyRenderer({ property }) {
  if (!property) return null;

  if (property.Task) {
    const { status, due } = property.Task;
    const statusClass = status === 'Done' ? 'done' : status === 'Doing' ? 'doing' : 'todo';

    let dueDisplay = null;
    if (due) {
      if (due.Date) {
        dueDisplay = due.Date;
      } else if (due.DateTime) {
        dueDisplay = due.DateTime;
      } else if (due.Uninterpretable) {
        dueDisplay = due.Uninterpretable;
      }
    }

    return (
      <aside className={`task-metadata ${statusClass}`}>
        <span className={`task-status ${statusClass}`}>{status}</span>
        {dueDisplay && (
          <mark className="task-deadline">{dueDisplay}</mark>
        )}
      </aside>
    );
  }

  // Anchors are handled in the Line rendering
  return null;
});

/**
 * Main AstRenderer component
 */
export default function AstRenderer({ ast, onSelectFile }) {
  if (!ast) {
    return null;
  }

  return (
    <AstNodeRenderer
      node={ast}
      onSelectFile={onSelectFile}
      keyPrefix="root"
    />
  );
}
