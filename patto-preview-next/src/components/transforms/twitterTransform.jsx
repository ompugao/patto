/**
 * Transform handler for Twitter embeds (twitter-placeholder divs)
 */
import Tweet, { extractTwitterId } from '../Tweet.jsx';

/**
 * Check if a DOM node is a Twitter placeholder
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isTwitterPlaceholder(domNode) {
    return (
        domNode.type === 'tag' &&
        domNode.name === 'div' &&
        domNode.attribs?.class === 'twitter-placeholder'
    );
}

/**
 * Transform a Twitter placeholder into a Tweet component
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {JSX.Element|null}
 */
export function transformTwitterPlaceholder(domNode) {
    const url = domNode.attribs['data-url'];
    const id = extractTwitterId(url);

    if (id !== undefined && id !== null) {
        return <Tweet key={`tweet-${id}`} id={id} />;
    }

    return null;
}
