/**
 * Transform handlers for form elements (tables, buttons, inputs, forms)
 */
import { domToReact } from 'html-react-parser';

/**
 * Check if a DOM node is a table element
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isTable(domNode) {
    return domNode.type === 'tag' && domNode.name === 'table';
}

/**
 * Check if a DOM node is a button element
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isButton(domNode) {
    return domNode.type === 'tag' && domNode.name === 'button';
}

/**
 * Check if a DOM node is a form element
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isForm(domNode) {
    return domNode.type === 'tag' && domNode.name === 'form';
}

/**
 * Check if a DOM node is a checkbox input
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isCheckbox(domNode) {
    return (
        domNode.type === 'tag' &&
        domNode.name === 'input' &&
        domNode.attribs?.type === 'checkbox'
    );
}

/**
 * Check if a DOM node is an input element (non-checkbox)
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isInput(domNode) {
    return (
        domNode.type === 'tag' &&
        domNode.name === 'input' &&
        domNode.attribs?.type !== 'checkbox'
    );
}

/**
 * Check if a DOM node is a blockquote element
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {boolean}
 */
export function isBlockquote(domNode) {
    return domNode.type === 'tag' && domNode.name === 'blockquote';
}

/**
 * Transform a table element with Pure CSS classes
 * @param {Object} domNode - DOM node from html-react-parser
 * @param {Object} transformOptions - Parser transform options for children
 * @returns {JSX.Element}
 */
export function transformTable(domNode, transformOptions) {
    const attribs = { ...domNode.attribs };
    delete attribs.class;
    delete attribs.style;

    return (
        <table className="pure-table pure-table-striped" {...attribs}>
            {domToReact(domNode.children, transformOptions)}
        </table>
    );
}

/**
 * Transform a button element with Pure CSS classes
 * @param {Object} domNode - DOM node from html-react-parser
 * @param {Object} transformOptions - Parser transform options for children
 * @returns {JSX.Element}
 */
export function transformButton(domNode, transformOptions) {
    const attribs = { ...domNode.attribs };
    delete attribs.class;
    delete attribs.style;

    return (
        <button className="pure-button" {...attribs}>
            {domToReact(domNode.children, transformOptions)}
        </button>
    );
}

/**
 * Transform a form element with Pure CSS classes
 * @param {Object} domNode - DOM node from html-react-parser
 * @param {Object} transformOptions - Parser transform options for children
 * @returns {JSX.Element}
 */
export function transformForm(domNode, transformOptions) {
    const attribs = { ...domNode.attribs };
    delete attribs.class;
    delete attribs.style;

    return (
        <form className="pure-form" {...attribs}>
            {domToReact(domNode.children, transformOptions)}
        </form>
    );
}

/**
 * Transform a checkbox input element
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {JSX.Element}
 */
export function transformCheckbox(domNode) {
    const attribs = { ...domNode.attribs };
    delete attribs.class;
    delete attribs.style;

    let defaultChecked = attribs.checked === '';
    if (attribs.checked === '') delete attribs.checked;
    if (attribs.unchecked === '') delete attribs.unchecked;

    return <input className="pure-checkbox" defaultChecked={defaultChecked} {...attribs} />;
}

/**
 * Transform an input element with Pure CSS classes
 * @param {Object} domNode - DOM node from html-react-parser
 * @returns {JSX.Element}
 */
export function transformInput(domNode) {
    const attribs = { ...domNode.attribs };
    delete attribs.class;
    delete attribs.style;

    return <input className="pure-input" {...attribs} />;
}

/**
 * Transform a blockquote element with enhanced styling
 * @param {Object} domNode - DOM node from html-react-parser
 * @param {Object} transformOptions - Parser transform options for children
 * @returns {JSX.Element}
 */
export function transformBlockquote(domNode, transformOptions) {
    const attribs = { ...domNode.attribs };
    delete attribs.class;
    delete attribs.style;

    return (
        <blockquote {...attribs}>
            {domToReact(domNode.children, transformOptions)}
        </blockquote>
    );
}
