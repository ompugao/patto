/**
 * Transform handler for SpeakerDeck embeds (speakerdeck-placeholder divs)
 */
import SpeakerDeck, { extractSpeakerDeckId } from '../SpeakerDeck.jsx';

/**
 * Check if a DOM node is a SpeakerDeck placeholder
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isSpeakerDeckPlaceholder(domNode) {
    return (
        domNode.type === 'tag' &&
        domNode.name === 'div' &&
        domNode.attribs?.class === 'speakerdeck-placeholder'
    );
}

/**
 * Transform a SpeakerDeck placeholder into a SpeakerDeck component
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {JSX.Element|null}
 */
export function transformSpeakerDeckPlaceholder(domNode) {
    const url = domNode.attribs['data-url'];
    const id = domNode.attribs['data-id']; // Support for direct hash ID
    const speakerDeckUrl = extractSpeakerDeckId(url);

    if (id) {
        // Use hash ID if provided
        return <SpeakerDeck key={`speakerdeck-${id}`} id={id} />;
    } else if (speakerDeckUrl) {
        // Fallback to URL-based embedding
        return <SpeakerDeck key={`speakerdeck-${speakerDeckUrl}`} url={speakerDeckUrl} />;
    }

    return null;
}
