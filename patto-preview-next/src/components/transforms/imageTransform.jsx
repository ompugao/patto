/**
 * Transform handlers for images
 */
import styles from '../Preview.module.css';

/**
 * Check if a DOM node is an image that needs path rewriting
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isLocalImage(domNode) {
    if (
        domNode.type !== 'tag' ||
        domNode.name !== 'img' ||
        !domNode.attribs?.src
    ) {
        return false;
    }

    const src = domNode.attribs.src;
    return !src.startsWith('http') && !src.startsWith('data:') && !src.startsWith('/api/');
}

/**
 * Check if a DOM node is an image (any type)
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isImage(domNode) {
    return domNode.type === 'tag' && domNode.name === 'img' && domNode.attribs?.src;
}

/**
 * Transform an image element to use API path if local
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {JSX.Element}
 */
export function transformImage(domNode) {
    const attribs = { ...domNode.attribs };
    delete attribs.class;

    let src = attribs.src;
    if (!src.startsWith('http') && !src.startsWith('data:') && !src.startsWith('/api/')) {
        src = `/api/files/${src}`;
    }

    return <img className={styles.PreviewImage} {...attribs} src={src} />;
}
