/**
 * Transform handlers for patto-specific elements (lines, items, anchors, task elements)
 */
import { domToReact } from 'html-react-parser';
import styles from '../Preview.module.css';

/**
 * Get stable ID attribute from DOM node
 * @param {Object} domNode - DOM node from html-react-parser
 * @param {string} fallbackKey - Fallback key if no stable ID
 * @returns {string|null}
 */
export function getStableKey(domNode, fallbackKey) {
    if (domNode.attribs?.['data-line-id']) {
        return domNode.attribs['data-line-id'];
    }
    return fallbackKey;
}

/**
 * Check if a DOM node is a patto-line list item
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isPattoLine(domNode) {
    return (
        domNode.type === 'tag' &&
        domNode.name === 'li' &&
        domNode.attribs?.class === 'patto-line'
    );
}

/**
 * Check if a DOM node is a patto-item list item
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isPattoItem(domNode) {
    return (
        domNode.type === 'tag' &&
        domNode.name === 'li' &&
        domNode.attribs?.class === 'patto-item'
    );
}

/**
 * Check if a DOM node is an anchor span
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isAnchorSpan(domNode) {
    return (
        domNode.type === 'tag' &&
        domNode.name === 'span' &&
        domNode.attribs?.class === 'anchor'
    );
}

/**
 * Check if a DOM node is a task deadline mark
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isTaskDeadline(domNode) {
    return (
        domNode.type === 'tag' &&
        domNode.name === 'mark' &&
        domNode.attribs?.class === 'task-deadline'
    );
}

/**
 * Check if a DOM node is an aside element (task metadata)
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isAside(domNode) {
    return domNode.type === 'tag' && domNode.name === 'aside';
}

/**
 * Transform a patto-line list item
 * @param {Object} domNode - DOM node from html-react-parser
 * @param {Object} transformOptions - Parser transform options for children
 * @returns {JSX.Element}
 */
export function transformPattoLine(domNode, transformOptions) {
    const stableKey = getStableKey(domNode, null);
    const attribs = { ...domNode.attribs };
    delete attribs.class;
    delete attribs.style;

    return (
        <li key={stableKey} className={styles.PattoLine} {...attribs}>
            {domToReact(domNode.children, transformOptions)}
        </li>
    );
}

/**
 * Transform a patto-item list item
 * @param {Object} domNode - DOM node from html-react-parser
 * @param {Object} transformOptions - Parser transform options for children
 * @returns {JSX.Element}
 */
export function transformPattoItem(domNode, transformOptions) {
    const stableKey = getStableKey(domNode, null);
    const attribs = { ...domNode.attribs };
    delete attribs.class;
    delete attribs.style;

    return (
        <li key={stableKey} className={styles.PattoItem} {...attribs}>
            {domToReact(domNode.children, transformOptions)}
        </li>
    );
}

/**
 * Transform an anchor span with stable key
 * @param {Object} domNode - DOM node from html-react-parser
 * @param {Object} transformOptions - Parser transform options for children
 * @returns {JSX.Element}
 */
export function transformAnchorSpan(domNode, transformOptions) {
    const stableKey = getStableKey(domNode, `anchor-${domNode.attribs.id}`);
    const attribs = { ...domNode.attribs };
    delete attribs.class;
    delete attribs.style;

    return (
        <span key={stableKey} {...attribs}>
            {domToReact(domNode.children, transformOptions)}
        </span>
    );
}

/**
 * Transform a task deadline mark
 * @param {Object} domNode - DOM node from html-react-parser
 * @param {Object} transformOptions - Parser transform options for children
 * @returns {JSX.Element}
 */
export function transformTaskDeadline(domNode, transformOptions) {
    const attribs = { ...domNode.attribs };
    delete attribs.class;

    return (
        <mark className="task-deadline" {...attribs}>
            {domToReact(domNode.children, transformOptions)}
        </mark>
    );
}

/**
 * Transform an aside element (task metadata)
 * @param {Object} domNode - DOM node from html-react-parser
 * @param {Object} transformOptions - Parser transform options for children
 * @returns {JSX.Element}
 */
export function transformAside(domNode, transformOptions) {
    const attribs = { ...domNode.attribs };
    delete attribs.class;
    delete attribs.style;

    return (
        <aside {...attribs}>
            {domToReact(domNode.children, transformOptions)}
        </aside>
    );
}
