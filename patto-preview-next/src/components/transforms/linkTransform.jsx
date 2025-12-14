/**
 * Transform handlers for links (wiki-links, self-links, external links)
 */
import parse from 'html-react-parser';
import Link from 'next/link';
import styles from '../Preview.module.css';

/**
 * Check if a DOM node is a patto self-link
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isSelfLink(domNode) {
    return (
        domNode.type === 'tag' &&
        domNode.name === 'a' &&
        domNode.attribs?.class === 'patto-selflink' &&
        domNode.attribs?.href
    );
}

/**
 * Check if a DOM node is a patto wiki-link
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isWikiLink(domNode) {
    return (
        domNode.type === 'tag' &&
        domNode.name === 'a' &&
        domNode.attribs?.class === 'patto-wikilink' &&
        domNode.attribs?.href
    );
}

/**
 * Check if a DOM node is a local file link (needs rewriting to API)
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isLocalFileLink(domNode) {
    if (
        domNode.type !== 'tag' ||
        domNode.name !== 'a' ||
        !domNode.attribs?.href
    ) {
        return false;
    }

    const href = domNode.attribs.href;
    return (
        !href.startsWith('http') &&
        !href.startsWith('zotero:') &&
        !href.startsWith('mailto:') &&
        !href.startsWith('#') &&
        !href.startsWith('/api/')
    );
}

/**
 * Render children of a link node
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {Array}
 */
function renderLinkChildren(domNode) {
    return domNode.children?.map((child, index) => {
        if (child.type === 'text') {
            return child.data;
        }
        return parse(child);
    }) || [];
}

/**
 * Transform a self-link into a Next.js Link component
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {JSX.Element}
 */
export function transformSelfLink(domNode) {
    const attribs = { ...domNode.attribs };
    attribs.className = attribs.class;
    delete attribs.class;

    return (
        <Link className={styles.PattoWikiLink} {...attribs}>
            {renderLinkChildren(domNode)}
        </Link>
    );
}

/**
 * Transform a wiki-link into a client-side navigating Link component
 * @param {Object} domNode - DOM node from html-react-parser
 * @param {Function} onSelectFile - Callback to navigate to a file
 * @returns {JSX.Element}
 */
export function transformWikiLink(domNode, onSelectFile) {
    const urlSplit = domNode.attribs.href.split('#');
    const notename = urlSplit[0];
    const anchor = urlSplit.length > 1 ? urlSplit[1] : null;

    const attribs = { ...domNode.attribs };
    attribs.className = attribs.class;
    delete attribs.class;
    delete attribs.href;

    return (
        <Link
            className={styles.PattoWikiLink}
            {...attribs}
            href="#"
            onClick={(evt) => {
                evt.preventDefault();
                onSelectFile(notename, anchor);
            }}
        >
            {renderLinkChildren(domNode)}
        </Link>
    );
}

/**
 * Transform a local file link to use the API endpoint
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {JSX.Element}
 */
export function transformLocalFileLink(domNode) {
    const href = domNode.attribs.href;
    const newHref = `/api/files/${href}`;

    return (
        <a {...domNode.attribs} href={newHref}>
            {renderLinkChildren(domNode)}
        </a>
    );
}
