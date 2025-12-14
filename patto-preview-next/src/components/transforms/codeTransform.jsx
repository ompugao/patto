/**
 * Transform handlers for code blocks and mermaid diagrams
 */
import { MermaidDiagram } from "@lightenna/react-mermaid-diagram";
import LazyCode from '../LazyCode.jsx';

/**
 * Check if a DOM node is a mermaid code block
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isMermaidBlock(domNode) {
    return (
        domNode.type === 'tag' &&
        domNode.name === 'pre' &&
        domNode.attribs?.class?.includes('mermaid')
    );
}

/**
 * Check if a DOM node is a code element
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isCodeElement(domNode) {
    return domNode.type === 'tag' && domNode.name === 'code';
}

/**
 * Transform a mermaid code block into a MermaidDiagram component
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {JSX.Element|null}
 */
export function transformMermaidBlock(domNode) {
    const mermaidCode = domNode.children?.[0]?.data;
    if (mermaidCode) {
        return <MermaidDiagram>{mermaidCode}</MermaidDiagram>;
    }
    return null;
}

/**
 * Transform a code element into a LazyCode component with syntax highlighting
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {JSX.Element}
 */
export function transformCodeElement(domNode) {
    const codeText = domNode.children[0]?.data || '';
    const language = domNode.attribs?.class?.replace('language-', '') || '';

    return <LazyCode code={codeText} language={language} />;
}
